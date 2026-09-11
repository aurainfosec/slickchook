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
