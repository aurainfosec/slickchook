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

- Added the `sippts-scan` plugin, wrapping the `scan` module of
  `Pepelux/sippts` (pinned to its real latest tag, v4.1.2 - actively
  maintained, most recently pushed June 2026) to discover and
  fingerprint SIP (VoIP) services on an authorized target, including
  matching a detected implementation against sippts's own bundled CVE
  reference data. Chose this over the unofficial PyPI package named
  `sippts` (confirmed live it's an unrelated third-party fork, "Forked
  SIPPTS for PyPi Publishing", version 0.1.0, one release only - not the
  real project) and built from the real upstream source instead.
  Read the real source directly rather than assume from its docs:
  confirmed `sipscan.py` (the module behind the `scan` subcommand
  exposed here) only uses plain UDP/TCP/TLS sockets, never a raw socket
  - so unlike `ike-scan`, this plugin needs no elevated privilege or
  capability at all.
  Also caught a real upstream packaging bug before ever building the
  image: this tag's `pyproject.toml` declares its dependencies
  dynamically from a `requirements.txt` file that doesn't actually exist
  anywhere in the real repo contents at this tag (confirmed by listing
  them directly) - `pip install .` would fail outright on this alone.
  Worked around it by writing that file ourselves in the Dockerfile,
  matching `setup.py`'s own real `install_requires` list exactly, so the
  install can actually resolve its metadata.
  Deliberately exposes only the `scan` subcommand of sippts's much
  larger multi-tool suite (~20 subcommands) - scoped to discovery and
  fingerprinting for this first plugin. Explicitly excluded from any
  future plugin covering this project: `flood` (an actual SIP flood/DoS
  tool) and `spoof` (ARP spoofing) - both are destructive techniques
  outside what this fleet builds, not just deprioritized. `method`
  (options/register/invite) is documented as a real intrusiveness
  choice, not just a technical detail, since `invite` can cause a real
  target phone to ring.
  No Docker build/live verification yet - this sandbox has no `docker`
  socket access, matching how every other plugin this session has been
  verified: build/install/live-test happens after merge, on the real
  deployed host.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean.

- Fixed a real `sippts-scan` build failure found on its first actual
  `slickchook-plugin install` after merge: `netifaces` (pulled in
  transitively - `sipscan.py` itself imports it via sippts's own
  `lib/functions.py`) has no prebuilt wheel for this Python/platform
  combination, and needs to compile a small C extension from source,
  which failed outright with `error: [Errno 2] No such file or
  directory: 'gcc'` - the `python:3.13-slim` base image has no C
  compiler installed at all. Fixed by adding `build-essential` to the
  image's apt install step. This is exactly the kind of thing that
  can't be caught without a real Docker build - `cargo test`'s manifest
  scan only validates the TOML structure, not that the referenced
  Dockerfile actually builds.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12, unaffected - Dockerfile-only change) clean.

- Added the `sippts-dcrack` plugin, wrapping sippts's real `dcrack`
  module to crack SIP Digest authentication hashes offline (the kind
  the not-yet-built `sippts-leak` plugin's SIP Digest Leak technique
  will produce, or any other capture in the classic SipCrack format).
  Purely local computation - `network = "none"`, the safest category
  this fleet has.
  Read `sipdigestcrack.py`'s own source directly rather than guess the
  input format: confirmed the real file layout (12 double-quote-
  separated fields per line: source/dest IP, username, realm, method,
  URI, nonce, cnonce, nc, qop, algorithm, response) and reused the same
  wrapper pattern as `psk-crack` - a thin Python entrypoint accepts the
  hash line(s) as an inline `--slickchook-hashes` option (verified
  locally against a stand-in binary: correctly extracts it regardless
  of position, writes it to a file, and forwards everything else
  unchanged), then execs the real `dcrack` with `-f` pointing at that
  file. Confirmed the same real, easy-to-misread gotcha as `psk-crack`:
  exit code is always 0 regardless of whether anything was cracked.
  Found a second, more serious real risk from source: dcrack's own
  bruteforce-mode defaults (charset=printable, a 94-character set, max
  length=8) combine to roughly 6*10^15 possible candidates - the
  manifest documents this loudly and relies on this plugin's own
  `timeout_seconds` to cut off a run that's too large, rather than
  overriding the tool's own defaults outright.
  Also found and fixed a real read-only-filesystem gotcha from source:
  dcrack writes a resume-cache file (`sipdigestcrack.res`) to its own
  current working directory on every run - meaningless in this fleet's
  fresh-container-per-run model, but needing a writable CWD regardless
  to avoid crashing. Fixed with `WORKDIR /tmp` in the Dockerfile, which
  also keeps this leftover file out of `/artifacts` entirely.
  Wordlist mode needs a file already present under the run's own
  `/input` directory, populated by chaining `input_run_id` from a prior
  wordlist-generating plugin's run - the same chaining mechanism already
  used elsewhere in this fleet, documented rather than reinvented with
  a bundled static wordlist.
  No Docker build/live verification yet - this sandbox has no `docker`
  socket access; the wrapper's own argument-extraction/file-writing
  logic was verified locally against a stand-in binary, independent of
  docker, matching how `psk-crack` was verified before its own first
  real build.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean.

- Found and fixed a genuine upstream bug in `sippts-dcrack`'s bruteforce
  mode, caught during its first real live-verification attempt: an
  independently-computed, correct SIP Digest hash (following the exact
  RFC 2409/7616 HA1/HA2/response algorithm confirmed from `sippts`'s own
  `calculateHash()`) was correctly tried as a candidate - visible in the
  tool's own progress output - but still reported as "not found."
  Reproduced this locally, entirely independent of Docker, before
  assuming it was a bug in this plugin, our JSON/argv transport, or the
  wrapper: called `SipDigestCrack.crack()` directly with the exact known-
  correct arguments and confirmed it still returned empty. Traced it
  with a temporarily instrumented copy of the real source (replacing its
  bare `except: pass` with one that prints the real traceback) to
  `sipdigestcrack.py`'s own `save_data()` method, called immediately
  after a real match is found to record it: its bruteforce-mode format
  string is missing one `%s` placeholder relative to its 6-element
  argument tuple (its wordlist-mode sibling, two lines above in the same
  function, correctly has all six), raising a real `TypeError` that gets
  silently swallowed by `crack()`'s own bare `except`, so a genuinely
  successful crack reports as a clean miss every single time. Confirmed
  the fix by patching that exact line locally and re-running the same
  test - the same known-correct password was then correctly reported.
  Patched at build time (`sed` against the installed package file,
  targeting only the exact broken 5-placeholder string - its two
  already-correct 6-placeholder siblings are untouched) rather than
  shipping a bruteforce mode that could never actually report success.
  Documented the finding directly in the manifest, since without this
  fix bruteforce mode in the real upstream tool cannot ever report a
  successful crack, regardless of whether the password is actually
  right.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12, manifest/Dockerfile-only change) clean.

- Added the `sippts-leak` plugin, wrapping sippts's real `leak` module
  to exploit the well-known SIP Digest Leak vulnerability - completing
  the capture-then-offline-crack pairing `sippts-dcrack` already sets up
  (mirroring `ike-scan`'s `pskcrack` + `psk-crack` pairing). Confirmed
  from source (`sipdigestleak.py`) that this sends a genuine INVITE and
  a follow-up BYE to a real target - meaningfully more intrusive than a
  passive fingerprint, since a vulnerable phone can actually ring
  briefly - documented with the same authorization-awareness caution
  already given to `sippts-scan`'s `invite` method.
  Confirmed from source it needs no elevated privilege or capability at
  all, unlike `ike-scan`'s equivalent Aggressive-Mode PSK capture: it
  tries to bind local port 5060 first (matching real SIP client
  behavior) but automatically falls back to a free ephemeral port if
  that fails, and only ever uses a plain socket, never a raw one.
  Always writes any captured digest challenge to
  `/artifacts/results.txt` in the exact SipCrack format `sippts-dcrack`
  already expects, so its output can be read back and fed directly into
  that plugin's `hashes` option.
  Having just found a real format-string bug in the sibling `dcrack`
  module (see the entry above), proactively checked every `%`-format
  string in this module for the same placeholder-count-vs-argument-
  count mismatch before shipping, rather than wait to discover it during
  live verification - all of them (including the exact line that writes
  `/artifacts/results.txt`) check out correctly.
  `auth_user`/`auth_pass` (test-only credentials for phones that only
  leak on a second, authenticated round-trip) carry the same "test-only,
  retained in run history" caution this fleet's `medusa` plugin already
  documents.
  No Docker build/live verification yet - this sandbox has no `docker`
  socket access, matching how every other plugin this session has been
  verified: build/install/live-test happens after merge, on the real
  deployed host.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean.

- Added the `katana-headless` plugin, a companion to the existing
  `katana` plugin (which is deliberately scoped to non-headless HTTP
  crawling only) enabling Katana's real Chromium-based headless
  crawling engine - genuinely different functionality, not just a
  slower version of the same thing: a real browser executes each
  page's JavaScript, discovering endpoints/content that only exist
  after client-side rendering, and (with `xhr_extraction`) capturing
  the actual dynamic XHR/fetch calls a page makes at runtime - dynamic
  behavior neither the plain `katana` plugin nor `gowitness`'s
  screenshot capture can see.
  Resolved the two things the plain `katana` plugin's own description
  had explicitly flagged as unverified, by reading Katana's real source
  (`pkg/engine/hybrid/hybrid.go`) rather than its docs: (1) without an
  explicit system-chrome path, Katana's headless engine (go-rod) falls
  back to auto-downloading its own Chromium build from the network on
  first use - since every plugin run here is a fresh container, that
  would mean a real network download attempt on every single run, so
  `-system-chrome` is unconditionally set in `fixed_args`, forcing it to
  use the Chromium already baked into the official image (confirmed via
  its own real Dockerfile: Alpine's `apk add chromium`) via go-rod's own
  standard-path lookup instead; (2) Chrome's own internal process
  sandbox needs privileges this fleet's containers deliberately don't
  have - confirmed `-no-sandbox` maps directly to Chrome's real
  `--no-sandbox` flag and is required for headless mode to start at all
  here, so it's also unconditionally set. Also confirmed from source
  that Katana itself errors outright if `-no-sandbox`/`-system-chrome-path`
  are set without headless (or hybrid) mode also enabled - since headless
  mode is unconditionally set here too, this can never occur through
  this plugin.
  Deliberately does not expose `automatic_form_fill`: confirmed from
  Katana's own source comment that it is explicitly disabled/ignored
  for headless navigation upstream ("incorrect implementation... should
  be handled via headless actions within the page context") - a real,
  confirmed upstream limitation, not an oversight on this plugin's part.
  Sized `/dev/shm` well above Docker's small 64 MiB default in this
  plugin's own docker policy - a well-known real headless-Chrome crash
  cause (heavy inter-process shared-memory use between renderer
  processes) that this plugin has no other way to work around, since it
  doesn't expose Katana's raw `-headless-options` passthrough for a
  `--disable-dev-shm-usage`-style flag instead.
  `auto_login` performs a real, live authenticated action against the
  target - the same test-only, retained-in-run-history caution this
  fleet's `medusa` plugin already documents applies here too.
  No Docker build/live verification yet - this sandbox has no `docker`
  socket access, matching how every other plugin this session has been
  verified: build/install/live-test happens after merge, on the real
  deployed host.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean.

- Fixed a real bug in `katana-headless` found during its first real live
  test, caught only because it was actually run rather than trusting the
  source read alone: `-system-chrome` (the auto-detecting flag this
  plugin's own manifest previously relied on) turned out to only be
  honored by Katana's separate `hybrid` engine - pure headless mode uses
  a genuinely different code path (`pkg/engine/headless/browser`, not
  `pkg/engine/hybrid` where that logic actually lives) that silently
  ignores it. The live test showed exactly the failure this plugin was
  built to avoid: a real `chrome-linux.zip` download from Google's own
  CDN starting on every run, eating the crawl's own time budget and
  producing no actual crawl output at all in the run this surfaced in.
  Fixed by switching to `-system-chrome-path` with an explicit path to
  the real Alpine-packaged binary (`/usr/bin/chromium`, confirmed
  against Alpine 3.24's own chromium package file listing) - pure
  headless mode's code does correctly honor an explicit path, just not
  the auto-detecting flag. Updated the manifest description to reflect
  what was actually verified live, not the original (wrong) claim.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12, manifest-only change) clean.

- Found and fixed a second, deeper real bug in `katana-headless`,
  caught on the redeploy verifying the fix above: after the download
  issue was gone, a real crawl against `scanme.nmap.org` still produced
  zero output at all - `exit_code: 0`, completely empty stdout and
  stderr, even with verbose logging enabled. Root-caused this properly
  rather than guess, by bisecting one container restriction at a time
  via direct `docker run` on the deployed host: capabilities dropped or
  not, `no-new-privileges` on or off, read-only root filesystem on or
  off - every combination under this fleet's usual unprivileged run-as
  user (UID 65534, no matching `/etc/passwd` entry) failed exactly the
  same way, silently. Running the identical invocation as root produced
  a full, real crawl immediately (real page rendering, iframe/link
  extraction, hundreds of discovered URLs). This isolates the cause
  specifically to the unprivileged UID itself, not any capability or
  filesystem restriction this fleet's containers otherwise apply.
  Fixed with `run_as_root = true` in this plugin's docker policy - a
  deliberate, scoped exception, the same kind `nmap`'s own manifest
  already documents for a different underlying reason (nmap's raw-
  packet techniques need literal EUID 0; here it's Chrome/Chromium's
  own apparent need for a real, resolvable user identity to initialize
  successfully - setting `$HOME` alone, the fix already used for the
  plain `katana` plugin's own config-lookup issue, was not sufficient
  for this deeper one).
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12, manifest-only change) clean.
