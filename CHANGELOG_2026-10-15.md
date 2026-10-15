# Changelog — 2026-10-15

- Added a new `ROADMAP.md` section, "Prompt-injection resilience and
  testing", per direct request. Covers two related but distinct
  directions:
  - Defensive: extending hidden/encoded-instruction detection beyond
    screenshots (the existing `screensanitize`/SC-017 pairing) to
    arbitrary text/HTML/JSON content any fetcher or crawler plugin
    returns, plus other content-based injection surfaces a target can
    serve back at the reading AI client (HTTP headers, DNS TXT
    records, file contents, zero-width Unicode, homoglyphs, RTL
    override tricks) - a text-based analog of what `screensanitize`
    already does for images.
  - Offensive/testing: a dedicated capability for testing whether a
    target's own AI/LLM-integrated feature (a chatbot, support agent,
    AI-powered search, etc.) is itself vulnerable to prompt injection -
    an authorized red-team technique distinct from everything this
    fleet currently runs, and from the defensive items above.
  Also added a regression-test item (untrusted asset content must
  never gain tool-calling authority regardless of which plugin or code
  path it arrived through) to make the defensive direction verifiable,
  not just aspirational.
  Bumped the roadmap's own "Updated" date. Documentation-only change,
  no code - `ROADMAP.md` isn't compiled or exercised by any test.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean.
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
- Added a `LICENSE` file (GNU General Public License v3.0 or later,
  full canonical FSF text), per direct request. No license file or
  mention existed anywhere in the repo before this - confirmed by
  checking for an existing `LICENSE`, and for any license references
  in `README.md`/`Cargo.toml`, before adding one.
  Used the "or later" variant (`GPL-3.0-or-later` in `Cargo.toml`'s new
  `license` field) rather than a strict GPL-3.0-only - the FSF's own
  recommended default for a new project, and the more common choice in
  practice - since the request didn't specify "only"; flagged here so
  this is easy to correct if a strict version-3-only license was
  actually intended.
  Added a short "## License" section to `README.md` pointing at the
  new file, and noting explicitly that this license covers this
  repository's own code and plugin manifests - not the third-party
  tools each plugin wraps or references (nmap, testssl.sh, Nuclei, and
  the rest), which each keep their own separate licenses.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean - `Cargo.lock` needed no changes, since a
  package's own `license` field is metadata, not a dependency
  resolution input.
- Documentation cohesion pass across the core (non-changelog) docs, per
  direct request, following a read-only survey that flagged five
  concrete issues:
  - `ROADMAP.md`'s "Static web-content analysis" section had four
    unchecked items (authorized URL fetcher, read-only `/input` mount,
    network-disabled/non-root analyzers, never executing downloaded
    JS/HTML) that are all already shipped - checked each one off with a
    one-line pointer to where it's actually implemented/enforced
    (`webfetch` in `PLUGINS.md`, `input_read_only` in `RUNNER.md`, the
    fleet-wide container policy in `SECURITY_CONSIDERATIONS.md`).
  - Moved eight misfiled items (JSluice/LinkFinder, SecretFinder,
    source-map discovery, asset/response-size limits, staged-asset
    cleanup tests, the fetch-to-analyzer `input_run_id` refactor,
    input-run rejection rules, end-to-end fetcher/analyzer integration
    tests) out of "Multi-client MCP deployment" and into "Static
    web-content analysis," where they actually belong - none of them
    are session/OAuth/transport work.
  - `ARCHITECTURE.md`'s recon-workflow diagrams (both the detailed
    phase diagram and the high-altitude convergence diagram) stopped at
    §5 and didn't reflect `SKILL.md`'s §6 ("Building a visual
    attack-surface map," added earlier this week) - added a matching §6
    block to both diagrams.
  - `SECURITY_CONSIDERATIONS.md`'s "Last reviewed" header was dated
    2026-08-25 despite SC-019 being updated as recently as today -
    bumped it to today's date.
  - `README.md`'s "MCP surface" bullet listed every tool except
    `purge_runs_by_target`, which `SKILL.md` and `CLIENT_SETUP.md` both
    document as a real, callable tool - added it.
  No code changes; `ROADMAP.md`/`ARCHITECTURE.md`/
  `SECURITY_CONSIDERATIONS.md`/`README.md` aren't compiled or exercised
  by any test.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean.
- Fixed a real, applicable `rustls` Dependabot alert
  ([GHSA-2mjx-qc3c-rqvc](https://github.com/rustls/rustls/security/advisories/GHSA-2mjx-qc3c-rqvc),
  Medium, no CVE yet): `rustls` 0.23.13-0.23.44 accept a TLS 1.3
  handshake message packed into the same record as a preceding
  key-changing message instead of rejecting it per RFC 8446 §5.1.
  `Cargo.lock` had resolved to the vulnerable `0.23.42`, and - unlike
  the four `rmcp` alerts closed out in SC-019 - this one actually
  applies: `tokio-postgres-rustls` uses `rustls` as a TLS client for
  the PostgreSQL connection when `database_tls = "require"`, exactly
  the role the advisory describes.
  Fixed by `cargo update -p rustls`, resolving to `0.23.46`
  (`rustls-webpki` moved to `0.103.15` alongside it) - both already
  satisfy the existing `Cargo.toml` constraint, so no manifest edit was
  needed. Added `SC-020` documenting the alert, why it applies, and the
  fix.
  Also added an "Intended network environment" note to `README.md`'s
  security-model section, per direct request: Slickchook, its paired
  MCP client, and its PostgreSQL backend are intended to run on a
  private network segment (behind NAT, a private LAN/VPN, or
  equivalent) reachable only by each other, never on the public
  internet or a network where an on-path party could sit between them
  - the precondition several controls already relied on implicitly
  (SC-014's per-instance pinned certificate, SC-016's DNS-rebinding
  mitigation, and now SC-020's on-path caveat) but that was never
  stated as an explicit, up-front deployment assumption before.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean against the updated lockfile.
