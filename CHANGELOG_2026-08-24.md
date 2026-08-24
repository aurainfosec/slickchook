# Changelog — 2026-08-24

- Added a new `waf-fingerprint` plugin - identifies which WAF/CDN (if
  any) fronts an authorized target, via `wafw00f`
  (github.com/EnableSecurity/wafw00f). Per a direct request to first
  check how effective the tool actually is before building around it -
  researched this honestly rather than assuming the most famous name
  in this space is automatically the right choice:
  - Confirmed actively maintained (last pushed 2026-04-19, latest
    release v2.4.2 in January 2026 with new WAF signatures added as
    recently as that month) - not stagnant.
  - Confirmed a real, important fact that was not previously
    documented anywhere and changes how this plugin must be scoped:
    reading its own source directly shows it is NOT a passive
    fingerprint - alongside passive header comparisons, it sends
    genuinely attack-shaped payloads (an XSS string, a time-based
    blind SQLi string, an LFI path-traversal string, an XXE payload,
    and an OS-command-injection-looking string) as part of its
    detection technique. Nothing is actually exploited, but these will
    very plausibly appear in a target's own WAF/SIEM logs as attempted
    attacks - documented prominently in the manifest, including the
    exact payloads, not glossed over as "just a fingerprint."
  - Confirmed a genuinely mixed, honest effectiveness picture rather
    than presenting it as infallible: its own commit/issue history
    shows a real, ongoing pattern of detection gaps being found and
    fixed (a FortiGuard false-negative fix in January 2026, a Google
    Cloud Armor detection gap in 2024, historical misses on
    StackPath/Siteground/F5 BIG-IP ASM) - documented as a real
    limitation, not implied to work more reliably than it does.
  - Checked whether the already-installed template-based vulnerability
    scanner's own bundled WAF-detection capability makes this
    redundant before building it - confirmed it has a single-request,
    single-payload-type WAF-detection template, meaningfully narrower
    than wafw00f's own multi-payload-type, 173-signature approach
    (some WAFs only react to certain payload categories) - genuinely
    complementary, not redundant, and documented as such.
  - Confirmed the PyPI-published package version matches the latest
    GitHub release exactly (`2.4.2` on both) - a plain version pin is
    safe here, unlike several other tools this fleet has had to pin by
    commit instead due to a stale package index.
  Added a new, generically-worded §0 scope-discipline category to
  `SKILL.md` for this real pattern - "anything whose detection
  technique itself sends deliberately attack-shaped payloads" -
  distinct from the existing credentialed-action and
  internet-wide-database categories, since it's a genuinely different
  kind of risk (log/alert noise on the target, not credential misuse
  or cost) that could apply to a future plugin too, not just this one.
  Checked with `grep` - no plugin/tool names added.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean. No docker access in this sandbox to build/run
  the image locally; live verification pending install.

- Live-verified `waf-fingerprint` after install with two real targets:
  `scanme.nmap.org` correctly returned `detected: false` (a bare test
  page with no WAF); `www.cloudflare.com` correctly returned `detected:
  true, firewall: Cloudflare`, with `trigger_url` showing the exact
  attack-shaped payloads (XSS/SQLi/path-traversal) documented in the
  manifest, sent as real URL parameters. Exit code 0 on both, clean
  JSON output matching the design exactly.

- Added a new `email-security-posture` plugin - checks an authorized
  domain's SPF, DKIM, and DMARC DNS records for common
  misconfigurations, purely passive DNS lookups only, no contact with
  the domain's actual mail infrastructure. Closes one of two remaining
  gaps identified in this session's own "what's still missing"
  review.
  Verified real record formats directly via live DNS-over-HTTPS
  queries against real domains (google.com/microsoft.com/github.com)
  before writing detection logic, rather than assuming textbook
  formats: confirmed real SPF records (including one split across
  multiple concatenated TXT strings due to the 255-byte string limit -
  `b"".join(rdata.strings)` handles this correctly), real DMARC
  records with p=/pct=/rua=/ruf=/fo= fields, and a real, important
  DKIM nuance - Microsoft 365 tenants often publish DKIM as a CNAME to
  a `<selector>-<domain>._domainkey.<tenant>.onmicrosoft.com` target
  rather than a direct TXT record, confirmed live that a standard
  TXT-type resolution already follows this transparently with no
  special handling needed.
  DKIM selectors have no canonical discoverable location in DNS at
  all - checks a list of selectors publicly documented as defaults by
  widely-used providers (Google Workspace, Microsoft 365, SendGrid,
  Mailchimp/Mandrill, Constant Contact, ProtonMail) plus generic
  self-hosted conventions, honestly documented as a best-effort guess
  rather than an exhaustive/authoritative check - a `selectors` option
  lets a caller add a real known selector from an actual email's
  DKIM-Signature header.
  Directly exercised the SPF/DMARC analysis logic (not just read for
  correctness) against the real record text captured from those live
  DNS queries, plus a battery of edge cases: a bare `all` mechanism
  (implicit `+`, distinct from an explicit `+all`), multiple SPF
  records, a missing `all` mechanism entirely, and a DMARC record with
  `pct<100` and no `rua=` - all produced the expected findings.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean. No docker access in this sandbox to build/run
  the image locally; live verification through the actual MCP server
  pending install.

- Added a new `jwt-analyzer` plugin - decodes a JWT's header/claims and
  offline-tests it for common weaknesses, via `jwt_tool`
  (github.com/ticarpi/jwt_tool). Closes the second of two remaining
  gaps identified in this session's own "what's still missing"
  review: several already-installed plugins (`dehashed`,
  `sourcemap-scanner`, `trufflehog`) flag "found a JWT-looking string"
  as a raw regex match, but nothing previously decoded or analyzed one.
  Researched honestly rather than assuming this well-known name is
  automatically the right, current choice: confirmed it is NOT
  actively maintained in the strict sense (last real commit/release
  v2.3.0, 2025-05-01, over a year stale) - but still the standard,
  most-referenced tool for this with no better-maintained alternative
  found. No PyPI package exists for it at all under either plausible
  name - built from source, pinned to that exact release tag, since
  there's nothing to check for staleness against a package index this
  time (the tag itself already is the latest real release).
  Confirmed directly from its own source, not assumed, that it is
  fully offline/local when its `-t`/`-ju` flags are omitted - grepped
  every network-call site in the file to confirm those two flags are
  the *only* ones that trigger any request at all - neither is exposed
  here, so `network = "none"` and this plugin can never reach a live
  target regardless of input.
  Also confirmed from source that its general interactive
  tamper/claim-injection modes (`-T`/`-I`) are built entirely around
  live `input()` prompts - dozens of them - which would simply fail in
  this fleet's non-interactive containers; deliberately not exposed.
  Its dedicated, non-interactive exploit/crack/sign flags (`-X`/`-C`/
  `-S`) cover the same practical ground (alg:none, null signature,
  psychic-signature CVE-2022-21449, blank-password, inline-JWKS
  injection, key-confusion, HMAC dictionary cracking, re-signing) and
  were confirmed clean of any interactive prompts by the same source
  read, not assumed safe by association.
  Uses jwt_tool's own real bundled wordlist of common/weak JWT signing
  secrets by default for cracking, matching this fleet's established
  preference for a tool's own real data over an invented list.
  This plugin never submits a crafted token anywhere itself - it only
  ever produces output (decoded claims, a crafted tampered token, a
  cracked secret); using one against a live target is left as a
  separate, deliberate follow-up call needing the same authorization
  as any other active test.
  Added a short, plugin-agnostic paragraph to `SKILL.md`'s Phase 4
  describing this "found a JWT, now analyze it" follow-up generically,
  alongside the existing dependency-manifest/CVE-lookup/source-map
  follow-ups already there - checked with `grep`, no plugin/tool names
  added.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean. No docker access in this sandbox to build/run
  the image locally; live verification pending install.

- Live-verified `email-security-posture` after install against
  `github.com`: SPF/DMARC output matched exactly what had already been
  confirmed via direct DNS-over-HTTPS queries during development, and
  the DKIM selector check turned up real hits across six different
  selectors at once (`google`, `selector1`, `k1`, `k2`, `s1`, `s2`) -
  an unexpectedly strong real-world confirmation that the curated
  common-selector list has genuine practical value, not just
  theoretical plausibility. Exit code 0, `findings.json` written
  correctly.

- Fixed two real bugs in `jwt-analyzer` found on its very first live
  test - a plain decode of the standard jwt.io example token returned
  completely empty output (empty stdout, empty stderr, `exit_code`
  1 - jwt_tool exits 1 unconditionally on every invocation by design,
  confirmed from its own source, so that part alone wasn't the
  problem):
  1. `jwt_tool`'s own `-b`/`--bare` flag - hardcoded into `fixed_args`
     for "cleaner output" - actually suppresses nearly *all* of its
     informational output, not just the decorative banner its own
     `--help` text implies. Confirmed directly from source: almost
     every print in the tool routes through one helper function that
     `-b` disables outright, including the decoded-claims printout
     itself - the one thing this plugin exists to produce. Removed
     from `fixed_args` entirely.
  2. While tracing that bug, also found and preemptively fixed a
     second, related one before it could bite live: jwt_tool's
     key-confusion exploit (`-X k`) and its RSA/EC/PSS signing modes
     both call Python's `open()` directly on the given key argument -
     meaning `pubkey`/`privkey` need a real file already present on
     disk, not key content passed as a plain value, despite the
     tool's own `--help` text describing them as "Public/Private Key"
     content generically. This fleet's per-call options schema has no
     way to supply an arbitrary file per invocation, so `pubkey`,
     `privkey`, the `k` exploit choice, and the rs*/es*/ps* `sign`
     choices are all removed - narrowing the manifest to exactly what
     was confirmed (by reading every relevant function's actual
     source, not just its own docs) to work with plain string/no-file
     inputs: exploit choices a/n/p/b/i (the last confirmed to
     self-provision its own demo signing key into the already-writable
     `$HOME` on first use, not requiring anything from the caller),
     `crack`/`password`, and `sign` limited to hs256/hs384/hs512.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean. Live re-verification (a real decode actually
  producing output, plus an exploit/crack path) pending redeploy.

- Fixed a real, blocking `jwt-analyzer` bug found on the redeploy after
  the `-b` fix above: still zero decoded output, this time stopping
  right after "Configuration file built - review contents of
  jwtconf.ini..." with no error anywhere. Traced this to a genuine
  first-run quirk in jwt_tool itself: on its very first invocation
  (no `jwtconf.ini` present yet under `~/.jwt_tool`), it writes a
  default config plus demo RSA/EC keypairs and then unconditionally
  `exit(1)`s *before ever touching the given JWT at all* - it expects
  a *second*, separate invocation to actually do anything. Since every
  plugin run in this fleet is a fresh, ephemeral container, every
  single call was hitting this exact "first run" path and none could
  ever reach the real decode logic - a fundamentally blocking bug, not
  a cosmetic one.
  Fixed by pointing `HOME` at a directory baked into the image itself
  (`/opt/jwt_tool_home`) instead of `/tmp` (a tmpfs mount that starts
  empty on every container run, so anything written there at build
  time would never survive to runtime regardless), and triggering the
  one-time config setup once, at image build time, so every real
  invocation afterward finds an existing config and skips straight
  past it - confirmed from source that the version-matching config
  reload path (same script, same pinned commit both writing and later
  reading it) has no drift risk here. `chmod -R 0777` afterward, since
  jwt_tool still appends to a log file and writes a marker file under
  that same directory on every run regardless.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean. Live re-verification pending redeploy - this
  is the third real bug found in this one plugin through live testing
  alone, none of which were visible from reading the tool's own
  `--help` text or its README.

- Fixed a fourth real `jwt-analyzer` bug, caught by comparing a direct
  `docker run` (root, no `--read-only`) against the actual redeployed
  behavior through slickchook itself, which was still silently
  stopping in the exact same place: the direct run decoded the token
  perfectly, confirming the `-b`/config-bootstrap fixes above were
  both correct - the remaining difference was that this fleet's
  containers always run `--read-only` as an unprivileged UID, which a
  bare `docker run` doesn't apply. jwt_tool writes a log line and a
  marker file under `~/.jwt_tool` on *every* invocation, not just the
  first - so even with a valid config baked into the image, that
  directory being read-only at runtime made every single run fail
  silently the same way the original first-run bug did, just for a
  different underlying reason.
  Fixed with a small wrapper entrypoint (`entrypoint.sh`) that copies
  the read-only, build-time-baked `~/.jwt_tool` template into the
  writable tmpfs `/tmp` on every run and points `HOME` there, rather
  than trying to make the baked-in copy itself writable (`chmod -R
  0777` on a read-only-mounted path does nothing - permission bits
  don't matter when the mount itself refuses writes). `chmod -R a+rX`
  on the baked-in source directory instead, so the unprivileged
  runtime UID can read and copy it even though it doesn't own it.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean. This is the fourth real bug found in this one
  plugin through live testing alone - live re-verification pending
  redeploy.

- Reviewed `jwt-analyzer` against every documented best practice in
  `PLUGINS.md`, per a direct request, after its four rounds of live
  bug-fixing above - confirmed it's clean against every existing
  checklist item (no `[[secrets]]`/`[[assets]]` misuse, no unneeded
  `run_as_root`, no privileged capabilities, correct `network = "none"`
  claim, each fix a new changelog entry rather than a silent edit).
  The one gap: the actual fix that worked (bake the tool's required
  first-run state into the image at build time, *then* copy that
  baked-in template into the writable tmpfs at every runtime start) is
  a genuinely new variant of the read-only-filesystem lesson that the
  existing two documented options (plain `ENV HOME=/tmp`, or mount a
  tool's specific subpath) don't cover on their own - added as a new
  checklist entry so a future plugin author facing the same "tool
  needs a second invocation to do real work" shape doesn't have to
  rediscover it live. Also notes, as part of that same new entry, a
  real methodological lesson from this exact debugging: the first
  three of those four bugs all "passed" when checked with a bare
  `docker run` on the deployment host, because that runs as root with
  a normal writable filesystem - no `--read-only`, no unprivileged UID
  - masking the actual runtime failure entirely; only a real
  `run_plugin`/`start_plugin` call through Slickchook itself exercises
  the real policy. Also added a short clarifying sentence to the
  existing `wpscan`/`shodan-*` "thin wrapper, real tool" paragraph
  noting `jwt-analyzer` uses the same kind of wrapper for a different
  reason (filesystem bootstrapping, not secret translation), so a
  future reader doesn't assume every such wrapper exists only for
  credential injection.
  Documentation-only change, no build/test impact beyond confirming
  nothing else broke.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean.

- Added a new `theharvester` plugin - aggregates emails/subdomains/
  names/IPs/URLs for an authorized domain from many passive sources at
  once, via theHarvester (github.com/laramies/theHarvester). Built per
  a direct request specifically requiring artifact retention and
  credential reuse across plugins, not just a basic wrap.
  Confirmed actively maintained (real commits the same day this was
  written) before building - pinned to the real latest release
  (`4.11.1`, unprefixed tag) rather than PyPI's own drastically stale
  published package (`0.0.1`).
  Always writes a JSON and an XML export under `/artifacts` via a
  fixed `-f` flag the caller can't override, satisfying the artifact-
  retention requirement directly rather than leaving results only in
  console output.
  Confirmed directly from source, not assumed, that theHarvester reads
  API keys *only* from its own local YAML config file - zero
  environment-variable support anywhere in its credential-loading path
  (the same gap `waymore` has, noted at the time but never wired up -
  wired up here since credential reuse was explicitly requested this
  time). Fetched the real `api-keys.yaml` template at the pinned tag to
  confirm the exact nested structure before writing a generator for
  it, then tested that generator directly (including a value containing
  a literal double-quote, to confirm escaping) by round-tripping the
  output back through a real YAML parser, not just eyeballing the text.
  Declares 38 `[[secrets]]` total: 5 deliberately reuse the exact same
  `env_var` name as an already-installed plugin covering the same
  underlying provider (`SHODAN_API_KEY`, `VT_API_KEY`, `GH_TOKEN`,
  `BRAVE_API_KEY`, `DEHASHED_API_KEY`) - configuring any of those once
  already makes it available here too - and 33 new
  `THEHARVESTER_*`-prefixed ones for every other key-requiring source
  theHarvester supports that no other installed plugin already covers.
  Every one is optional; theHarvester works fine on its many free/
  keyless sources alone if nothing is configured.
  Documented both new patterns in `PLUGINS.md`: that reusing the same
  `env_var` across plugins for a shared provider is intentional and
  supported, not a collision to avoid; and a new "wrapping a tool that
  only reads secrets from its own config file" section generalizing the
  existing CLI-flag-wrapper pattern to the config-file case, since this
  is the second tool found with this exact shape.
  No `SKILL.md` change needed - this is a Phase 1 passive-enumeration
  tool, already covered by its existing generic wording.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean (the `scan_plugins`-based tests exercise every
  manifest under `plugins/`, including this one's 38 secrets, and
  passed). No docker access in this sandbox to build/run the image
  locally; live verification pending install.

- Expanded `nuclei` to enable its own DAST/fuzz templates (SQLi, XSS,
  SSRF, SSTI, command injection, and related classes) - a direct
  follow-up to a request for tools that actively identify likely
  high-impact vulnerability areas, not just known-signature/CVE
  matching. Manifest-only change - no Dockerfile/image rebuild needed,
  since the relevant templates (a real, separate ProjectDiscovery
  `dast/` template set - confirmed real but modest: 2 sqli, 3 xss, 2
  ssrf, 6 ssti, 3 cmdi templates, not a huge library) turned out to
  already be baked into this plugin's existing template bundle, just
  never enabled. Confirmed nuclei's own out-of-band interaction client
  (interactsh, used for blind SSRF detection) is already on by default
  (only a disable flag exists) before assuming a separate mechanism was
  needed for that specific class.
  New `dast` option is the master switch; `fuzz_aggression` (low/
  medium/high) defaults to `low`, matching ordinary internet-facing
  scan noise rather than either silent or maximally aggressive, so an
  initial pass gives triage-level signal without excessive noise -
  raise it deliberately once triage has identified where to spend
  deeper effort. `fuzz_scope`/`fuzz_out_scope` keep fuzzing itself
  properly bounded to the authorized target. Documented plainly that
  `dast=true` crosses into the same attack-shaped-payload category
  §0 already covers for `wafw00f` - default (non-dast) nuclei usage is
  unaffected and remains passive signature-matching as before.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean. No docker access in this sandbox; live
  verification pending install (manifest-only, so a lightweight
  reinstall, not a rebuild).

- Added `sqlmap`, `dalfox`, and `commix` plugins - active detection for
  SQL injection, XSS, and OS command injection respectively - completing
  the classic high-impact bug-class set requested alongside the nuclei
  DAST expansion above (SSTI/SSRF are covered by that expansion; command
  injection also gets a lightweight nuclei signal there, `commix` gives
  it a dedicated, much deeper one). Per explicit direction: the AI
  controls intrusion depth per engagement, artifacts stay retrievable
  per this fleet's usual design, and each tool's *default* settings
  should approximate the noise level a typical internet-facing target
  already sees from ordinary scanners - enough signal to know where a
  pentester should spend time, not an exhaustive run by default.
  All three confirmed real, current, and (with one exception) still
  actively maintained before building anything: `sqlmap` (real commits
  the day before this was written, real PyPI package matching the
  project exactly - `1.10.8` on both, no stale-index problem this time),
  `dalfox` (real commits the day before, completely rewritten in Rust as
  of `v3.2.1`, a fresh official Docker image pushed the same day as that
  release), `commix` (real commits the same day, recent commit messages
  describing genuine reliability improvements, not just churn - the
  actual exception is its PyPI package, drastically stale at `0.1`,
  built from source instead).
  Two real bugs caught by reading `dalfox`'s actual upstream Dockerfile
  directly before trusting the official image as-is: it uses `CMD`, not
  `ENTRYPOINT` (this fleet's declared options/arguments become the
  container command, which would have replaced a bare `CMD` entirely
  rather than appending to it), and it `chown`s its own binary to a
  named non-root user this fleet's always-overridden UID 65534 has no
  particular relationship to - fixed with an explicit `ENTRYPOINT` and a
  `chmod a+rX` before either was ever run for real, not discovered live.
  Each tool's own real intrusiveness/noise dials were confirmed directly
  from source, not assumed from memory or README text - and in two
  cases this directly changed the design: `commix`'s `--level` turned
  out to control *which input vectors get tested* (URL/POST → +cookies
  → +headers), not payload aggressiveness as the name suggests;
  `sqlmap`'s `--risk` level 3 specifically adds OR-based payloads that
  can alter what a live query actually returns, a genuinely different
  kind of risk than "more requests," not just a bigger version of level
  1's behavior. Both are documented plainly rather than described by
  their surface-level names alone. All three default to their
  respective quietest built-in posture (`sqlmap`: `level=1 risk=1
  technique=BEUTQ`, deliberately excluding stacked-queries `S` since it
  can execute arbitrary additional SQL rather than only detect;
  `dalfox`: `deep_scan` off, matching ordinary scan noise, with a
  `dry_run` option that plans a scan without sending a single payload at
  all; `commix`: its own real defaults, `skip_waf` left off).
  All three are detection-only by design: each real tool's own separate,
  explicit exploitation-tier features (`sqlmap`'s `--dump`/`--sql-shell`/
  `--os-shell`, `commix`'s `--os-cmd`/`--file-write`/`--file-read`) are
  deliberately not exposed - finding and reporting a likely issue is
  each plugin's job, not exploiting one. Documented a real, confirmed
  `dalfox` exit-code quirk before it could be mistaken for a bug: `1`
  means findings were reported (a successful, informative run), not a
  failure - only `2` is a real error.
  Added a new, plugin-agnostic paragraph to `SKILL.md`'s Phase 4 tying
  this whole capability area together: default to a plugin's own
  quietest setting first, escalate deliberately once triage has
  identified a specific area worth deeper attention, and treat detection
  results as a human-directed decision point rather than something to
  escalate into exploitation on its own initiative. Checked with `grep`
  - no plugin/tool names added.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean (the `scan_plugins`-based tests exercise every
  manifest under `plugins/`, including all three new ones, and passed).
  No docker access in this sandbox to build/run any of these images
  locally; live verification pending install for all three.
