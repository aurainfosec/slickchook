# Changelog — 2026-09-09

- Added the `ike-scan` plugin, wrapping Debian bookworm's `ike-scan`
  package (1.9.5-1+b1, confirmed matching royhills/ike-scan's real latest
  git tag exactly - not stale) to discover and fingerprint IKE
  (ISAKMP/IPsec VPN) endpoints: Main Mode probing by default, optional
  Aggressive Mode, backoff-pattern implementation fingerprinting
  (`showbackoff`), and Aggressive Mode pre-shared-key hash capture
  (`pskcrack`) for offline cracking with the separately-packaged
  `psk-crack` utility (not yet built as its own plugin - see below).
  Read ike-scan's real C source directly (royhills/ike-scan) rather than
  assume from its man page: confirmed the default Main/Aggressive Mode
  path uses a plain UDP socket, not a raw socket (raw sockets are only
  used for a separate source-IP-spoofing feature this plugin doesn't
  expose) - so the only real elevated-privilege need is binding to the
  default UDP source port 500, a privileged port below 1024. This fleet's
  `cap_add` allowlist only supports `NET_RAW`, which doesn't help here,
  so `run_as_root = true` is set as a deliberate, scoped exception -
  matching the exact same kind of exception nmap's own manifest already
  documents, for a different underlying reason (nmap's raw-packet
  techniques need literal EUID 0; ike-scan just needs to bind a low
  port).
  Documented the well-known real Aggressive-Mode PSK-hash-disclosure
  technique (`aggressive` + `id` + `pskcrack`) as meaningfully more
  sensitive than a passive fingerprint - deliberately eliciting crackable
  key material from the target - needing the same authorization
  awareness §0 already requires for other higher-sensitivity techniques,
  even though no attack payload is sent. Also documented `ikev2` as
  upstream's own stated "experimental, not extensively tested, only the
  default proposal" limitation, and that `vendor`/`trans` can each only
  be supplied once per call through this schema even though ike-scan
  itself supports repeating either flag.
  No Docker build/live verification yet - this sandbox has no `docker`
  socket access, matching how every other plugin this session has been
  verified: build/install/live-test happens after merge, on the real
  deployed host.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12, including the existing `scan_plugins`-based manifest
  scan which parses every plugin directory) clean.

- Added the `psk-crack` plugin, wrapping the real `psk-crack` binary
  bundled in the same Debian `ike-scan` package, completing the
  ike-scan-to-offline-crack workflow the `ike-scan` plugin's own
  `pskcrack` option sets up. Purely local, offline computation with no
  network activity of its own - `network = "none"` in its docker policy,
  the safest resource category this fleet has, needing no elevated
  privilege or capability at all (unlike `ike-scan` itself).
  Read psk-crack's own C source directly rather than trust its man page
  alone: the man page's synopsis shows `< psk-parameters-file>` as if
  the PSK data can only arrive via shell stdin redirection, but the real
  source (`psk-crack.c`) confirms it actually takes that file as a
  genuine positional argv argument, read via a plain `fopen()` (with
  `-` specifically meaning stdin). Since this fleet's plugin schema has
  no way to hand a container an inline multi-line blob except as a CLI
  option value, a thin Python wrapper (`entrypoint.py`) takes that blob
  as its own `--slickchook-psk-params` option (verified locally,
  independent of docker, against a stand-in binary: correctly extracts
  that option regardless of its position among the other real
  passthrough flags, writes it verbatim newline-terminated to a file,
  and forwards everything else unchanged), then execs the real
  `psk-crack` with that file as its positional argument.
  Also confirmed from source a real, easy-to-misread gotcha: psk-crack's
  exit code is always 0 whether or not a key was actually cracked - a
  successful crack and a clean miss both exit cleanly, so the manifest
  documents that the result must be read from the printed text itself,
  not `exit_code`.
  Bounded `bruteforce` mode's real unbounded-runtime risk (runtime grows
  exponentially with length) via this plugin's own `timeout_seconds`,
  documented explicitly so a deliberately large brute-force attempt is
  cut off rather than left running indefinitely.
  No Docker build/live verification yet, for the same reason as
  `ike-scan` above - this sandbox has no `docker` socket access.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean.

- Proposed `PLUGIN_WORKFLOW.md`, an end-to-end outline of the
  plugin-building workflow this project has actually used throughout its
  development: stable baseline -> clone/branch -> implement and verify
  locally -> PR -> update local repo and redeploy -> test from a real,
  containerized MCP client connecting over TLS (distinct from the
  loopback diagnostic shortcuts used for fast iteration) -> iterate.

- Extended `PLUGIN_WORKFLOW.md` with an explicit note, per direct
  request: `SKILL.md` (and documentation generally) should stay
  plugin-agnostic, describing methodology by capability role rather than
  hardcoding specific plugin names. This isn't just a style preference -
  the MCP client is meant to enumerate what's actually available via
  `list_plugins`, reading each plugin's own always-current manifest
  description directly, rather than falling back on a separate
  written-down reference to a named plugin that can silently drift out
  of sync as plugins are added, renamed, or changed. Self-documenting,
  enumerable plugins are the point of the design; fixed prose
  documentation that re-describes individual plugins would undermine
  that.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12, docs-only change) clean.

- Live-verified `ike-scan` and `psk-crack` after redeploy (both were
  merged with Docker build/live verification still pending, noted in
  their own entries above - this is that follow-up, not an edit to
  those entries). `ike-scan`: a real Main Mode probe against
  `scanme.nmap.org` completed cleanly (`exit_code: 0`), confirming
  `run_as_root` correctly allows binding to the privileged source port
  500, with an honest, expected negative result (no IKE responder on
  that particular test target). `psk-crack`: a genuine cryptographic
  correctness check, not just a smoke test - independently computed a
  real IKE Aggressive-Mode PSK hash in Python (following the exact RFC
  2409 HMAC-SHA1 SKEYID/HASH_R algorithm confirmed from `psk-crack.c`'s
  own source) using a real word from the bundled dictionary as the
  password, fed the resulting 9-field colon-separated hex line in as
  `psk_params`, and the plugin correctly cracked it - confirming the
  wrapper's arg extraction, the temp-file handoff, and the real
  binary's own hash computation all work correctly end to end.

- Added the `cmseek` plugin, wrapping `Tuhinshubhra/CMSeeK` to identify
  which CMS (180+ signatures, all bundled directly in the tool's own
  source - no separate database to fetch) an authorized web target runs,
  with deeper version/plugin/theme enumeration for the major platforms
  it supports (WordPress, Joomla, Drupal, Magento, and others).
  CMSeeK has no meaningful version tags - its latest git tag (v.1.1.3)
  is frozen at a 2020 commit even though master keeps receiving real,
  ongoing community fixes and new CMS signatures (confirmed live via its
  own commit history, most recently July 2026) - pinned to that exact
  current commit (03a6e7f8) instead of the stale tag.
  Read the tool's real source directly before designing the manifest,
  rather than trust its README: confirmed its single-target `-u`
  invocation (used here) never hits any of its several interactive
  `input()` prompts, which only exist on its multi-site/menu-driven
  paths this plugin deliberately doesn't expose - except for two real
  edge cases that `-u` alone doesn't avoid: a fresh results directory on
  first run, and its own default (unset) redirect-handling behavior,
  which normally prompts interactively on any mid-scan redirect. Both
  are confirmed suppressed by unconditionally setting `--batch` in
  `fixed_args` - the redirect case specifically auto-defaults to
  "follow it" under batch mode with no extra flag needed, so only
  no_redirect (the sole way to select "never follow a redirect") is
  exposed as its own option, not a --follow-redirect flag as well, which
  would have been redundant.
  Also confirmed from source (cmseekdb/basic.py) a genuine
  read-only-filesystem gotcha, the same class already seen in several
  other plugins this project has built: CMSeeK writes results to its
  own working directory, using its installed directory only if that
  happens to be writable and falling back to the process's actual
  current working directory otherwise. Fixed with a second `WORKDIR
  /artifacts` in the Dockerfile (after the one used to `pip install` its
  requirements from its own installed directory) - this makes the tool
  silently take the correct, writable fallback path with no prompt of
  any kind, rather than needing a custom wrapper script.
  No Docker build/live verification yet - this sandbox has no `docker`
  socket access, matching how every other plugin this session has been
  verified: build/install/live-test happens after merge, on the real
  deployed host.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean.

- Live-verified `cmseek` after redeploy (merged with Docker build/live
  verification still pending, noted in its own entry above - this is
  that follow-up, not an edit to that entry) against two pre-authorized
  public test targets: `scanme.nmap.org` (real HTTP fetch, real
  signature-matching pipeline, honest true-negative "Unable to detect
  CMS" result - that host doesn't run one) and `testphp.vulnweb.com`
  (the HTTP request itself timed out, most likely that target being
  slow/unreachable at the moment rather than a plugin bug, since the
  other target connected fine on the same deployment around the same
  time). Both runs completed with `exit_code: 0` and correctly did not
  hang on the "Result directory does not exist" first-run message. The
  positive-detection path (a real CMS actually getting matched) hasn't
  been exercised yet - that needs an explicitly authorized target that
  actually runs one.

- Added `CLIENT_SETUP.md`, covering the client side of a deployment: MCP
  connection setup (hostname resolution via `/etc/hosts`/DNS since Caddy
  keys its certificate to `CADDY_HOSTNAME`, not a bare IP; TLS trust,
  following `RUNNER.md`'s existing recommendation of provisioning-time
  certificate/fingerprint pinning over trust-on-first-connection, with
  TOFU documented only as an explicitly weaker fallback and the same
  rationale RUNNER.md already gives), loading `SKILL.md` into a client's
  own context (confirmed from source there's no MCP resource/prompt
  endpoint serving it - it's a plain repo file, loaded once per client
  through whatever mechanism that client offers for standing
  instructions), refreshing a client's cached plugin list after a
  redeploy (confirmed from source there's no `tools/list_changed`
  notification - an existing session needs a full reconnect, not a
  partial refresh, to see a newly-installed or changed plugin), and five
  common end-of-engagement tasks built on the existing tool surface:
  reviewing/purging prior run history for a target
  (`list_runs_by_target`/`purge_runs_by_target`), a timeline
  (`list_runs_by_target`'s own already-sorted, timestamped output),
  a recon/attack-surface report (an explicitly client-side synthesis
  task over `list_runs_by_target` + `read_artifact`, not a built-in
  export - matching `SKILL.md`'s own §4), an importable Burp Suite URL
  list (pointing at `RUNNER.md`'s existing import-mechanics section
  rather than duplicating it), and the full raw artifact archive
  (`list_run_artifacts` + `read_artifact` across every run, matching
  §5). Added a one-line pointer to it from `README.md`, next to the
  existing `SKILL.md` reference.
  Documentation-only change, no code/behavior impact.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean.
