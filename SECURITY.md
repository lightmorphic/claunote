# Security Policy

Claunote is a personal, single-maintainer project, not a company or a
team with a dedicated security function. This document sets realistic
expectations rather than promising something that isn't true.

## Supported versions

Only the latest release (the `latest` tag / most recent tagged commit) is
supported. There's no long-term-support branch and no backporting of
fixes to older versions: if a vulnerability is found, the fix lands in
the next build, and you're expected to update to it.

## Reporting a vulnerability

**Please don't open a public GitHub issue for a security vulnerability.**
A public issue discloses the problem to everyone, including anyone
running an unpatched copy, before a fix exists.

Instead, use GitHub's private vulnerability reporting:
1. Go to the repository's **Security** tab
2. Click **Report a vulnerability**
3. Describe the issue, ideally with steps to reproduce

This opens a private advisory visible only to the maintainer until a fix
is ready.

If you'd rather not use GitHub for this, opening a regular issue that
says only "I have a security issue to report, please contact me" (with
no details) is a reasonable fallback. It flags the need without
disclosing anything publicly.

## What to expect

This is maintained in whatever time is available outside other
commitments, not on an SLA. Best-effort, not guaranteed:
- Acknowledgement of a report: aim for a few days
- A fix or mitigation: depends entirely on severity and complexity.
  Could be same-day for something simple and serious, could be longer
  for something subtle

## Scope

This policy covers the Claunote application code itself (this
repository): the Rust backend, the frontend, the Dockerfile, and the CI
workflow. It does not cover:
- Vulnerabilities in dependencies themselves (report those upstream, to
  the dependency's own maintainers or via [RustSec](https://rustsec.org/))
- Your own deployment environment (reverse proxy config, TLS setup,
  network exposure, host OS); those are outside this project's control

## What's already in place

For context on the existing security posture (so a report can focus on
what's actually new), see the **Security notes** section in
[README.md](./README.md). Briefly: Argon2id password hashing,
server-side session tokens, per-IP login rate limiting, allow-list
filename sanitization against path traversal, a non-root container user,
OS packages patched at every build plus a weekly scheduled rebuild, and
a capped request body size.
