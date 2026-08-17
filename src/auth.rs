//! Authentication.
//!
//! Single user, by design - this is a personal notes app, not a
//! multi-tenant product. Security still matters just as much for a
//! single-user app (arguably more, since there's no admin watching for
//! abuse), so:
//!
//! - The password is never stored or compared in plaintext. Only its
//!   Argon2id hash is ever kept, in memory and in `.auth.json` on the
//!   data volume (so a Settings-panel password change survives a
//!   restart). On first run this file doesn't exist yet, so
//!   NOTES_USERNAME/NOTES_PASSWORD seed it once; every later run
//!   ignores those env vars in favor of whatever's on disk.
//! - Sessions are opaque random tokens, held server-side in memory and
//!   handed to the browser as an HttpOnly, Secure, SameSite=Strict
//!   cookie. The cookie value itself carries no information an attacker
//!   could use even if intercepted in a way that skipped TLS - it's just
//!   a lookup key, not a signed blob of claims.
//! - Login attempts are rate-limited per source IP to blunt brute force.

use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::RwLock;
use std::time::{Duration, Instant};

pub const SESSION_COOKIE_NAME: &str = "claunote_session";
const SESSION_LIFETIME: Duration = Duration::from_secs(60 * 60 * 24 * 14); // 14 days
const MAX_LOGIN_ATTEMPTS: u32 = 8;
const LOGIN_ATTEMPT_WINDOW: Duration = Duration::from_secs(15 * 60);

#[derive(Serialize, Deserialize, Clone)]
struct Credentials {
    username: String,
    password_hash: String,
}

pub struct Auth {
    credentials: RwLock<Credentials>,
    // Where credentials.json lives - the Settings panel changes
    // username/password at runtime, unlike the rest of this app's
    // config, so it needs somewhere durable to persist a change
    // instead of only ever reflecting whatever NOTES_USERNAME/
    // NOTES_PASSWORD were at container start.
    credentials_path: PathBuf,
    sessions: RwLock<HashMap<String, Instant>>,
    login_attempts: RwLock<HashMap<String, (u32, Instant)>>,
}

impl Auth {
    /// On first run, hashes NOTES_USERNAME/NOTES_PASSWORD and persists
    /// them to `data_dir/.auth.json` - the plaintext password is
    /// dropped immediately after, only the Argon2 hash is ever kept in
    /// memory or on disk. On every later run, the env vars are ignored
    /// in favor of whatever's in that file, since the Settings panel
    /// may have since changed it - editing docker-compose.yml's
    /// NOTES_PASSWORD after the first run has no effect, by design.
    pub fn load_or_bootstrap(
        data_dir: &std::path::Path,
        env_username: String,
        env_password: &str,
    ) -> anyhow::Result<Self> {
        let credentials_path = data_dir.join(".auth.json");
        let credentials = match std::fs::read_to_string(&credentials_path) {
            Ok(json) => serde_json::from_str(&json)
                .map_err(|e| anyhow::anyhow!("corrupt {}: {e}", credentials_path.display()))?,
            Err(_) => {
                let salt = SaltString::generate(&mut OsRng);
                let password_hash = Argon2::default()
                    .hash_password(env_password.as_bytes(), &salt)
                    .map_err(|e| anyhow::anyhow!("failed to hash configured password: {e}"))?
                    .to_string();
                let creds = Credentials {
                    username: env_username,
                    password_hash,
                };
                let json = serde_json::to_string(&creds)?;
                std::fs::write(&credentials_path, json).map_err(|e| {
                    anyhow::anyhow!("writing {}: {e}", credentials_path.display())
                })?;
                creds
            }
        };

        Ok(Self {
            credentials: RwLock::new(credentials),
            credentials_path,
            sessions: RwLock::new(HashMap::new()),
            login_attempts: RwLock::new(HashMap::new()),
        })
    }

    /// Returns true if `client_key` (e.g. remote IP) is currently locked
    /// out from further login attempts.
    pub fn is_locked_out(&self, client_key: &str) -> bool {
        let attempts = self.login_attempts.read().unwrap();
        if let Some((count, first_attempt)) = attempts.get(client_key) {
            if first_attempt.elapsed() < LOGIN_ATTEMPT_WINDOW && *count >= MAX_LOGIN_ATTEMPTS {
                return true;
            }
        }
        false
    }

    fn record_failed_attempt(&self, client_key: &str) {
        let mut attempts = self.login_attempts.write().unwrap();
        // Bound memory growth: an attacker with access to many source
        // addresses (trivial with IPv6's address space) could otherwise
        // accumulate unbounded entries here, since an entry only gets
        // touched again if that same exact IP comes back. Sweeping on
        // every failed attempt keeps this self-limiting even under
        // sustained attack, at the cost of an O(n) scan proportional to
        // exactly the entries this same growth would otherwise leave
        // behind forever.
        attempts.retain(|_, (_, first_attempt)| first_attempt.elapsed() < LOGIN_ATTEMPT_WINDOW);
        let entry = attempts
            .entry(client_key.to_string())
            .or_insert((0, Instant::now()));
        if entry.1.elapsed() > LOGIN_ATTEMPT_WINDOW {
            *entry = (0, Instant::now());
        }
        entry.0 += 1;
    }

    fn clear_attempts(&self, client_key: &str) {
        self.login_attempts.write().unwrap().remove(client_key);
    }

    /// Verifies credentials and, on success, creates and returns a new
    /// session token. Username comparison and password verification both
    /// run regardless of whether the username matches, so a mistyped
    /// username doesn't respond measurably faster than a wrong password
    /// (a small timing-side-channel precaution).
    pub fn login(&self, client_key: &str, username: &str, password: &str) -> Option<String> {
        if self.is_locked_out(client_key) {
            return None;
        }

        let creds = self.credentials.read().unwrap().clone();
        let username_ok = constant_time_eq(username.as_bytes(), creds.username.as_bytes());
        let password_ok = PasswordHash::new(&creds.password_hash)
            .ok()
            .map(|hash| {
                Argon2::default()
                    .verify_password(password.as_bytes(), &hash)
                    .is_ok()
            })
            .unwrap_or(false);

        if username_ok && password_ok {
            self.clear_attempts(client_key);
            Some(self.create_session())
        } else {
            self.record_failed_attempt(client_key);
            None
        }
    }

    /// Changes username and/or password, verifying `current_password`
    /// first regardless of whether the caller already has a valid
    /// session - a session left open on a shared machine shouldn't be
    /// enough on its own to lock the real owner out. Persists to disk
    /// and invalidates every session (including the caller's) so the
    /// change takes effect everywhere immediately, forcing a fresh
    /// login with the new credentials.
    pub fn change_credentials(
        &self,
        current_password: &str,
        new_username: Option<String>,
        new_password: Option<String>,
    ) -> Result<(), &'static str> {
        let current = self.credentials.read().unwrap().clone();
        let current_ok = PasswordHash::new(&current.password_hash)
            .ok()
            .map(|hash| {
                Argon2::default()
                    .verify_password(current_password.as_bytes(), &hash)
                    .is_ok()
            })
            .unwrap_or(false);
        if !current_ok {
            return Err("current password is incorrect");
        }

        let mut updated = current;
        if let Some(username) = new_username {
            updated.username = username;
        }
        if let Some(password) = new_password {
            let salt = SaltString::generate(&mut OsRng);
            updated.password_hash = Argon2::default()
                .hash_password(password.as_bytes(), &salt)
                .map_err(|_| "failed to hash new password")?
                .to_string();
        }

        let json = serde_json::to_string(&updated).map_err(|_| "failed to serialize credentials")?;
        std::fs::write(&self.credentials_path, json).map_err(|_| "failed to persist credentials")?;

        *self.credentials.write().unwrap() = updated;
        self.sessions.write().unwrap().clear();
        Ok(())
    }

    fn create_session(&self) -> String {
        let mut bytes = [0u8; 32];
        OsRng.fill_bytes(&mut bytes);
        let token = base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, bytes);
        let mut sessions = self.sessions.write().unwrap();
        // Same bounded-growth reasoning as record_failed_attempt: an
        // expired session otherwise sits in memory forever unless
        // validate_session happens to be called again with that exact
        // token. Sweeping here keeps this self-limiting.
        sessions.retain(|_, created| created.elapsed() < SESSION_LIFETIME);
        sessions.insert(token.clone(), Instant::now());
        token
    }

    pub fn validate_session(&self, token: &str) -> bool {
        let mut sessions = self.sessions.write().unwrap();
        match sessions.get(token) {
            Some(created) if created.elapsed() < SESSION_LIFETIME => true,
            Some(_) => {
                sessions.remove(token);
                false
            }
            None => false,
        }
    }

    pub fn logout(&self, token: &str) {
        self.sessions.write().unwrap().remove(token);
    }

    pub fn username(&self) -> String {
        self.credentials.read().unwrap().username.clone()
    }
}

/// Compares two byte strings in constant time, regardless of where they
/// first differ, to avoid leaking length/content via response timing.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        // Still walk a full comparison of equal length so this branch
        // doesn't return meaningfully faster than the equal-length path.
        let filler = vec![0u8; a.len()];
        let mut _diff = 0u8;
        for (x, y) in a.iter().zip(filler.iter()) {
            _diff |= x ^ y;
        }
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

pub fn build_session_cookie(token: &str, secure: bool) -> String {
    let secure_flag = if secure { "; Secure" } else { "" };
    format!(
        "{SESSION_COOKIE_NAME}={token}; HttpOnly; SameSite=Strict; Path=/; Max-Age={}{secure_flag}",
        SESSION_LIFETIME.as_secs()
    )
}

pub fn build_logout_cookie(secure: bool) -> String {
    let secure_flag = if secure { "; Secure" } else { "" };
    format!("{SESSION_COOKIE_NAME}=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0{secure_flag}")
}

pub fn extract_session_token(cookie_header: Option<&str>) -> Option<String> {
    let header = cookie_header?;
    for part in header.split(';') {
        let part = part.trim();
        if let Some(value) = part.strip_prefix(&format!("{SESSION_COOKIE_NAME}=")) {
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}
