# Changelog — 2026-09-21

- Added `SC-019` to `SECURITY_CONSIDERATIONS.md`, reviewing three new
  `rmcp` Dependabot alerts that appeared this week
  (GHSA-9pj6-vhgr-3mwh/CVE-2026-63128 session-table-leak DoS,
  GHSA-33f5-2c5q-wgwj/CVE-2026-63127 OAuth resource-metadata
  validation, GHSA-9g45-5xwm-f3wc/CVE-2026-64684 custom-header
  redirect leak) on top of the already-tracked SC-016. Confirmed
  directly against the pinned `rmcp 0.5.0` crate source
  (`~/.cargo/registry/.../rmcp-0.5.0`), not just the advisories' own
  text, that none apply to this binary: the vulnerable modules require
  Cargo features (`transport-streamable-http-server`, `auth`, `client`,
  `transport-streamable-http-client`) that Slickchook's own
  `Cargo.toml` never enables (only `server`/`transport-sse-server`
  are on), so that code isn't compiled in at all - and, applying the
  same rigor SC-016 used rather than trusting feature separation
  alone, confirmed the session-leak's specific bug pattern (create a
  session, leak it on an early-return before validation) doesn't
  independently exist in `sse_server.rs`'s own session handling either.
  No code change - this is a review finding, not a fix. Flagged for
  re-verification if `rmcp` is ever upgraded to enable any of those
  features (which the SC-016 migration to `StreamableHttpService`
  eventually will).
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean.
