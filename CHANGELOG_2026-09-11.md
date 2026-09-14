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

- Fixed `gowitness` silently producing zero screenshot artifacts, per a
  direct report. Reproduced live: `run_plugin` against
  `https://scanme.nmap.org` came back `status: success`, `exit_code:
  0`, but the only artifacts were `stdout.txt`/`stderr.txt` - no
  screenshot file at all, and stdout was completely empty (no
  `result 🤖` success line either, meaning the capture itself never
  completed). The only stderr line was an unrelated warning about no
  `--write-*` output writers being configured - a red herring: per
  gowitness's own source (`cmd/scan.go`'s help text and
  `pkg/runner/runner.go`), those writers control optional metadata
  persistence (db/csv/jsonl), not the screenshot file, which is written
  independently whenever a capture actually succeeds.
  The real problem: gowitness only logs a per-target capture failure
  (Chrome error, DNS failure, navigation timeout, etc.) when
  `--log-scan-errors` is set - confirmed directly from source
  (`runner.Run()`'s `if run.options.Logging.LogScanErrors { ...
  log.Error(...) }` guards on every failure path), and that flag was
  never being passed. Without it, a failed capture is completely
  silent: no stderr, no per-target log line, and the process still
  exits 0 - indistinguishable from "nothing to report" without the real
  error text. Added `--log-scan-errors` as an always-on `fixed_args`
  entry (never a reason to want failures hidden) and documented the
  gap directly in the manifest: a `success`/`exit_code: 0` result is
  not proof a screenshot was produced, the artifacts list must be
  checked for an actual image file, and stderr should be checked for
  the real reason before assuming a site is simply unreachable or
  blocking automation.
  This is a visibility fix, not yet a root-cause fix for why the actual
  capture is failing against this target - that still needs the real
  error text `--log-scan-errors` will now surface, from a redeploy and
  rerun.
  Manifest-only change, no Dockerfile/image change. Verified: `cargo
  build --locked --release --bins` and `cargo test --lib` (12/12)
  clean.
