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
use totp_rs::{Algorithm, Secret, TOTP};

pub const SESSION_COOKIE_NAME: &str = "claunote_session";
const SESSION_LIFETIME: Duration = Duration::from_secs(60 * 60 * 24 * 14); // 14 days
const MAX_LOGIN_ATTEMPTS: u32 = 8;
const LOGIN_ATTEMPT_WINDOW: Duration = Duration::from_secs(15 * 60);
// Deliberately much shorter than a real session: this token only ever
// exists to carry a user from "password verified" to "TOTP code
// verified" in the same login attempt, never anything longer-lived.
const PENDING_2FA_LIFETIME: Duration = Duration::from_secs(5 * 60);
const BACKUP_CODE_COUNT: usize = 8;
const TOTP_ISSUER: &str = "Claunote";

#[derive(Serialize, Deserialize, Clone, Default)]
struct TwoFactor {
    #[serde(default)]
    enabled: bool,
    // Present as soon as setup starts, even before the user has
    // confirmed a code - `enabled` is what actually gates login, so a
    // setup someone starts and abandons never affects anything.
    #[serde(default)]
    secret: Option<String>,
    // Argon2id hashes, same as the password - never the plaintext
    // codes, which are shown to the user exactly once at
    // generation time. Each is removed the moment it's used, so a
    // captured-and-reused code can never work twice.
    #[serde(default)]
    backup_code_hashes: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
struct Credentials {
    username: String,
    password_hash: String,
    #[serde(default)]
    two_factor: TwoFactor,
}

pub enum LoginOutcome {
    /// Full session token - either 2FA isn't enabled, or (not
    /// applicable here) it already passed.
    Success(String),
    /// Username/password were correct, but a TOTP or backup code is
    /// still needed. Carries a short-lived, single-use token that
    /// proves the password step already passed - not a session, and
    /// not usable for anything but completing this same login.
    NeedsTwoFactor(String),
    Failed,
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
    pending_2fa: RwLock<HashMap<String, Instant>>,
}

impl Auth {
    /// On first run, hashes NOTES_USERNAME/NOTES_PASSWORD and persists
    /// them to `data_dir/.auth.json` - the plaintext password is
    /// dropped immediately after, only the Argon2 hash is ever kept in
    /// memory or on disk. On every later run, the env vars are ignored
    /// in favor of whatever's in that file, since the Settings panel
    /// may have since changed it - editing docker-compose.yml's
    /// NOTES_PASSWORD after the first run has no effect, by design.
    ///
    /// The strength check only runs here, on the bootstrap path - not
    /// unconditionally on every startup - so a NOTES_PASSWORD left in
    /// docker-compose.yml after the first run (where it's inert) can
    /// never block a restart just because it no longer meets the bar.
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
                validate_password_strength(env_password).map_err(|e| anyhow::anyhow!("{e}"))?;
                let salt = SaltString::generate(&mut OsRng);
                let password_hash = Argon2::default()
                    .hash_password(env_password.as_bytes(), &salt)
                    .map_err(|e| anyhow::anyhow!("failed to hash configured password: {e}"))?
                    .to_string();
                let creds = Credentials {
                    username: env_username,
                    password_hash,
                    two_factor: TwoFactor::default(),
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
            pending_2fa: RwLock::new(HashMap::new()),
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

    /// Verifies credentials. On success: a full session if 2FA isn't
    /// enabled, or a short-lived pending-2FA token if it is - the
    /// caller still needs `verify_two_factor_login` to actually get
    /// in. Username comparison and password verification both run
    /// regardless of whether the username matches, so a mistyped
    /// username doesn't respond measurably faster than a wrong
    /// password (a small timing-side-channel precaution).
    pub fn login(&self, client_key: &str, username: &str, password: &str) -> LoginOutcome {
        if self.is_locked_out(client_key) {
            return LoginOutcome::Failed;
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
            if creds.two_factor.enabled {
                LoginOutcome::NeedsTwoFactor(self.create_pending_2fa())
            } else {
                LoginOutcome::Success(self.create_session())
            }
        } else {
            self.record_failed_attempt(client_key);
            LoginOutcome::Failed
        }
    }

    fn create_pending_2fa(&self) -> String {
        let mut bytes = [0u8; 32];
        OsRng.fill_bytes(&mut bytes);
        let token = base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, bytes);
        let mut pending = self.pending_2fa.write().unwrap();
        pending.retain(|_, created| created.elapsed() < PENDING_2FA_LIFETIME);
        pending.insert(token.clone(), Instant::now());
        token
    }

    /// Completes a login that `login()` flagged as needing 2FA. The
    /// pending token is single-use regardless of outcome - removed
    /// here whether the code turns out right or wrong, so a leaked
    /// pending token is only ever worth one guess. Accepts either the
    /// current/adjacent TOTP code or an unused backup code; a backup
    /// code is deleted the instant it's used.
    pub fn verify_two_factor_login(
        &self,
        client_key: &str,
        pending_token: &str,
        code: &str,
    ) -> Option<String> {
        if self.is_locked_out(client_key) {
            return None;
        }
        let still_pending = {
            let mut pending = self.pending_2fa.write().unwrap();
            match pending.remove(pending_token) {
                Some(created) if created.elapsed() < PENDING_2FA_LIFETIME => true,
                _ => false,
            }
        };
        if !still_pending {
            self.record_failed_attempt(client_key);
            return None;
        }

        if self.verify_totp_or_backup_code(code) {
            self.clear_attempts(client_key);
            Some(self.create_session())
        } else {
            self.record_failed_attempt(client_key);
            None
        }
    }

    fn verify_totp_or_backup_code(&self, code: &str) -> bool {
        let creds = self.credentials.read().unwrap().clone();
        if let Some(secret) = &creds.two_factor.secret {
            if let Some(totp) = build_totp(secret, &creds.username) {
                if totp.check_current(code).unwrap_or(false) {
                    return true;
                }
            }
        }

        // Fall back to backup codes. Constant-time-ish in spirit -
        // Argon2 verification already dominates the timing regardless
        // of which entry (if any) matches - and the matched hash is
        // removed so it can never be replayed.
        let matched_index = creds
            .two_factor
            .backup_code_hashes
            .iter()
            .position(|hash| {
                PasswordHash::new(hash)
                    .ok()
                    .map(|h| Argon2::default().verify_password(code.as_bytes(), &h).is_ok())
                    .unwrap_or(false)
            });
        if let Some(index) = matched_index {
            let mut updated = creds;
            updated.two_factor.backup_code_hashes.remove(index);
            if let Ok(json) = serde_json::to_string(&updated) {
                let _ = std::fs::write(&self.credentials_path, json);
            }
            *self.credentials.write().unwrap() = updated;
            true
        } else {
            false
        }
    }

    pub fn two_factor_enabled(&self) -> bool {
        self.credentials.read().unwrap().two_factor.enabled
    }

    /// Starts (or restarts) enrollment: generates a fresh secret and
    /// stores it, but leaves `enabled` untouched. A setup someone
    /// starts and never confirms has no effect on login - only
    /// `confirm_two_factor_setup` flips `enabled`, and it only does
    /// that once the user has proven their authenticator app actually
    /// has the right secret by producing a valid code.
    /// Returns (base32 secret for manual entry, otpauth:// URL, QR
    /// code as a data: URI ready to drop straight into an `<img
    /// src>`).
    pub fn start_two_factor_setup(&self) -> Result<(String, String, String), &'static str> {
        let secret = Secret::generate_secret();
        let secret_b32 = secret.to_encoded().to_string();
        let username = self.credentials.read().unwrap().username.clone();
        let totp = build_totp(&secret_b32, &username).ok_or("failed to build TOTP")?;
        let otpauth_url = totp.get_url();
        let qr_data_uri = totp
            .get_qr_base64()
            .map(|b64| format!("data:image/png;base64,{b64}"))
            .map_err(|_| "failed to generate QR code")?;

        let mut updated = self.credentials.read().unwrap().clone();
        updated.two_factor.secret = Some(secret_b32.clone());
        let json = serde_json::to_string(&updated).map_err(|_| "failed to serialize credentials")?;
        std::fs::write(&self.credentials_path, json).map_err(|_| "failed to persist credentials")?;
        *self.credentials.write().unwrap() = updated;

        Ok((secret_b32, otpauth_url, qr_data_uri))
    }

    /// Confirms enrollment with a code from the authenticator app.
    /// Only this call ever sets `enabled = true` - generates and
    /// returns a fresh set of backup codes (plaintext, shown exactly
    /// once) at the same time, since without them a lost device would
    /// mean permanent lockout with no support desk to call.
    pub fn confirm_two_factor_setup(&self, code: &str) -> Result<Vec<String>, &'static str> {
        let creds = self.credentials.read().unwrap().clone();
        let secret = creds
            .two_factor
            .secret
            .as_ref()
            .ok_or("no 2FA setup in progress - start setup first")?;
        let totp = build_totp(secret, &creds.username).ok_or("failed to build TOTP")?;
        if !totp.check_current(code).unwrap_or(false) {
            return Err("that code didn't match - check your authenticator app's time is correct");
        }

        let (plaintext_codes, hashes) = generate_backup_codes()?;
        let mut updated = creds;
        updated.two_factor.enabled = true;
        updated.two_factor.backup_code_hashes = hashes;
        let json = serde_json::to_string(&updated).map_err(|_| "failed to serialize credentials")?;
        std::fs::write(&self.credentials_path, json).map_err(|_| "failed to persist credentials")?;
        *self.credentials.write().unwrap() = updated;

        Ok(plaintext_codes)
    }

    /// Turns 2FA off entirely - requires the current password, same
    /// sensitivity as changing the account itself. Clears the secret
    /// and every backup code, not just the `enabled` flag, so a later
    /// re-enable always starts from a clean slate.
    pub fn disable_two_factor(&self, current_password: &str) -> Result<(), &'static str> {
        let current = self.credentials.read().unwrap().clone();
        if !self.password_matches(&current, current_password) {
            return Err("current password is incorrect");
        }
        let mut updated = current;
        updated.two_factor = TwoFactor::default();
        let json = serde_json::to_string(&updated).map_err(|_| "failed to serialize credentials")?;
        std::fs::write(&self.credentials_path, json).map_err(|_| "failed to persist credentials")?;
        *self.credentials.write().unwrap() = updated;
        Ok(())
    }

    /// Invalidates every existing backup code and issues a fresh set -
    /// for when the old ones might have been seen by someone else, or
    /// just run out.
    pub fn regenerate_backup_codes(&self, current_password: &str) -> Result<Vec<String>, &'static str> {
        let current = self.credentials.read().unwrap().clone();
        if !self.password_matches(&current, current_password) {
            return Err("current password is incorrect");
        }
        if !current.two_factor.enabled {
            return Err("two-factor authentication isn't enabled");
        }
        let (plaintext_codes, hashes) = generate_backup_codes()?;
        let mut updated = current;
        updated.two_factor.backup_code_hashes = hashes;
        let json = serde_json::to_string(&updated).map_err(|_| "failed to serialize credentials")?;
        std::fs::write(&self.credentials_path, json).map_err(|_| "failed to persist credentials")?;
        *self.credentials.write().unwrap() = updated;
        Ok(plaintext_codes)
    }

    fn password_matches(&self, creds: &Credentials, password: &str) -> bool {
        PasswordHash::new(&creds.password_hash)
            .ok()
            .map(|hash| Argon2::default().verify_password(password.as_bytes(), &hash).is_ok())
            .unwrap_or(false)
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
        if !self.password_matches(&current, current_password) {
            return Err("current password is incorrect");
        }

        let mut updated = current;
        if let Some(username) = new_username {
            updated.username = username;
        }
        if let Some(password) = new_password {
            validate_password_strength(&password)?;
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

/// Builds a TOTP validator from a stored base32 secret - standard
/// 6-digit/30-second/SHA1, the defaults every authenticator app
/// (Google Authenticator, Authy, 1Password, etc.) assumes. `skew: 1`
/// tolerates one step of clock drift either side, since phones and
/// servers are never perfectly in sync.
fn build_totp(secret_b32: &str, username: &str) -> Option<TOTP> {
    let secret_bytes = Secret::Encoded(secret_b32.to_string()).to_bytes().ok()?;
    TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret_bytes,
        Some(TOTP_ISSUER.to_string()),
        username.to_string(),
    )
    .ok()
}

/// Generates a fresh set of one-time backup codes: returns the
/// plaintext (shown to the user exactly once, at generation time) and
/// their Argon2id hashes (the only form ever persisted).
fn generate_backup_codes() -> Result<(Vec<String>, Vec<String>), &'static str> {
    let mut plaintext = Vec::with_capacity(BACKUP_CODE_COUNT);
    let mut hashes = Vec::with_capacity(BACKUP_CODE_COUNT);
    const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789"; // no 0/O/1/I - easy to misread
    for _ in 0..BACKUP_CODE_COUNT {
        let mut raw = [0u8; 8];
        OsRng.fill_bytes(&mut raw);
        let chars: String = raw.iter().map(|b| ALPHABET[(*b as usize) % ALPHABET.len()] as char).collect();
        let code = format!("{}-{}", &chars[0..4], &chars[4..8]);

        let salt = SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
            .hash_password(code.as_bytes(), &salt)
            .map_err(|_| "failed to hash backup code")?
            .to_string();

        plaintext.push(code);
        hashes.push(hash);
    }
    Ok((plaintext, hashes))
}

/// Compares two byte strings in constant time, regardless of where they
/// first differ, to avoid leaking length/content via response timing.
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
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

/// Enforced everywhere a password is ever set - the initial
/// NOTES_PASSWORD bootstrap and every change made through Settings -
/// so the account can never end up on a weak password regardless of
/// which path set it. VPS-appropriate minimum: length plus the usual
/// upper/lower/digit/symbol mixture, not just a character count.
pub fn validate_password_strength(password: &str) -> Result<(), &'static str> {
    if password.chars().count() < 10 {
        return Err("password must be at least 10 characters");
    }
    let has_upper = password.chars().any(|c| c.is_ascii_uppercase());
    let has_lower = password.chars().any(|c| c.is_ascii_lowercase());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    let has_symbol = password.chars().any(|c| c.is_ascii_graphic() && !c.is_ascii_alphanumeric());
    if !has_upper || !has_lower || !has_digit || !has_symbol {
        return Err(
            "password must include at least one uppercase letter, one lowercase letter, \
             one number, and one symbol",
        );
    }
    Ok(())
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
