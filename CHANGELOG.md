# Changelog

## 0.2.1 — 2026-08-17

- Renamed the GHCR-publishing secret from the generic `GHCR_PAT` to
  `CLAUNOTE_GHCR_PAT`, so it's identifiable at a glance alongside
  every other project's own publishing token instead of looking
  identical to all of them.

## 0.2.0 — 2026-08-17

The headline difference from Notespice: a built-in, hardened MCP
server, plus a security posture built for internet exposure rather
than a trusted home network.

- Restored and hardened the MCP companion server Notespice once had
  and later removed (`mcp/`, Node.js, official MCP SDK, Streamable
  HTTP). Same tools — `list_notes`, `read_note`, `search_notes`,
  `create_note`, `update_note`, `append_to_note`, `delete_note` — but
  `MCP_TOKEN` is now mandatory (the container refuses to start
  without one) instead of optional, checked in constant time, plus
  its own per-IP rate limit independent of the reverse proxy in
  front of it. CI now also builds and publishes
  `ghcr.io/lightmorphic/claunote-mcp`.
- `docker-compose.yml` now assumes internet exposure by default:
  both containers run with every capability dropped,
  `no-new-privileges`, a read-only root filesystem, and memory/PID
  limits; both ports bind to `127.0.0.1` only rather than every
  interface, requiring a reverse proxy in front.
- Added `Strict-Transport-Security` to the app's response headers,
  sent automatically whenever secure-cookie mode is on (the same
  signal already used for the session cookie's `Secure` flag).
- Rebuilt the marketing site with its own identity instead of
  mirroring Notespice's: a violet accent instead of amber, cooler
  neutrals, copy led by the MCP integration instead of the
  just-markdown-files pitch, and new "Built for Claude" and
  "Security" sections.

## 0.1.0 — 2026-08-15

Forked from [Notespice](https://github.com/lightmorphic/notespice) as
Claunote, a separate ongoing project from this point forward.
Renamed everywhere it appears — package name, binary, Docker image
(`ghcr.io/lightmorphic/claunote`), session cookie, PWA manifest,
service worker cache, README, and the marketing site (now served at
claunote.lightmorphic.co.uk instead of notespice.org). Version reset
to 0.1.0 for Claunote's own release train; everything below this line
is inherited history from before the fork and is left as written.

## 1.8.9 — 2026-08-10

- Centered the login screen's logo and "Notespice" heading; it was
  left-aligned inside the card before.

## 1.8.8 — 2026-08-10

- Centered the Why-section Lightmorphic badge instead of left-aligning
  it.
- Replaced the footer's old inline "Sponsored by" credit-line mark
  with the same standalone Lightmorphic badge used in the Why
  section, and removed the now-unused `.sponsor-mark`/`.sponsor-logo`
  CSS and its duplicate base64-embedded logos.

## 1.8.7 — 2026-08-10

- Added a standalone "Created by Lightmorphic" badge under the Why
  section's text on the notespice.org site, same pill shape as the
  one already in the app and in the footer credit line. Logo files
  moved to `site/images/` instead of embedding another pair of
  base64 blobs in `index.html`.

## 1.8.6 — 2026-08-10

- Switched the app's own typeface from self-hosted Geist to
  self-hosted Manrope (`static/fonts/manrope.woff2`), matching the
  notespice.org site and Lightmorphic's house font. One variable file
  covers weight 200-800; the two Latin/Latin-ext Geist files and their
  license are removed. Service worker shell cache bumped v4 -> v5 to
  drop the old font files and pick up the new one.

## 1.8.5 — 2026-08-10

- Added `.github/workflows/pages.yml` to deploy `site/` to GitHub
  Pages on every push to `main` that touches it, matching the
  repo-owned-Pages-source-is-GitHub-Actions setup on notespice.org.

## 1.8.4 — 2026-08-10

- Switched the notespice.org site's typeface from an embedded-base64
  Geist to self-hosted Manrope (`site/fonts/manrope.woff2`), matching
  Lightmorphic's own house font. Dropped the base64-embedded font data
  from `site/index.html` in favor of a real local file, cutting the
  page from 173KB to 92KB.

## 1.8.3 — 2026-08-10

- Added the notespice.org marketing site under `site/`, folded into
  this repo instead of living as a separate one (its old standalone
  repo no longer exists under the new GitHub org). Deployment is
  handled separately via GitHub Actions/Pages.
- Fixed leftover `FOSSCharlie`/`fosscharlie` GitHub and GHCR
  references (the app's UI, README, and docker-compose.yml, plus the
  new site) left over from the repo's move to the `lightmorphic` org.

## 1.8.2 — 2026-08-10

- Added a small "Created by Lightmorphic" badge to the sidebar footer,
  linking to lightmorphic.co.uk. Uses the Lightmorphic horizontal
  logo lockup, swapping between a light-background and dark-background
  variant to match the app's own light/dark theme.

## 1.8.1 — 2026-08-05

- Fixed the "New note" button being unreachable on real phones. The
  page sized itself with `100vh`, which is the *largest* possible
  mobile viewport (it includes the strip hidden behind the browser's
  collapsible address bar); combined with `overflow: hidden`, that
  strip clipped off anything pinned near the bottom of the screen -
  including the FAB - whenever the address bar was showing, which on
  a real phone is most of the time. Now sized with `100dvh` (falling
  back to `100vh` on older browsers), which always tracks the actual
  visible viewport.
- Fixed the editor chrome (toolbar, blank canvas) rendering behind the
  "No notes yet" message before any note had ever been created. A
  `.editor { display: flex }` rule was winning a same-specificity tie
  against the `[hidden]` attribute, the same class of bug already
  fixed for `.format-bar` in 1.5.x but missed on `.editor` itself.

## 1.8.0 — 2026-08-05

Full pre-deployment polish pass over the whole app, verified in a real
browser at every step (16-step click-through, 58-scenario converter
suite, and targeted reproductions).

- Fixed a data-loss bug: switching notes or logging out while an
  autosave was still pending (the editor debounces saves until 500ms
  after the last keystroke) silently dropped everything typed since
  the last completed save. Pending saves are now flushed before the
  editor switches away, and closing or backgrounding the tab sends the
  save with `keepalive` so it completes even after the page is gone.
- Security headers on every response, set by the app itself so every
  deployment gets them with or without a reverse proxy:
  `Content-Security-Policy` (own-origin scripts and styles only;
  notes can still embed images by https URL), `X-Content-Type-Options:
  nosniff`, `X-Frame-Options: DENY`, and `Referrer-Policy:
  no-referrer`.
- PWA manifest colors caught up with the redesign: theme color is now
  the app's yellow (#FFC107) and the splash background the dark navy
  (#10141C). Installed-app splash screens previously still showed the
  pre-redesign orange and graphite.
- Accessibility: the search box, note title, Writer surface, and
  Markdown textarea now expose proper labels and roles to screen
  readers. All color pairs measured at AA contrast or better (weakest
  6.1:1).
- "New note" no longer opens the freshly created note twice (two
  redundant requests when no note was open).
- The zip-import per-entry cap now derives from the single-file
  upload cap instead of restating the same number.
- Removed the unused `favicon.png` (nothing referenced it), the
  finished one-time backfill-releases workflow, and a stale
  `.gitignore` entry.
- README corrected: the Docker publish workflow authenticates with
  the `GHCR_PAT` repository secret, not the default `GITHUB_TOKEN`.
- Writing sweep across the README, SECURITY.md, UI copy, and code
  comments per the house style rules; wording only, no functional
  changes.
- `cargo audit`: clean, 0 advisories across all 120 dependencies.
  Service worker cache bumped to v4 so updated files are picked up.

## 1.7.2 — 2026-07-29

- Reverted 1.7.1: the login screen logo is back to its previous
  32px inline size next to the app name.

## 1.7.1 — 2026-07-29

- The login screen now shows the logo at its original size (192px),
  centered above the app name — it was previously scaled down to a
  32px inline icon next to the title.

## 1.6.0 — 2026-07-28

- Added the OCI `org.opencontainers.image.source` label to the
  Docker image, so GHCR automatically links the published package
  back to this repository.

## 1.5.9 — 2026-07-28

- Typeface changed from Urbanist to Geist, everywhere — UI, Writer,
  Markdown view, and code blocks. Urbanist is removed completely:
  font files, license file, `@font-face` rules, stylesheet
  references, service-worker cache entries, and documentation.
  Geist is self-hosted as a variable font (weights 100-900, latin +
  latin-ext, SIL Open Font License) — verified in a real browser
  that the page loads it from local files with zero external
  network requests of any kind. Geist ships no italic faces, so
  italics render as a browser-synthesized oblique. Service-worker
  cache bumped (v3) so installed PWAs pick up the new font on next
  load.

## 1.5.8 — 2026-07-28

- Dark theme recolored from warm browns to a cool navy palette: the
  main surface is now `#10141C` and container boxes (sidebar drawer,
  code blocks, login card) are `#19212B`, with borders, input fields,
  canvas, and muted text derived from the same hue family. The brand
  yellow and the light theme are unchanged.

## 1.5.7 — 2026-07-28

- Removed the emoji icons from the README's feature list.

## 1.5.6 — 2026-07-28

- Blank lines in Writer are now REAL lines, not spacing tricks. The
  parser previously split plain text into separate paragraph blocks
  with a CSS margin as the visible gap — which meant the blank line
  between two paragraphs wasn't actually there: pressing the down
  arrow jumped straight from one paragraph to the next, skipping the
  gap the eye could see, unlike the Markdown view where every blank
  line is a real line the cursor can land on. Consecutive text
  paragraphs now parse into a single flow where a paragraph break is
  a real rendered blank line (and each explicit `<br>` line one
  more), exactly matching what typing in Writer produces. Verified in
  a real browser: arrow keys now move one visual line per press
  through blank lines, identically to the Markdown textarea; every
  gap is pixel-identical between the two views (24px per line, blank
  lines included); the full 58-scenario round-trip suite still
  passes; and the saved markdown is byte-identical through repeated
  mode switches.

## 1.5.5 — 2026-07-27

- The sidebar now shows "Notespice vX.Y.Z" beneath Log out, linking
  to the GitHub repository — so which build is running is always one
  glance away. The version comes from the server itself (baked in
  from Cargo.toml at compile time and reported via `/api/session`),
  not from the frontend files, so it can't drift from the binary
  actually serving the app.

## 1.5.4 — 2026-07-27

- A paragraph break now renders in Writer as exactly one full blank
  line — the same gap the Markdown view shows for its literal blank
  line. Previously it rendered as only a small spacing bump, so a
  note with a paragraph break followed by plain line breaks showed
  three subtly-different line gaps in Writer while Markdown mode
  showed a clear blank line: the two views disagreed about the same
  content. Measured in-browser after the fix: paragraph gap 48px,
  plain line break 24px, in both views identically.
- The release workflow can now also be run manually (workflow
  dispatch with a version input): it creates the tag at main's head
  and the release in one step, for situations where pushing a tag
  directly isn't possible.

## 1.5.3 — 2026-07-27

- Releases are now created automatically: pushing a version tag (e.g.
  `1.5.3`) triggers a new `release.yml` workflow that builds the
  GitHub release in the format this project has always used by hand —
  tag as the bare version number, title taken from the changelog
  heading (`1.5.3 — YYYY-MM-DD`), and the release body copied verbatim
  from that version's CHANGELOG.md section. Cutting a release is now
  just `git tag -a <version> -m "Notespice <version>" && git push
  origin <version>`.
- A companion one-time `backfill-releases.yml` workflow (manual
  trigger, idempotent, safe to delete afterwards) creates the missing
  releases for every version that shipped without one — 1.3.1 through
  1.5.3 — each tagged at the commit where that version first landed.

## 1.5.2 — 2026-07-27

- Documentation refresh: the README's security notes claimed a 5MB
  request-body cap when the actual limit is 25MB (with a 20MB
  per-file attachment cap), and didn't mention the decompressed-size
  caps on zip imports added in 1.3.0. Both corrected. Added the
  `tests/` directory to the project-structure listing, and a
  Development note pointing at the 58-scenario end-to-end
  Writer<->Markdown suite that guards the converter.

## 1.5.1 — 2026-07-27

- The save indicator now always shows a state: "saved" from the moment
  a note is opened, rather than sitting blank until the first edit.
- The app bar's four action groups — undo/redo, Writer/Markdown, the
  save state, and delete — now sit at equal distances: a uniform gap
  on desktop, spread edge-to-edge on mobile. Fixed the mobile row
  overflowing a phone-width screen (the delete button was pushed past
  the right edge, and the segmented control truncated to "Mari…") —
  the compact mobile sizing was being silently overridden by
  equal-specificity rules later in the stylesheet, so it now lives at
  the end of the file where it wins the cascade.

## 1.5.0 — 2026-07-27

Visual redesign release: Material design language, Urbanist type,
same yellow.

- **Typeface: Urbanist everywhere.** Inter is gone entirely; so is
  the monospace face that Markdown mode and code blocks used.
  Urbanist is self-hosted as a variable font (one face covers every
  weight from 100 to 900, plus true italics) — still no Google Fonts
  CDN, no external font request of any kind. License file swapped to
  the Urbanist OFL.
- **Material design treatment throughout**, keeping the exact same
  brand yellow (#FFC107) as the highlight color in both themes:
  - "New note" is now an extended floating action button, bottom
    right — the primary action, always on screen and thumb-reachable
    on mobile without opening the drawer first (it used to be a tiny
    "+" hidden next to the search box, inside the drawer).
  - The sidebar is a proper navigation drawer: pill-shaped note rows
    with a yellow-tinted active state, a rounded Material search bar,
    and pill-shaped Export/Import/Log out buttons.
  - Writer/Markdown is a segmented control instead of two bare text
    labels — the active mode also no longer renders white-on-white
    in light theme (previously a hardcoded `#fff`).
  - Round icon buttons with Material state-layer hovers everywhere:
    app bar, format bar, drawer.
  - The brand mark now sits small and quiet in the top app bar, so
    it stays visible even when the drawer is collapsed.
  - Warm yellow-biased neutrals in both themes instead of flat grays;
    links and callout titles use a dark amber in light theme (the raw
    brand yellow was unreadable on a light ground) and the brand
    yellow in dark theme.
  - Visible focus rings for keyboard navigation, and all transitions
    disabled under `prefers-reduced-motion`.
- Service-worker cache bumped (v2) and the font files added to the
  cached shell, so installed PWAs pick the redesign up on next load.
- PWA theme color updated from the old orange to the brand yellow.

## 1.4.0 — 2026-07-25

Writer <-> Markdown fidelity release. Every fix below was found by a
new 58-scenario end-to-end suite driving the real app in a real
browser (now committed as `tests/e2e-roundtrip.js`), and all 58
scenarios pass after these changes — covering the full GFM feature
set in both directions plus real toolbar/keyboard interactions.

- **Inline formatting inside blockquotes was destroyed on save** —
  bold/italic/strikethrough/code in a quote silently became plain
  text (`> **a** b` -> `> a b`, permanently). The quote serializer
  read `textContent`, which strips every inline element. This is the
  likely mechanism behind "I made text bold, switched to Markdown,
  and it wasn't bold" reports. Now serializes quote content through
  the same inline-aware path as everything else.
- **Enter could never create a second list item.** The custom Enter
  handler unconditionally inserted a line break, which inside a list
  item just breaks the line *within* that item — and the resulting
  markdown drifted on every subsequent mode switch. Enter inside a
  list now uses the browser's native behavior: split into a new item,
  or exit the list when the current item is empty.
- **Indenting a list item could erase the whole list.** Browsers'
  native indent nests a `<ul>` directly inside a `<ul>` with no
  wrapping `<li>` — invalid HTML, but real — and the serializer only
  looked for `<li>` children, so such a list serialized to an empty
  string: total data loss on save. The list serializer now walks all
  child elements and handles directly-nested lists.
- **Enter inside a code block lost the line break on save.** The
  code-block serializer read `textContent`, to which a `<br>`
  contributes nothing — two code lines silently joined into one.
  Enter in a code block now inserts a literal newline character, and
  the serializer converts any `<br>` it still finds into a newline.
- **GFM hard breaks were downgraded to soft breaks on resave.** A
  note containing a real hard break (two trailing spaces before the
  newline) lost those spaces the first time it was saved from Writer
  mode, because a single `<br>` serialized as a bare `\n` — the same
  bytes a soft break round-trips as, so the two could not coexist.
  A single line break (including one typed with Enter) now serializes
  as a proper GFM hard break. Visually identical in Writer and on
  GitHub; the saved markdown is now valid GFM for a forced break and
  survives round trips.
- Hardening from the same test pass: formatting expressed as styled
  `<span>`s (what some browsers produce instead of `<b>`/`<i>`) is
  now read off the inline style instead of dropped; an inline
  formatting element left holding only whitespace/breaks no longer
  emits orphaned `**`/`*` markers; and content that must stay on one
  markdown line (headings, table cells, list items, quotes, callout
  bodies, footnote definitions) has embedded line breaks flattened to
  spaces so a stray break can't corrupt the construct's syntax.

## 1.3.10 — 2026-07-24

- Fixed three Enters in a row silently doing nothing beyond a normal
  paragraph break, instead of adding the extra explicit line the
  design has always intended. Root cause, confirmed via direct DOM
  inspection in a real browser: inserting a `<br>` at the very start
  of an existing text node (e.g. right after a previous Enter's
  zero-width-space filler) splits that node, leaving an invisible
  *empty* text node behind as the new `<br>`'s previous sibling. That
  phantom node broke the run-length count used to tell a soft break
  from a paragraph break from an explicit `<br>` line — a run of 3
  consecutive `<br>`s got fragmented into a run of 1 and a run of 2,
  neither of which reaches the "add an explicit line" threshold.
  Fixed by removing that empty text node immediately after insertion.
  Verified against the exact reported case, and confirmed it survives
  a real save, page reload, and repeated mode-switching without
  drifting — this also resolves the long-standing caveat that
  resaving a 3-Enter note without further edits collapsed it back to
  a plain paragraph break.
- Fixed the "/" search-focus shortcut stealing keystrokes anywhere in
  the app, not just outside of it. It only ever excluded the search
  box itself, so typing a URL like `https://example.com` directly
  into a note's title, Writer view, or Markdown view yanked focus to
  search mid-keystroke and dropped the `/`. Now skips the shortcut
  whenever focus is already on anything you can type into — any
  `input`/`textarea`/`select`, or the Writer editor's `contenteditable`
  — not just the search box specifically.

## 1.3.9 — 2026-07-23

- Fixed the formatting toolbar staying visible in Markdown mode. The JS
  already set the bar's `hidden` attribute on every mode switch, but
  `.format-bar { display: flex }` and the browser's built-in
  `[hidden] { display: none }` have identical specificity, so the
  author rule won the tie and the bar never hid. Added
  `.format-bar[hidden] { display: none }` (attribute selector raises
  specificity) — the same fix already used for the login/app screens.
- Spread the mobile editor toolbar evenly across its four groups. Undo
  and redo are now wrapped in a `.undo-redo-group` so they stay a pair,
  and the mobile toolbar row uses `justify-content: space-between`
  (was `flex-end`) so Undo/Redo, the Writer/Markdown toggle, the save
  indicator, and Delete distribute across the full width.

## 1.3.8 — 2026-07-23

- Matched the Writer editor's line spacing to Markdown mode's
  (line-height 1.6, was 2) so switching between the two views doesn't
  change the visible line gap.

## 1.3.7 — 2026-07-23

- Built and ran a comprehensive round-trip test suite covering every
  GFM feature this app supports (empty content, single lines, soft
  breaks, paragraph breaks, explicit `<br>` lines, all six heading
  levels, bold, italic, strikethrough, inline code, all combinations
  of inline formatting, links, images, unordered lists, ordered lists,
  nested lists, task lists, blockquotes, code blocks with and without
  language, tables, horizontal rules, GitHub-style callouts,
  footnotes, and every combination of these), plus an 11-scenario
  user-workflow test simulating the exact "build in Writer, switch
  to Markdown, switch back" sequence for each pattern. 68 scenarios
  total, all passing. Each verifies not just that the round trip is
  correct, but that repeating it is idempotent — switching modes back
  and forth does not drift.
- One real bug caught by that testing and fixed: the 3-or-more-Enter
  pattern (which serializes to a paragraph break plus an explicit
  `<br>` line) lost that extra `<br>` line when switching Markdown ->
  Writer -> Markdown a second time. Root cause: when a paragraph in
  the DOM started with a `<br>`, the serializer's leading-whitespace
  strip was erasing what should have been an explicit `<br>` line
  marker. Now counts and preserves leading `<br>`s per paragraph.

## 1.3.6 — 2026-07-23

- Fixed Enter at the end of an inline formatting element (bold,
  italic, code, etc.) producing broken markdown: the `<br>` was
  inserted *inside* the element, so the closing marker got orphaned
  onto its own line — `**a**` typed then Enter pressed at the end of
  `a` serialized as `**a\n**\nb` instead of `**a**\nb`. Now detects
  when the cursor is at the very end (or very start) of an inline
  element and inserts the `<br>` outside that element. Verified
  against the exact reported scenario; full existing test suite
  still passes.
- Increased the visible line spacing in the Writer editor by about
  50% (line-height 1.8 instead of the browser default ~1.2).

## 1.3.5 — 2026-07-23

- Finally fixed the disappearing-content bug after four rounds of
  wrong theories. The actual root cause, identified from a browser
  DevTools screenshot of the real DOM: when typing straight into the
  editor `<div>`, the content ended up as bare text nodes and `<br>`s
  as direct children of that div — no wrapping `<p>` at all. The
  markdown serializer's top-level walk had `if (node.nodeType !== 1)
  return`, so every bare text node ("a", "b", "c") at the top level
  was silently dropped, leaving nothing to serialize. That's the
  blank content. Fixed by gathering consecutive bare text and inline
  elements at the top level into an implicit paragraph, flushing it
  whenever a real block element is encountered. Verified against the
  exact DOM from the screenshot and against the full existing GFM
  test suite. Genuinely could not have found this without the DOM
  inspection — the previous four rounds were all reasoning about
  what the DOM *should* be, not what it actually was.

## 1.3.4 — 2026-07-23

- Fixed pasting multi-line text producing inconsistent, worsening
  corruption on each mode switch (extra blank lines that changed each
  time, and eventually lost content). Same root cause as the Enter-key
  bug fixed in the previous two entries: paste was still using
  `execCommand("insertText")` with a multi-line string, the same
  category of browser-dependent behavior that's unverifiable in this
  environment. Replaced with the same manual, tested Range-based
  insertion used for Enter — split the pasted text on newlines and
  insert a real `<br>` between each line directly, rather than
  delegating a multi-line string to execCommand and hoping for a
  predictable result.

## 1.3.3 — 2026-07-23

- Found the actual bug behind "needs a double Enter to reach the next
  line" — the previous entry's filler-character fix only checked
  whether the inserted `<br>` had *no* next sibling. Proved with a
  direct test that this check was wrong for the single most common
  case: pressing Enter with existing text before the cursor. Inserting
  a node into an existing text node splits it, leaving an *empty*
  text node as the `<br>`'s next sibling — not `null`, so the previous
  check incorrectly skipped the filler every time there was already
  text on the line, which is effectively always. An empty trailing
  text node contributes nothing visually, same as no sibling at all,
  so the browser still had nothing to render a line for. Now treats
  an empty text-node sibling the same as no sibling, and reuses it
  for the filler rather than creating a redundant extra node.

## 1.3.2 — 2026-07-23

- Found the actual root cause of the previous entry's data-loss bug
  still happening after that fix: a `<br>` with nothing after it (the
  normal case — pressing Enter at the end of what you're typing)
  doesn't render a visible new line by itself in most browsers, so the
  first Enter appeared to do nothing. The natural reaction — pressing
  Enter again — inserted a *second* real `<br>`, which the run-length
  logic correctly reads as a paragraph break rather than a second
  simple line break, producing structure the person never intended.
  Fixed by inserting a zero-width-space filler character (not a
  second real `<br>`) so the browser has something to render a line
  for, without it counting as an extra break when saved. Stripped
  before saving in the normal case where typing continues right after
  (verified: the filler gets naturally consumed), and explicitly
  stripped as a fallback if a person stops typing right after
  pressing Enter and the filler is still sitting there dangling.
- Honest limitation, given two rounds of this: I still don't have a
  real browser available to verify visual rendering directly — only
  the resulting DOM structure and serialization, which I can and do
  test rigorously (traced through the exact reported keystroke
  sequence step by step both times). If this still isn't right, the
  next useful thing to report is exactly what the DOM looks like via
  the browser inspector after the sequence that breaks, since that's
  the one thing I can't currently see for myself.

## 1.3.1 — 2026-07-23

- Fixed a serious data-loss bug: typing multiple lines in Writer mode
  (e.g. three lines separated by single Enter presses) could produce
  completely blank content when switching to Markdown mode, and stay
  blank switching back. Root cause: Enter was handled via
  `execCommand("insertLineBreak")`, whose exact resulting DOM
  structure is browser-dependent in ways this environment has no way
  to verify (no real browser available, and jsdom doesn't implement
  `execCommand` at all). Replaced with the same manual Range-based
  insertion already used and verified elsewhere in this file — traced
  through the exact type/Enter/type/Enter/type sequence step by step,
  confirmed it now produces a clean, predictable structure, and
  confirmed that structure serializes correctly rather than blank.
- Fixed soft breaks (a single Enter/Shift+Enter, or a bare newline in
  markdown typed directly) rendering as one run-on line instead of a
  visible line break — e.g. three lines typed directly in Markdown
  mode collapsed into one line in Writer mode. GitHub's own GFM
  renderer treats a soft break as an actual visible line break (a
  well-known deviation from strict CommonMark, where it's ambiguous
  and often collapses to a space); matched that with `white-space:
  pre-line` on paragraph content, rather than the browser's default
  whitespace-collapsing behavior.

## 1.3.0 — 2026-07-23

Security and code-quality audit.

- **XSS (fixed):** a note containing `[x](javascript:...)` — typed
  directly, or imported from a file — produced a real clickable link
  that executed arbitrary script in the logged-in session. Added a
  URL sanitizer (allow-list: `http(s)`, `mailto`, relative paths;
  everything else neutralized) and applied it everywhere a URL is
  inserted into `href`/`src`, both in the markdown parser and all four
  toolbar insertion paths. Verified against `javascript:`, `vbscript:`,
  and `data:` payloads.
- **Zip-bomb DoS (fixed):** import read each zip entry's decompressed
  content with no size limit at all — a few KB compressed could
  decompress to gigabytes. Added a 20MB per-entry cap and a 200MB
  cumulative cap across the whole archive. Verified against an actual
  25MB-decompressed/25KB-compressed test file: correctly rejected,
  nothing written to disk, server stayed healthy.
- **Writer is GFM-only, enforced, not just assumed:** paste is now
  forced to plain text only (rich HTML from Word/Google Docs/a webpage
  can carry both non-GFM formatting and markup that bypasses the
  parser's own URL sanitization), native drag-and-drop is blocked
  (same reasoning — use the Upload/Attach buttons instead), and Ctrl+U
  is blocked explicitly (browsers auto-wire this to underline for any
  `contenteditable`, with no code of ours calling for it).
- **Unbounded memory growth (fixed):** both the session map and the
  failed-login-attempts map in `auth.rs` only ever grew, cleaned up
  lazily only if the exact same key was looked up again. Since the
  rate-limit key is the raw client IP, and IPv6's address space makes
  generating huge numbers of distinct source addresses trivial, this
  was a real exhaustion vector. Fixed by sweeping expired entries on
  every new insert in both maps.
- **Search bug (found and fixed):** a note whose only distinctive word
  was in its *title*, not its body, was completely unfindable —
  the index only ever tokenized note content, and the separate
  title-match boost only re-ranks notes that already matched via
  content, it was never an independent source of matches. Now indexes
  title words too. Found by testing, not by reading the code.
- **Search optimization:** switched the index's outer map from
  `HashMap` to `BTreeMap`, so prefix matching (the common case —
  partial words like "fold" should match "folding") is now a real
  sorted-range query instead of a full linear scan over every unique
  token in the index on every search request.
- Removed a dead parameter (`title_lookup`) from `search()` that was
  always passed as `|_| true` and never actually invoked anywhere.
- Audited: path-traversal defenses in `store.rs` (titles and
  attachment filenames) tested directly against `../../`, absolute
  paths, and null-byte payloads sent through the real running API —
  all correctly contained. Confirmed attachment downloads require
  auth. Confirmed the global 25MB request body limit is actually
  enforced (413, server stays healthy). All 120 dependencies in
  `Cargo.lock` checked against the current RustSec advisory database
  with precise semver comparison — zero actionable vulnerabilities.
  Every `.unwrap()`/`.expect()` in the Rust source reviewed — none
  reachable from untrusted input in a way that could panic.

## 1.2.0 — 2026-07-23

- Enter and Shift+Enter now behave identically, and what they produce
  depends on how many land in a row rather than which key was used:
  one is a plain soft break (no gap), two is a real paragraph break
  (blank line), and three or more adds an explicit `<br>` line on top
  of the paragraph break — since GFM collapses any number of blank
  lines to a single paragraph break, an actual line-break tag is the
  only way to force extra space beyond that. Verified all three cases
  produce exactly this, that a saved `<br>` line reopens as a real
  line break rather than literal text, and that the existing GFM
  suite and prior fixes still pass.
- One honest caveat: reopening a note that has the three-or-more-break
  pattern and resaving it *without further edits* collapses it back to
  a plain paragraph break, since reopening only recreates one real
  `<br>` element rather than the original three. The markdown stays
  valid either way; it just loses the extra spacing if resaved
  untouched immediately after reopening.

## 1.1.3 — 2026-07-23

- Fixed Shift+Enter producing markdown that isn't valid GFM: a bare
  single newline is only a "soft break" in the spec, which most
  renderers collapse to a space rather than an actual line break —
  and the parser didn't even round-trip it correctly, reading it back
  as two separate paragraphs, identical to a blank line. A real GFM
  hard break requires two trailing spaces before the newline (or a
  trailing backslash). Fixed both directions: `<br>` now serializes
  to that exact syntax, and the parser properly consumes multi-line
  paragraphs, distinguishing an explicit hard break from a plain soft
  break rather than treating every line as its own paragraph. Verified
  the full round trip is now exact for both break types, and confirmed
  the existing GFM feature suite and the earlier nested-content fix
  still pass.

## 1.1.2 — 2026-07-22

- Fixed a real data-loss bug in the Writer editor: typing multiple
  lines and switching to Markdown mode could silently lose earlier
  lines entirely. Root cause: browsers are inconsistent about what
  element Enter produces in a `contenteditable` (Chrome defaults to
  `<div>`, and the exact nesting can vary by cursor position), and the
  markdown converter walked the DOM expecting clean sibling elements.
  Fixed two ways: explicitly setting the default paragraph separator
  so Enter reliably produces a real paragraph, and making the
  converter itself defensive against unexpected nesting (verified with
  several simulated bad-nesting cases, including one that reproduced
  the exact symptom, before and after the fix). Also confirms the
  Enter-vs-Shift+Enter distinction (blank line vs. no gap) was already
  correct, expected behavior once nesting is handled properly, not a
  separate bug.

## 1.1.1 — 2026-07-22

- Fixed undo/redo, the Writer/Markdown toggle, and delete being
  unreachable on narrow/mobile screens — the toolbar is one row that
  doesn't fit that content plus the title on screens this width, so
  those controls were getting squeezed off rather than actually
  disappearing. Now wraps into two rows on narrow screens, with that
  action group on its own row above the title.

## 1.1.0 — 2026-07-22

- Fixed the sidebar not actually reflecting recently-viewed order:
  the server was always recording views and reordering correctly, but
  the frontend re-rendered the list from a stale array after opening
  a note instead of re-fetching, so the reorder never showed up until
  something else happened to trigger a full reload.
- Added a pin button (top-right of the sidebar) that keeps the
  sidebar open regardless of clicking the editor, typing, or
  selecting a note — previously it always auto-collapsed on any of
  those.
- "New note" no longer prompts for a title in a dialog — it creates
  an "Untitled" note immediately (deduping against existing titles)
  and opens it with the title field focused and selected, so you
  rename it in place instead.
- Delete no longer uses a browser `confirm()` dialog — clicking it
  turns the button into an inline red "Confirm" button (matching the
  pattern used in chkt) that reverts after 4 seconds or a click
  elsewhere if you don't follow through.

## 1.0.0 — 2026-07-22

Initial release.

- Notes stored as plain `.md` files, filename = title — no database.
  Attachments (images, uploads, generic files) stored alongside them
  under `files/`, referenced by ordinary markdown links.
- Storage split into two directories: `NOTES_DIR` for notes and
  attachments (the actual vault), `NOTES_DATA_DIR` for app-only state
  (currently just the recently-viewed list) — kept separate so nothing
  that isn't your data ever lives inside your vault.
- Full GitHub Flavored Markdown toolbar: headings, bold/italic/
  strikethrough/inline code, all three list types with indent/outdent,
  blockquotes, fenced code blocks, tables, horizontal rules, footnotes,
  and GitHub-style callouts (`> [!NOTE]` etc.) — rendered with the same
  colored-box treatment GitHub.com uses.
- WYSIWYG editor with a one-click Writer/Markdown toggle and undo/redo.
  A small hand-written markdown ⇄ HTML converter, not a third-party
  editor library loaded from a CDN — nothing external to
  version-mismatch or break.
- In-memory inverted-index full-text search, rebuilt from disk at
  startup.
- Sidebar shows the last 10 *viewed* notes first (most-recent-open at
  top, not last-edited), falling back to last-modified order for
  everything else. Collapsible/overlay sidebar on narrow screens, with
  instant name-filtering and full-content search on Enter.
- Export to a dated zip; import that same zip (or a loose `.md` file)
  back in, deduplicating on title collision rather than overwriting.
- Installable PWA — manifest, icons, and a service worker that caches
  only the static shell, never note content.
- Single username/password auth: Argon2id hashing, server-side session
  tokens, per-IP login rate limiting.
- Self-hosted Inter typeface — no Google Fonts CDN, no external font
  request of any kind.
- Multi-stage Dockerfile (non-root user, OS packages patched at build
  time, healthcheck), docker-compose example, and a GitHub Actions
  workflow publishing to GHCR with a weekly rebuild for security
  patches.
