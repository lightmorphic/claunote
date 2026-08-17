// Claunote MCP server — exposes your notes to Claude (Claude Code,
// claude.ai custom connectors, Claude Desktop) over the Model Context
// Protocol's Streamable HTTP transport.
//
// It is a thin client of the Claunote HTTP API: every call it makes
// carries its own bearer token straight through to the app (the app
// accepts that same token as an alternative to a session cookie, but
// only on note/search/file endpoints — never account or settings
// changes), so search indexing, title sanitization, and collision
// handling all behave exactly as they do in the app. It never touches
// the notes directory directly, and it never knows your login
// password - changing that from the app's Settings panel can't break
// this server, because it was never involved.
//
// Unlike Notespice's version of this (optional, off by default, token
// optional — built for trusted local/tailnet use), this server assumes
// it may be reachable from the public internet: the bearer token is
// mandatory, checked in constant time, and requests are rate-limited
// per source IP on top of whatever the reverse proxy already does.
//
// Environment:
//   NOTES_URL          base URL of the Claunote app   (default http://claunote:8080)
//   MCP_PORT           port to listen on               (default 4200)
//   MCP_SETTINGS_FILE  path to the token + connect/disconnect switch
//                      managed by the app's Settings panel (default
//                      /data/.mcp_settings.json). This is the normal
//                      path: mount the app's own data volume here
//                      read-only and it just works, no separate
//                      secret to generate or copy by hand. Re-read on
//                      every request, so a token regeneration or a
//                      disconnect flips take effect immediately.
//   MCP_TOKEN          bearer token, used only if MCP_SETTINGS_FILE
//                      isn't readable — an explicit override for
//                      setups that don't share the data volume. When
//                      falling back to this, the connect/disconnect
//                      switch doesn't exist; the server is always on.
//                      Generate a long random value, e.g.
//                      `openssl rand -hex 32`.

const fs = require("fs");
const crypto = require("crypto");
const express = require("express");
const { z } = require("zod");
const { McpServer } = require("@modelcontextprotocol/sdk/server/mcp.js");
const {
  StreamableHTTPServerTransport,
} = require("@modelcontextprotocol/sdk/server/streamableHttp.js");

const BASE = (process.env.NOTES_URL || "http://claunote:8080").replace(/\/+$/, "");
const PORT = parseInt(process.env.MCP_PORT || "4200", 10);
const SETTINGS_FILE = process.env.MCP_SETTINGS_FILE || "/data/.mcp_settings.json";
const TOKEN_ENV = process.env.MCP_TOKEN || "";

// Read fresh on every request rather than cached once at startup, so
// a change made in the app's Settings panel takes effect on the very
// next request instead of needing this container restarted.
function currentSettings() {
  try {
    const raw = JSON.parse(fs.readFileSync(SETTINGS_FILE, "utf8"));
    if (raw && typeof raw.token === "string" && raw.token) {
      return { token: raw.token, enabled: raw.enabled !== false };
    }
  } catch {
    // File missing, unreadable, or not valid JSON — fall through to
    // the env override, which has no concept of connect/disconnect.
  }
  return { token: TOKEN_ENV, enabled: true };
}

if (!currentSettings().token || currentSettings().token.length < 20) {
  console.error(
    "No usable MCP token found (checked " +
      SETTINGS_FILE +
      " and MCP_TOKEN) — this server is meant to run reachable from " +
      "the internet, unlike Notespice's optional/local-only version, " +
      "so it refuses to start without one. Mount the app's data volume " +
      "read-only here, or set MCP_TOKEN directly."
  );
  process.exit(1);
}

// ---------- Claunote API client ----------
// The same token this server checks incoming /mcp requests against is
// also what it presents to the app - it's both this server's own
// credential and the thing it authenticates its callers with.

async function api(path, opts = {}) {
  return fetch(`${BASE}/api${path}`, {
    ...opts,
    headers: { ...(opts.headers || {}), Authorization: `Bearer ${currentSettings().token}` },
  });
}

async function apiJson(path, opts = {}) {
  const res = await api(path, opts);
  if (!res.ok) {
    const body = await res.json().catch(() => ({}));
    throw new Error(body.error || `Claunote API error ${res.status}`);
  }
  return res.json();
}

// ---------- MCP tools ----------
const text = (s) => ({ content: [{ type: "text", text: s }] });

function buildServer() {
  const server = new McpServer({ name: "claunote", version: require("./package.json").version });

  server.registerTool(
    "list_notes",
    {
      title: "List notes",
      description:
        "List all notes with their titles and last-modified timestamps (unix seconds), most recently viewed first.",
      inputSchema: {},
    },
    async () => {
      const notes = await apiJson("/notes");
      return text(JSON.stringify(notes, null, 2));
    }
  );

  server.registerTool(
    "read_note",
    {
      title: "Read a note",
      description: "Read a note's full markdown content by its exact title.",
      inputSchema: { title: z.string().describe("Exact note title") },
    },
    async ({ title }) => {
      const note = await apiJson(`/notes/${encodeURIComponent(title)}`);
      return text(note.content);
    }
  );

  server.registerTool(
    "search_notes",
    {
      title: "Search notes",
      description:
        "Full-text search across all note titles and contents. Returns matching note titles, best match first.",
      inputSchema: { query: z.string().describe("Search terms") },
    },
    async ({ query }) => {
      const matches = await apiJson(`/search?q=${encodeURIComponent(query)}`);
      if (!matches.length) return text("No notes matched.");
      return text(matches.map((m) => m.title).join("\n"));
    }
  );

  server.registerTool(
    "create_note",
    {
      title: "Create a note",
      description:
        "Create a new note with the given title and GitHub Flavored Markdown content. If the title already exists, the note is created with a (1), (2), ... suffix rather than overwriting.",
      inputSchema: {
        title: z.string().describe("Note title (becomes the filename)"),
        content: z.string().describe("Markdown content"),
      },
    },
    async ({ title, content }) => {
      const created = await apiJson("/notes", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ title, content }),
      });
      return text(`Created note: ${created.title}`);
    }
  );

  server.registerTool(
    "update_note",
    {
      title: "Update a note",
      description:
        "Replace a note's markdown content (and optionally rename it). The previous content is overwritten — read it first if you need to preserve parts of it.",
      inputSchema: {
        title: z.string().describe("Exact current title of the note"),
        content: z.string().describe("New markdown content (full replacement)"),
        new_title: z.string().optional().describe("Optional new title (rename)"),
      },
    },
    async ({ title, content, new_title }) => {
      const body = { content };
      if (new_title) body.new_title = new_title;
      const updated = await apiJson(`/notes/${encodeURIComponent(title)}`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      });
      return text(`Updated note: ${updated.title}`);
    }
  );

  server.registerTool(
    "append_to_note",
    {
      title: "Append to a note",
      description:
        "Append markdown to the end of an existing note (separated by a blank line), without touching what's already there.",
      inputSchema: {
        title: z.string().describe("Exact note title"),
        content: z.string().describe("Markdown to append"),
      },
    },
    async ({ title, content }) => {
      const note = await apiJson(`/notes/${encodeURIComponent(title)}`);
      const merged = note.content.replace(/\s+$/, "") + "\n\n" + content;
      await apiJson(`/notes/${encodeURIComponent(title)}`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ content: merged }),
      });
      return text(`Appended to note: ${note.title}`);
    }
  );

  server.registerTool(
    "delete_note",
    {
      title: "Delete a note",
      description:
        "Permanently delete a note by its exact title. This cannot be undone — confirm with the user before calling it.",
      inputSchema: { title: z.string().describe("Exact note title") },
    },
    async ({ title }) => {
      await apiJson(`/notes/${encodeURIComponent(title)}`, { method: "DELETE" });
      return text(`Deleted note: ${title}`);
    }
  );

  return server;
}

// ---------- constant-time bearer check ----------
function tokenMatches(header, expectedToken) {
  const prefix = "Bearer ";
  if (typeof header !== "string" || !header.startsWith(prefix)) return false;
  const given = Buffer.from(header.slice(prefix.length));
  const expected = Buffer.from(expectedToken);
  if (given.length !== expected.length) {
    // Still do a comparison of matching length so this branch doesn't
    // return measurably faster than the equal-length path.
    crypto.timingSafeEqual(given, given);
    return false;
  }
  return crypto.timingSafeEqual(given, expected);
}

// ---------- simple per-IP rate limit ----------
// Belt-and-suspenders on top of whatever the reverse proxy does: caps
// brute-force guessing of MCP_TOKEN and general abuse. Self-pruning so
// memory doesn't grow unbounded under sustained attack.
const RATE_LIMIT_WINDOW_MS = 60_000;
const RATE_LIMIT_MAX = 60;
const hits = new Map();

function rateLimited(ip) {
  const now = Date.now();
  for (const [key, entry] of hits) {
    if (now - entry.start > RATE_LIMIT_WINDOW_MS) hits.delete(key);
  }
  const entry = hits.get(ip);
  if (!entry || now - entry.start > RATE_LIMIT_WINDOW_MS) {
    hits.set(ip, { start: now, count: 1 });
    return false;
  }
  entry.count += 1;
  return entry.count > RATE_LIMIT_MAX;
}

// ---------- Streamable HTTP endpoint ----------
const app = express();
// Behind Caddy on the VPS: trust the immediate proxy hop so req.ip is
// the real client address, not the proxy's, for rate limiting and logs.
app.set("trust proxy", 1);
app.use(express.json({ limit: "25mb" }));

// Health probe: registered BEFORE the auth gate on purpose. The
// container's own Docker HEALTHCHECK (and any monitoring) probes this
// without credentials — guarding it made the container permanently
// "unhealthy" whenever MCP_TOKEN was set. It leaks nothing.
app.get("/healthz", (req, res) => res.json({ ok: true }));

app.use((req, res, next) => {
  if (rateLimited(req.ip)) {
    res.status(429).json({ error: "too many requests" });
    return;
  }
  const { token, enabled } = currentSettings();
  // Checked even for a well-formed, correct token: disconnecting from
  // the app's Settings panel must actually cut access, not just hide
  // the token from the UI.
  if (!enabled) {
    res.status(503).json({ error: "MCP access is disconnected in Claunote's settings" });
    return;
  }
  if (!tokenMatches(req.headers.authorization, token)) {
    res.status(401).json({ error: "unauthorized" });
    return;
  }
  next();
});

// Stateless mode: every POST gets a fresh server+transport pair. No
// session state lives here (the Claunote session cookie is module
// level), which keeps this compatible with every Claude surface.
app.post("/mcp", async (req, res) => {
  try {
    const server = buildServer();
    const transport = new StreamableHTTPServerTransport({
      sessionIdGenerator: undefined,
    });
    res.on("close", () => {
      transport.close();
      server.close();
    });
    await server.connect(transport);
    await transport.handleRequest(req, res, req.body);
  } catch (err) {
    console.error("MCP request failed:", err);
    if (!res.headersSent) {
      res.status(500).json({
        jsonrpc: "2.0",
        error: { code: -32603, message: "Internal server error" },
        id: null,
      });
    }
  }
});

// Stateless servers have no long-lived stream or session to manage.
const methodNotAllowed = (req, res) =>
  res.status(405).json({
    jsonrpc: "2.0",
    error: { code: -32000, message: "Method not allowed" },
    id: null,
  });
app.get("/mcp", methodNotAllowed);
app.delete("/mcp", methodNotAllowed);

app.listen(PORT, () => {
  console.log(`Claunote MCP server listening on :${PORT} (endpoint: /mcp), talking to ${BASE}`);
});
