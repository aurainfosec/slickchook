# Changelog — 2026-10-15

- Updated `SC-019` in `SECURITY_CONSIDERATIONS.md` to cover a fourth
  `rmcp` Dependabot alert that appeared this week:
  [GHSA-c9xm-49cp-xcr9](https://github.com/modelcontextprotocol/rust-sdk/security/advisories/GHSA-c9xm-49cp-xcr9)
  (Medium, no CVE assigned yet) - `rmcp`'s OAuth client fetches a
  `resource_metadata` URL taken directly from a server-controlled
  `WWW-Authenticate` header with no same-origin or private-network
  check, an SSRF primitive a malicious MCP server could use against
  whatever network the client runs on.
  Confirmed not applicable for the same reason as the other three:
  checked directly in the pinned `rmcp 0.5.0` source
  (`src/transport.rs`) that `pub mod auth;` - the module containing
  both this bug and the already-reviewed resource-field-validation
  flaw - sits behind a literal `#[cfg(feature = "auth")]`, a Cargo
  feature Slickchook's own `Cargo.toml` never enables (only
  `server`/`transport-sse-server` are on). That code isn't compiled
  into this binary at all, and - independent of feature-gating -
  Slickchook never acts as an OAuth client in the first place, which
  this specific flaw requires.
  No code change - this is a review finding, not a fix. Same
  re-verification trigger as the rest of the entry: re-check if
  `rmcp` is ever upgraded to enable `auth`/`client`/the streamable-http
  transport features.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean.
