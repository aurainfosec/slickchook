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
- Added a new `SKILL.md` §6, "Building a visual attack-surface map", per
  direct request, after two real proof-of-concept builds this session:
  one against scanme.nmap.org (Nmap's own public test target) and one
  against aurainfosec.com's real recorded run history (69 runs across
  17 plugins). Both were built entirely from `list_runs_by_target` /
  `list_run_artifacts` / `read_artifact` - the same calls §5 already
  documents - stitched into a domain → IP/subdomain → port/service →
  web-finding tree and published as an HTML Artifact with inline SVG,
  no new tool or code required.
  Codifies what both builds converged on as real, load-bearing
  practice, not just a one-off styling choice:
  - Four consistent node/edge categories - confirmed, tested-negative/
    historical, notable/lead, and flagged-needs-reverification - so the
    diagram distinguishes "checked and found nothing" from "never
    checked", and "unusual, worth a look" from "actually a problem".
  - Call out corroborated findings (multiple independent plugins
    landing on the same underlying fact - e.g. the aurainfosec.com
    build's Cloudflare Bot Management signal, confirmed separately by
    httpx, kiterunner, and gowitness) explicitly as one finding, since
    that's stronger evidence than any single tool's result and explains
    why other tools may have come back empty.
  - Always include an evidence table under the diagram mapping every
    node back to its source plugin/run_id/date - what makes this
    reusable as part of a real report rather than a one-off picture -
    plus a caveats note naming any tool that failed outright rather
    than silently treating failure as a clean result.
  Scoped as an on-request reporting output, not a default end-of-
  engagement deliverable, matching this section's existing archive/
  export outputs.
  Placed in this project's own `SKILL.md` (the recon methodology
  loaded into every MCP client per `CLIENT_SETUP.md`), not the
  platform's bundled artifact-design/artifact-diagramming skills -
  those are general design references this project doesn't own or
  edit; this is project-specific methodology.
  Documentation-only change, no code. `cargo test --lib` still 12/12
  (SKILL.md isn't compiled or exercised by any test).
