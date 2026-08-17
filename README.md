![Claunote logo](docs/logo.png)

# Claunote

A self-hosted, database-less notes app. Every note is a plain markdown
file on disk (no database, ever) with a Rust backend, a full
GitHub Flavored Markdown toolbar, and an installable PWA frontend.
Built-in MCP server, so Claude can read and edit your notes directly.

Prefer a plain notepad with no MCP, no Claude connection, nothing to
set up beyond a password? [Notespice](https://github.com/lightmorphic/notespice)
is our other notes app, built for exactly that.

> ⚠️ Vibe coded with [Claude](https://claude.ai).

See [CHANGELOG.md](./CHANGELOG.md) for version history.

## Features

- Clean, minimal interface with a full GitHub Flavored Markdown
  toolbar: headings, lists, tables, footnotes, GitHub-style callouts,
  the works (see [GitHub Flavored Markdown support](#github-flavored-markdown-support))
- WYSIWYG editor with a one-click raw-markdown toggle. The converter
  behind it is small and hand-written, not a third-party library loaded
  from a CDN, so there's nothing external to version-mismatch or break
- No database: every note is just a `.md` file you can open,
  edit, or move with any other tool, even while the app is running
- Images, uploads, and file attachments, stored alongside your
  notes and referenced by plain markdown links
- Full-text search, plus instant name-filtering as you type
- Sidebar shows your last 10 *viewed* notes first, not just
  last-edited
- Undo/redo
- One-click export to a dated zip; import that same zip (or a loose
  `.md` file) back in, never overwriting on a title collision
- Installable PWA that works offline for the app shell, with "Add to
  Home Screen" on mobile or desktop
- Dark/light mode, following system preference
- Argon2id password hashing, per-IP login rate limiting, and a
  handful of other deliberate security choices (see
  [Security notes](#security-notes))
- Self-hosted [Manrope](https://github.com/sharanda/manrope) typeface: no font CDN,
  no external font request of any kind
- Built-in MCP server so Claude (Claude Code, Claude Desktop) can list,
  read, search, create, update, and delete your notes directly — ships
  in `docker-compose.yml`, enabled by default (see
  [Claude integration (MCP)](#claude-integration-mcp))

## Storage

Claunote stores notes as individual markdown files in one directory.
No database, no proprietary format: the filename *is* the note title,
and attachments live in a `files/` subfolder right alongside. That's
the entire data model. `ls` the directory, open a note in any text
editor, back it up with `rsync`, or stop using this app entirely;
nothing is ever locked away in a format only Claunote understands.

See [Data model](#data-model) below for the full detail, including how
search, recently-viewed tracking, and attachments all fit into that
same single directory.

## Quick Start

### Running locally without Docker

Requires a reasonably current stable Rust toolchain. Install via
[rustup](https://rustup.rs) if your OS package manager's version is old.

```bash
git clone https://github.com/lightmorphic/claunote.git
cd claunote
cargo build --release
NOTES_PASSWORD=changeMe123 NOTES_DIR=./notes NOTES_DATA_DIR=./appdata ./target/release/claunote
```

Open <http://localhost:8080>. Notes are written to `./notes`, as `.md`
files, with attachments under `./notes/files`. App-only state (the
recently-viewed list) lives separately under `./appdata`.

> Note: service workers require HTTPS (`localhost` is exempt for
> testing), so offline support and "Add to Home Screen" will only work
> once this is served over TLS on a real domain.

### Using Docker

1. Pull from GitHub Container Registry (after the Actions workflow has
   published it at least once, see
   [Automatic image publishing](#automatic-image-publishing))

   ```bash
   docker pull ghcr.io/lightmorphic/claunote:latest
   sudo mkdir -p /opt/media/notes /opt/claunote
   sudo chown -R 1000:1000 /opt/media/notes /opt/claunote
   docker run -p 8080:8080 \
     -e NOTES_PASSWORD=changeMe123 \
     -v /opt/media/notes:/notes \
     -v /opt/claunote:/data \
     ghcr.io/lightmorphic/claunote:latest
   ```

2. Or build locally

   ```bash
   docker build -t claunote .
   sudo mkdir -p /opt/media/notes /opt/claunote
   sudo chown -R 1000:1000 /opt/media/notes /opt/claunote
   docker run -p 8080:8080 \
     -e NOTES_PASSWORD=changeMe123 \
     -v /opt/media/notes:/notes \
     -v /opt/claunote:/data \
     claunote
   ```

3. Docker Compose

   ```yaml
   services:
     claunote:
       image: ghcr.io/lightmorphic/claunote:latest
       container_name: claunote
       restart: unless-stopped
       environment:
         NOTES_USERNAME: "admin"
         NOTES_PASSWORD: "changeMe!"
       volumes:
         - /opt/media/notes:/notes
         - /opt/claunote:/data
       ports:
         - "8080:8080"
   ```

   ```bash
   sudo mkdir -p /opt/media/notes /opt/claunote
   sudo chown -R 1000:1000 /opt/media/notes /opt/claunote
   docker compose up -d
   ```

> **Note:** the container runs as a non-root user (UID/GID 1000), not
> root. After creating the data directory (or if you're upgrading an
> existing install), run the `chown` command above so the container
> can actually write to the mounted folder. Without it, Claunote
> will fail to create or update notes.

Open <http://localhost:8080>. Notes (`*.md`) and attachments (`files/`)
end up under `/opt/media/notes`, and that's the one directory worth
backing up. `/opt/claunote` holds only app-internal state (currently
just the recently-viewed list used for sidebar ordering), not notes.
It's safe to lose, and kept separate on purpose so it's never mixed in
with your actual data. The `docker-compose.yml` itself can live wherever
you keep your other compose stacks; its own location is independent
of where either of these directories is.

Put this behind a TLS-terminating reverse proxy (nginx, Caddy, Traefik,
or Tailscale Serve) for anything beyond local testing. The `Secure`
cookie flag (on by default) requires the browser to have reached it
over HTTPS, and PWA installability requires it too.

After updating the image:

```bash
docker compose pull
docker compose down
docker compose up -d
```

(`docker compose up -d` alone does **not** re-pull a cached `:latest` tag.)

## Automatic image publishing

`.github/workflows/docker-publish.yml` builds and pushes both images —
the app and the MCP server — to `ghcr.io/lightmorphic/claunote` and
`ghcr.io/lightmorphic/claunote-mcp`, on every push to `main`, plus a
weekly scheduled rebuild so OS-level security patches keep landing
even without a code change. It authenticates with a repository secret
named `CLAUNOTE_GHCR_PAT` (a personal access token with the
`write:packages` and `repo` scopes). Make sure both resulting
packages are set to public in the repo's Packages tab if you want to
`docker pull` them without authenticating.

## Claude integration (MCP)

**This needs to be reachable from the open internet to actually work
with Claude.** Claude Code, Claude Desktop, and claude.ai all run
somewhere other than your own network, so they can't reach an MCP
server that's only bound to your home LAN or a Tailscale-only address
— there has to be a real public domain in front of it (see
[Security notes](#security-notes) for how that's hardened). A
Tailscale-only or LAN-only setup works fine for the web app itself,
just not for the MCP connection.

A companion container exposes your notes to Claude over the
[Model Context Protocol](https://modelcontextprotocol.io) (Streamable
HTTP transport). It's a thin client of Claunote's own API: every
request carries its own bearer token straight through to the app
(accepted as an alternative to a session cookie, but only on
note/search/file endpoints — never account or settings changes), and
goes through the same endpoints as the web app, so search indexing,
title sanitization, and collision handling behave exactly as in the
app. It never knows your login password, so changing it from Settings
can never break this server. Tools exposed: `list_notes`, `read_note`,
`search_notes`, `create_note`, `update_note`, `append_to_note`,
`delete_note`.

**Everything is set up from inside the app — the gear icon in the
sidebar opens Settings.** No token to generate or paste into
`docker-compose.yml` by hand: opening Settings for the first time
already has a real bearer token waiting (generated automatically),
plus:

- The exact `claude mcp add` command, pre-filled with your real
  endpoint and token, ready to copy
- **Regenerate token** — rotates it instantly; any client using the
  old one is locked out immediately, no restart needed
- A **Connected / Disconnected** switch — cuts off all MCP access
  instantly (even with a valid token) without touching the token
  itself, so re-enabling later doesn't mean reconnecting every client

This works because the MCP container mounts the app's own data volume
**read-only** and re-reads the token/switch on every request — so a
change in Settings takes effect on the very next request, on both
sides, with nothing to restart. `docker-compose.yml` only wires up
that shared volume; there's no secret to fill in there for a normal
setup.

**Connect Claude** to `https://<your-domain>/mcp` (put port 4200
behind the same reverse proxy/TLS as the app — never expose it
directly):

- **Claude Code:** paste the command straight from Settings, or:
  `claude mcp add --transport http claunote https://<your-domain>/mcp --header "Authorization: Bearer <token>"`
- **claude.ai (Chat) custom connectors cannot send custom headers**, so
  they can't authenticate to this server as configured. Claude Code
  and Claude Desktop (via a local HTTP-proxy shim that injects the
  header) are the supported paths.

| Variable | Required | Default | Purpose |
|---|---|---|---|
| `NOTES_URL` | no | `http://claunote:8080` | Base URL of the Claunote app |
| `MCP_PORT` | no | `4200` | Port the MCP server listens on |
| `MCP_SETTINGS_FILE` | no | `/data/.mcp_settings.json` | Where the token + connect switch live, managed by the app's Settings panel. Normal setups just mount the shared volume here — see `docker-compose.yml`. |
| `MCP_TOKEN` | no | — | Explicit token override, used only if `MCP_SETTINGS_FILE` isn't readable (i.e. the data volume isn't shared). No connect/disconnect switch in this mode — always on. |

## Account settings

Username and password are also managed from the Settings panel (gear
icon in the sidebar) once the app is running — changing either there
takes effect immediately and survives restarts, no env var or
container edit needed. `NOTES_USERNAME`/`NOTES_PASSWORD` only matter
once, to seed the account the very first time the app ever starts
(with nowhere on disk yet to load credentials from); every later
start ignores them in favor of whatever's already saved. Changing a
password signs out every active session, including the one that made
the change — you'll be prompted to log back in.

**Password requirements:** at least 10 characters, with an uppercase
letter, a lowercase letter, a number, and a symbol — enforced both on
the very first boot (`NOTES_PASSWORD`) and on every change made
through Settings. A password already in place from before this
requirement existed keeps working; it only applies going forward.

## Two-factor authentication

Enable it from Settings (gear icon) → **Two-factor authentication**.
Standard TOTP — works with Google Authenticator, 1Password, Authy, or
any other authenticator app:

1. **Enable** shows a QR code (and a manual-entry key, for apps that
   can't scan) — scan it, then enter the 6-digit code it produces to
   confirm the app actually has the right secret before anything is
   turned on.
2. Confirming generates **8 one-time backup codes**, shown exactly
   once. Save them somewhere safe — each works once, in place of a
   TOTP code, if you lose access to your authenticator app. There's no
   other recovery path for a self-hosted single-user app: no support
   desk to call if both the password and the authenticator are gone.
3. Logging in afterward asks for the code as a second step, after the
   password succeeds. Backup codes work in the same field as a TOTP
   code — no separate "use a backup code" mode to find.
4. **Regenerate backup codes** or **Disable** both require re-entering
   the current password, same sensitivity as changing the account
   itself.

## Environment variables

| Variable | Required | Default | Purpose |
|---|---|---|---|
| `NOTES_USERNAME` | no | `admin` | Login username, **first run only** — see [Account settings](#account-settings) |
| `NOTES_PASSWORD` | **yes** | (none) | Login password (min. 8 characters), **first run only**. Hashed with Argon2id; only the hash is ever stored or logged, never the plaintext. |
| `NOTES_DIR` | no | `/notes` | Where `.md` files and their `files/` attachments live. This is the actual vault. |
| `NOTES_DATA_DIR` | no | `/data` | Where app-only state lives (recently-viewed list, account credentials, MCP settings). Not notes, but back it up if you don't want to reconfigure Settings after a rebuild. |
| `NOTES_PORT` | no | `8080` | Port to listen on |
| `NOTES_INSECURE_COOKIES` | no | `false` | Set to `true` **only** for local testing over plain `http://`. Never set this in production; it removes the `Secure` flag from the session cookie. |

## GitHub Flavored Markdown support

Claunote targets full [GitHub Flavored Markdown](https://github.github.com/gfm/),
plus the callout/alert syntax GitHub's own renderer supports on top of
that spec. The toolbar covers all of it:

- Headings (1-6), bold, italic, strikethrough, inline code
- Bullet, numbered, and checkbox (task) lists, with indent/outdent for nesting
- Blockquotes, fenced code blocks, horizontal rules
- Tables
- Links, images (by URL or upload), and generic file attachments
  (inserted as a link, stored under `files/`, see
  [Data model](#data-model))
- Footnotes (`[^1]` / `[^1]: ...`), with a collected footnotes section
  rendered at the bottom of the note
- GitHub-style callouts (`> [!NOTE]`, `> [!TIP]`, `> [!IMPORTANT]`,
  `> [!WARNING]`, `> [!CAUTION]`) rendered with the same colored-box
  treatment GitHub.com uses, not just as plain blockquote text

Every file Claunote writes is plain, spec-compliant markdown. Open it
on GitHub, in another editor, or in a terminal, and it reads correctly
regardless of whether Claunote is involved at all. The editor is a
small hand-written markdown ⇄ HTML converter built specifically for
this app: no external editor library, no CDN dependency, nothing to
version-mismatch or break. It only implements the GFM subset this
toolbar exposes (listed above); anything outside that (raw HTML
embeds, non-standard extensions) round-trips as plain text rather than
being specially rendered.

## Security notes

Found a vulnerability? See [SECURITY.md](./SECURITY.md) for how to
report it. The notes below are about what's already built in, not how
to disclose something new.

- Passwords are hashed with Argon2id; only the hash is ever kept in
  memory or persisted to disk (`.auth.json` in `NOTES_DATA_DIR`), and
  it's never logged. Changing the password via Settings requires the
  current one and invalidates every session, including the one making
  the change.
- Password strength is enforced (min. 10 characters, upper/lower/
  digit/symbol) on first boot and every later change - not just a
  length check.
- Optional TOTP two-factor authentication, with Argon2id-hashed
  one-time backup codes for account recovery. See
  [Two-factor authentication](#two-factor-authentication).
- Sessions are opaque random tokens held server-side. The cookie
  itself carries no information, so nothing meaningful leaks if it's
  ever captured outside of TLS.
- Login attempts are rate-limited per source IP (8 attempts per 15
  minutes) to blunt brute-force attempts.
- Every note title is passed through an allow-list sanitizer before it
  ever touches the filesystem, closing off path traversal
  (`../../etc/passwd`-style requests) at the one place all note I/O
  goes through.
- The container runs as a non-root user, and the base image's OS
  packages are patched at every build (see the CI workflow's weekly
  scheduled rebuild).
- Request bodies are capped at 25MB (covering the 20MB per-file
  attachment cap plus multipart overhead), and zip imports are
  additionally capped on *decompressed* size, 20MB per entry and 200MB
  per archive, so a crafted "zip bomb" can't exhaust the server.
- `Strict-Transport-Security` is sent automatically whenever
  `NOTES_INSECURE_COOKIES` isn't set — the same signal already used for
  the session cookie's `Secure` flag.
- `docker-compose.yml` assumes internet exposure by default: both
  containers run with every Linux capability dropped,
  `no-new-privileges`, a read-only root filesystem, and memory/PID
  limits; both ports bind to `127.0.0.1` only, requiring a reverse
  proxy in front rather than facing the internet directly.
- The MCP server (see [Claude integration (MCP)](#claude-integration-mcp))
  requires a bearer token — the container won't start without one —
  checked in constant time, plus its own per-IP rate limit independent
  of whatever the reverse proxy already does. That token only ever
  grants note/search/file access: the app checks it against a
  separate code path from the session cookie, so it can't be used to
  reach the account or Settings endpoints even if it leaked.

## Export / Import

**Export** (sidebar → "Export all") downloads every note and
attachment as one zip, named `YYYY-MM-DD_notes.zip`: notes at the
root as `.md` files, attachments under `files/`. It's the same shape
either way, so an export is also a valid import.

**Import** (sidebar → "Import") accepts either:
- That same `.zip` shape. Every `.md` file at its root becomes a
  note, everything under `files/` becomes an attachment
- A single loose `.md` file. Its filename (minus the extension)
  becomes the note title

Import never overwrites an existing note. A title that already
exists gets a `(1)`, `(2)`, etc. suffix instead: importing a note
called `note-name` when `note-name.md` already exists produces
`note-name(1).md`; import it again and you get `note-name(2).md`, and
so on. Re-importing an export you already have, in other words, adds a
duplicate copy rather than silently replacing anything. (Imported
attachments use Claunote's regular file-upload collision handling
instead, a `-2`, `-3`, etc. suffix, since that's the same code path
as a normal upload through the editor.)

## Note list order

The sidebar shows your last 10 *viewed* notes first,
most-recently-opened at the top, not last-edited. Opening a note you
don't change still brings it to the top; editing isn't required.
Reopening a note already in that list moves it back to the top rather
than duplicating it. Every other note (anything outside the last 10
viewed) falls back to last-modified order underneath.

This is tracked in a small `.recent.json` file in `NOTES_DATA_DIR`:
plain JSON, an array of up to 10 titles, most-recent-first. It's
disposable app state, not a note, which is exactly why it lives in a
separate directory from the notes themselves rather than mixed in with
your vault: deleting it just resets the sidebar to modified-time order,
nothing else is affected, and it's excluded from search, export, and
the note list itself.

## Data model

Every note is `<NOTES_DIR>/<title>.md`, a plain UTF-8 markdown file.
Anything inserted into a note (images, PDFs, any other attachment)
is uploaded to `<NOTES_DIR>/files/<name>` and referenced from the note
by a relative link, so the whole vault (text and attachments together)
is still just one bind-mounted volume: `NOTES_DIR` is the only
directory you ever need to back up. `NOTES_DATA_DIR` is a separate,
smaller directory for app-only state (see
[Note list order](#note-list-order)), deliberately not part of the
vault, since it isn't your data.

There is no database, no hidden index file that matters (the search
index lives in memory only and rebuilds from these files on every
start), and no proprietary formatting. You can add, edit, or delete
`.md` files, or drop files directly into `files/`, while the app is
stopped. You can even do it while it's running, though you'll need to
restart to pick up out-of-band changes, since the index isn't watching
the filesystem.

Attachment filenames go through the same allow-list sanitizer as note
titles before ever touching disk, and duplicate uploads are
disambiguated with a `-2`, `-3`, and so on suffix rather than
overwriting. Fetching an attachment (`GET /api/files/<name>`) requires
the same session cookie as everything else; nothing is reachable by a
logged-out visitor just because it's rendered as an `<img>` tag rather
than fetched with JavaScript. Uploads are capped at 20MB per file.

## Project Structure

```
claunote/
├── src/                      # Rust backend (axum)
│   ├── main.rs
│   ├── auth.rs               # password hashing, sessions, rate limiting
│   ├── handlers.rs           # HTTP route handlers
│   ├── store.rs              # note/attachment/recent-views file I/O
│   └── search.rs             # in-memory inverted-index search
├── static/                   # Frontend: plain HTML/CSS/JS, no build step
│   ├── index.html
│   ├── app.js
│   ├── style.css
│   ├── manifest.json         # PWA manifest
│   ├── sw.js                 # service worker (app shell only, no note data)
│   ├── icons/
│   ├── fonts/                # self-hosted Manrope (variable weight)
│   └── images/                # Lightmorphic badge logos
├── tests/
│   └── e2e-roundtrip.js      # 58-scenario Writer<->Markdown suite (real browser)
├── mcp/                       # Companion MCP server (Node.js)
│   ├── server.js
│   ├── package.json
│   └── Dockerfile
├── docs/
│   └── logo.png
├── Cargo.toml
├── Dockerfile
├── docker-compose.yml
├── .dockerignore
├── .gitignore
├── CHANGELOG.md
├── SECURITY.md
├── LICENSE
└── .github/workflows/        # Docker image publishing + GitHub releases
```

## Development

Keep it simple. This app exists specifically to be small enough to
read top to bottom: no frontend build step, no database, no dependency
that isn't earning its place. If a change makes something harder to
understand without a clear payoff, it's probably the wrong change.

The Writer/Markdown converter is guarded by `tests/e2e-roundtrip.js`:
58 scenarios covering the full supported GFM feature set in both
directions, plus real toolbar and keyboard interactions, driven
against the actual app in a real browser (Playwright + Chromium; see
the file header for how to run it). Any change touching the editor,
the converter, or Enter/paste handling should keep that suite at
58/58 before shipping.

## License

Claunote is free software, licensed under the MIT License. See the
[LICENSE](./LICENSE) file for the full text.

## Disclaimer

This software is provided "as is", without warranty of any kind,
express or implied. See the LICENSE file for the exact legal terms.
You use it entirely at your own risk.
