# Changelog — 2026-09-11

- Corrected the `testssl` plugin's protocol-support-table guidance
  (added 2026-09-10, merged in #119) after redeploying and live-testing
  it against a real target caught another real error: the manifest
  claimed each protocol finding is exactly `'offered'` or
  `'not offered'`, but a real scan's actual JSON showed `TLS1_1`
  (TLS 1.0's finding is the same) reads `'offered (deprecated)'` and
  `TLS1_3` reads `'offered with final'` when genuinely offered - not
  the bare word. Confirmed from source that `'not offered'` is always
  that exact literal string in the normal case, but `'offered'` is not
  - several protocols append detail onto it (a deprecated-protocol
  marker for TLS1/TLS1_1, TLS 1.3 draft/final detail, or vulnerability
  detail for SSLv2/SSLv3). The manifest's tick/cross rendering
  instructions (added the same day, for building a NIST SP 800-52 Rev.
  2 style pass/fail table) now check `finding == 'not offered'` rather
  than `finding == 'offered'`, so real-world offered findings with
  extra detail are still classified as offered rather than silently
  falling through as unmatched.
  Manifest-only change, no Dockerfile/image change. Verified: `cargo
  build --locked --release --bins` and `cargo test --lib` (12/12,
  including the existing `scans_testssl_plugin_manifest` test) clean.

- Documented the `testssl` plugin's origin-IP WAF/CDN bypass false
  negative, per direct request: if target resolves to a CDN/WAF (e.g.
  Cloudflare) rather than the real origin server, this plugin only
  ever tests the CDN/WAF's own TLS stack - a real weakness in the
  origin's actual TLS configuration is a silent false negative, not
  evidence the origin itself is secure. Added a new `origin_ip` option
  (testssl's own `--ip` flag) so the client can connect directly to a
  known origin IP while keeping `target` as the real hostname, which
  keeps SNI/Host set correctly ("SNI spoofing") and bypasses the
  CDN/WAF's routing entirely. Confirmed the mechanism directly from
  testssl.sh's own source before writing this: SNI is only cleared
  when the target itself is a bare IP with no hostname at all
  (`determine_ip_addresses()`'s `elif is_ipv4addr "$NODE"` branch,
  which sets `SNI=""`) - when `--ip`/`CMDLINE_IP` is combined with a
  real hostname target, that branch is never taken, so SNI/Host stay
  correct and the origin server serves its own real certificate and
  config to whoever connects directly to it. This plugin cannot
  discover a candidate origin IP itself - `origin_ip` only helps once
  one is already known from other recon.
  Manifest-only change, no Dockerfile/image change. Verified: `cargo
  build --locked --release --bins` and `cargo test --lib` (12/12,
  including the existing `scans_testssl_plugin_manifest` test) clean.

- Added guidance to `SKILL.md` §4, per direct request: when compiling
  the end-of-engagement summary, derive a tools/methodology-used list
  from `list_runs_by_target`'s distinct `plugin` values per in-scope
  target, rather than relying on the current conversation's own memory
  of what it invoked - this also correctly captures runs from an
  earlier session or a different conversation entirely. Kept
  plugin-agnostic per existing practice (no plugin names hardcoded);
  the exact tool used is just whatever `list_runs_by_target` reports.
  Documentation-only change, no code.
