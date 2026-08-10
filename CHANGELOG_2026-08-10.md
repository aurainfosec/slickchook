# Changelog — 2026-08-10

- Ran a documentation-and-feature audit of all 24 installed plugins,
  assessing how well each self-documents to an MCP-connected agent and
  where real underlying-tool capabilities were left unexposed (published
  as an artifact for review before implementing). Four recurring
  patterns emerged across the fleet: tuning knobs (threads/rate limits)
  baked into `fixed_args` instead of exposed as options; timing controls
  that only went in the aggressive direction, never the stealthy one;
  default tool fingerprints (user-agent strings) left unoverridden; and
  overlapping recon tools whose differences weren't explained in the
  manifest itself. Implemented fixes for all four patterns across 12
  plugins, verifying every non-obvious flag against each tool's real
  `--help` output before writing it into a manifest rather than
  guessing — this caught one wrong assumption (`medusa` has no real
  per-attempt delay flag; left unchanged, it's already correctly
  conservative) and one pleasant surprise (`testssl.sh` ships literal
  `--sneaky`/`--ids-friendly` network-evasion flags already):
  - `feroxbuster`: `threads`/`rate_limit` moved from hardcoded
    `fixed_args` to real, tunable options; added `user_agent`; added two
    new small bundled wordlists (`wordlist-cms.txt`,
    `wordlist-api.txt`) alongside the existing generic one, selectable
    via a new `wordlist` option.
  - `nmap`: replaced the boolean `fast_timing` (only ever `-T4`) with a
    real `timing_template` option covering Nmap's full `0`-`5` timing
    scale, including the actual IDS-evasion end (`-T0`/`-T1`) that
    was previously unreachable.
  - `trufflehog`: added `only_verified` (filters to secrets TruffleHog
    confirmed are still live, sharply cutting false positives) and
    `concurrency`.
  - `kiterunner`: `concurrency`/`jobs` moved from hardcoded `fixed_args`
    to real options.
  - `dnsx`: added `threads`/`rate_limit`, confirmed against real
    `dnsx -h` output (`-t`/`-rl`).
  - `httpx`: added `threads`, `timeout`, `delay`, and
    `follow_redirects`, confirmed against real `httpx -h`. Documented
    that `httpx` already randomizes its user-agent by default, so no
    option was needed there.
  - `nuclei`: added `concurrency`, `timeout`, and `retries`, confirmed
    against real `nuclei -h`.
  - `gowitness`: added `user_agent` (override; default is already a
    real recent Chrome UA, not an obviously automated one), `timeout`,
    and `delay` (page-render wait, not request pacing — gowitness only
    ever screenshots one URL per run). Trimmed the description to lead
    with what the tool does; moved the `/input`-mount build rationale
    into a source comment.
  - `testssl`: added `vulnerable_only` (narrows the ~900s full battery
    to just vulnerability checks), and testssl's own real `sneaky` and
    `ids_friendly` evasion flags, plus `user_agent`.
  - `amass`: documentation-only — states explicitly that `-passive`
    means it never sends traffic to the target itself, only to
    third-party OSINT sources.
  - `webfetch`: added `user_agent`.
  - `spiderfoot`: broadened from 2 DNS-only modules to include
    `sfp_crt` (certificate transparency), `sfp_archiveorg` (Wayback
    Machine), and `sfp_builtwith` (third-party tech-stack
    fingerprinting) — all confirmed genuinely passive (never contact
    the target directly) against the real module list, closing the gap
    between the description's "OSINT collection" claim and what
    actually ran (previously just DNS resolution).
  `gospider` was left out of this batch: its image has never actually
  been built on this server (`docker` tried to pull it from Docker Hub
  instead of building locally), so its real `--help` output couldn't be
  checked. Deferred rather than guessed at.
  Verified for real: `cargo build --locked --release --bins` and
  `cargo test --lib` (12/12) both clean after every change; manually
  re-checked argv ordering for every touched manifest (fixed_args →
  options → arguments) against each tool's actual CLI syntax, after
  finding and fixing a real ordering bug in `graphqlcheck`'s manifest
  during this same pass (see the next entry, below).
- Added fingerprint-driven tool selection to `SKILL.md`, and a
  wordlist-based field-guessing fallback to `graphqlcheck`, closing out
  the rest of the plugin capability audit follow-up:
  - `SKILL.md`'s Phase 3 now explicitly steers tool choice off `httpx`'s
    `tech_detect` fingerprint instead of just using it to prioritize
    which hosts get attention: a CMS fingerprint selects
    `feroxbuster`'s new `/wordlist-cms.txt`, an API-shaped stack selects
    `/wordlist-api.txt` and weights `kiterunner`/`graphqlcheck` over
    `gospider`, and a GraphQL endpoint with introspection disabled
    triggers a `graphqlcheck` re-run with the new `guess_fields` option
    below rather than treating a closed introspection query as the end
    of the check.
  - `graphqlcheck` gained a `guess_fields` mode: a new bundled
    55-entry wordlist of common GraphQL query/mutation/type names,
    probed one at a time against the root `Query` type and classified
    by error-message differencing (a field that errors "unknown field"
    doesn't exist; one that errors any other way, e.g. missing
    arguments, or that succeeds outright, likely does) — the standard
    technique real tools like clairvoyance/graphql-cop use, for when
    introspection itself is disabled. Smoke-tested the classification
    regex locally against realistic GraphQL error-message shapes before
    shipping.
  - While building this, caught and fixed a real bug in `graphqlcheck`'s
    own manifest introduced earlier in the same editing pass: adding
    `guess_fields` as a declared option without also converting
    `target` from a positional `[[arguments]]` entry to a flagged
    `[[options]]` entry would have rendered as `--url --guess-fields
    <target>` — `--guess-fields` would have been consumed as `--url`'s
    value, breaking every call. Manifests render `fixed_args` → options
    → arguments in that fixed order, so any option declared between a
    fixed_args flag and its intended value breaks that flag; every
    other manifest touched in this and the prior PR was re-checked
    against this exact failure mode before either was shipped.
  Verified for real: `cargo build --locked --release --bins` and
  `cargo test --lib` (12/12) clean; confirmed no leftover argv-ordering
  risk across every manifest touched in both this and the prior PR.
- Added three new plugins: `wpscan`, `shodan-host`, and `shodan-search` —
  each needs a real API token (`WPSCAN_API_TOKEN`, `SHODAN_API_KEY`)
  configured via the `[[secrets]]` mechanism built earlier this week,
  never as a per-call input.
  - Neither WPScan nor the official `shodan` CLI reads its token from an
    environment variable the way `[[secrets]]` injects one — both expect
    it as a CLI flag (`--api-token`) or a one-time `init` step. Both
    plugins wrap the real tool in a tiny shell entrypoint that reads the
    injected env var and does that translation itself, so the token
    still never passes through slickchook's own argv construction at
    any point — documented as a general pattern in `PLUGINS.md`
    ("Wrapping an upstream tool that wants the secret as a CLI flag")
    for any future plugin with the same need.
  - `shodan-host` and `shodan-search` are two separate plugins, not one
    with a mode switch: `shodan host <ip>` is bounded to a single
    authorized target the way every other plugin here is; `shodan
    search <query>` queries Shodan's whole database and is not bounded
    to any one target at all. Keeping them separate means the
    unbounded one can't be reached for by accident, and `SKILL.md` now
    calls out `shodan-search`'s scope risk explicitly (§0), the same
    way it already does for `medusa`.
  - `wpscan` defaults `enumerate` to `vp,vt` (vulnerable plugins/themes
    only) rather than a full enumeration, matching this repo's general
    bias toward bounded-by-default plugins; exposes WPScan's own real
    `--stealthy`/`--random-user-agent`/`--throttle` options.
  - None of the flags beyond `--url`/`--api-token`
    (`wpscan`)/`host`/`search` (`shodan-*`) have been confirmed against
    real `--help` output yet — neither image had ever been built on
    this server. Said so explicitly in each manifest's description
    rather than presenting unverified flags as confirmed, matching the
    verification discipline used for every other plugin this week.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean; all three new manifests parse and validate via
  `scan_plugins`; all three entrypoint scripts pass `sh -n` syntax
  checking. Not yet built or run live — needs a real image build and a
  configured API token before further verification is possible.
- Changed `wpscan`'s default `enumerate` from `vp,vt` (vulnerable
  plugins/themes only) to `ap,at` (every detected plugin/theme, each
  still flagged if WPScan's own database knows it's vulnerable) — WPScan
  treats `ap`/`at` as supersets of `vp`/`vt`, not a separate mode, so
  this doesn't lose the vulnerability flagging, it just also keeps
  everything WPScan didn't flag. The point: WPScan's vulnerability
  database can lag behind a real CVE, especially for a less widely used
  open-source plugin/theme, so the full enumerated list is worth a
  manual look even when nothing gets flagged. Raised `timeout_seconds`
  from 600 to 900 to match the heavier default scope.
  `SKILL.md` updated to match: instructs treating WPScan's own flagged
  findings as headline results while still pulling the full enumerated
  list from the artifact for manual follow-up, and the summary format
  now reports both separately rather than only the flagged findings.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean.
- Fixed `wpscan`'s first real build, which failed outright: `chmod:
  /usr/local/bin/slickchook-wpscan-entrypoint.sh: Operation not
  permitted`. `wpscanteam/wpscan`'s own Dockerfile sets a non-root
  `USER` before its final layer, which carries over into a derived
  Dockerfile that doesn't override it; `COPY` always writes as
  `root:root` regardless of the current effective user, so the
  inherited non-root user couldn't `chmod` a file it didn't own. Fixed
  by adding an explicit `USER root` before the `COPY`/`chmod` steps.
  `shodan-host` and `shodan-search` built and installed cleanly on the
  first attempt (their images build from `python:3.13-alpine` directly,
  with no inherited `USER` to work around).
- Fixed `shodan-host` failing every real run: `ModuleNotFoundError: No
  module named 'pkg_resources'`. The `shodan` PyPI package still
  imports `pkg_resources` (from `setuptools`) at startup, but recent
  `python:*-alpine` images no longer bundle `setuptools` by default.
  Fixed both `shodan-host` and `shodan-search`'s Dockerfiles (identical
  base image) by installing `setuptools` alongside `shodan`.
  Found live: a real `shodan-host` call against `1.1.1.1`, with a
  correctly configured `SHODAN_API_KEY`, failed with this exact
  traceback before the fix.
- Correction to the entry above: that fix was wrong. Rebuilt and
  re-ran the exact same live `shodan-host` call and got the identical
  `ModuleNotFoundError: No module named 'pkg_resources'` traceback,
  even with `setuptools` now installed. Diagnosed live inside the
  built image rather than guessing again: `pip show setuptools`
  reported version `84.0.0`, and `python3 -c 'import pkg_resources'`
  still failed the same way — `setuptools` has removed `pkg_resources`
  entirely as of `81.0`, so a bare `pip install setuptools` installs
  whatever the latest version is and doesn't actually restore the
  module `shodan` needs. Fixed both Dockerfiles by pinning
  `setuptools<81` instead of an unpinned install.
  Re-verified live after this fix: `shodan-host` against `1.1.1.1`
  returned real Shodan data (hostnames, open ports, cert info) with
  only the expected `pkg_resources` deprecation warning, no error.
  `shodan-search` against a scoped query (`hostname:cloudflare.com`)
  correctly built its argv and reached Shodan's API, which itself
  returned `403 Forbidden` — most likely an account-tier restriction
  on the `search` method specifically (it typically costs query
  credits `host` lookups don't), not a plugin bug, since the same key
  worked fine for `shodan-host` moments earlier.
- Fixed `wpscan` failing every run: `"scan_aborted": "Update
  required, you can not run a scan if a database file is missing."`
  `--no-update` was carried over from `nuclei`'s
  `-disable-update-check`/`subfinder`'s `-disable-update-check`
  pattern on the assumption it meant "skip a redundant check" - for
  WPScan it actually means "don't even initialize the database," and
  every run here is a fresh, ephemeral container with no persisted
  state, so there's never an existing database to skip updating.
  Removed `--no-update` from `fixed_args`; WPScan now updates its own
  fingerprint database over the network at the start of every run.
  Documented the resulting network-dependent startup cost in the
  description, and noted baking the database into the image at build
  time as a possible future optimization, not yet attempted (build-time
  baking would need to land in a path outside any tmpfs mount, since a
  runtime tmpfs mount shadows whatever was baked into that same path at
  build time - not attempted here to avoid guessing wrong a second time
  in the same session).
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean.
- Fixed `wpscan` running out of disk space mid-scan: `"scan_aborted":
  "No space left on device @ rb_sys_fail_on_write -
  /tmp/.cache/wpscan/cache/..."`, found on the very next real run after
  the fix above got the database update itself working. WPScan caches
  HTTP response bodies under `$HOME/.cache/wpscan` (`/tmp/.cache/wpscan`
  here) during a scan — this one real `ap,at` run against
  `wordpress.org` alone received 61.9 MB (`data_received`) before
  hitting the 64m `/tmp` tmpfs limit. That same real run also proved
  the separate `/.wpscan` tmpfs mount from the original manifest was
  never used at all - WPScan's actual cache path is
  `$HOME/.cache/wpscan`, not `$HOME/.wpscan`. Removed the unused mount
  and raised `/tmp` to 512m (well within the container's existing 1g
  memory budget - actual RSS in the same run was `used_memory: 188.7
  MB`, plenty of headroom left).
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean.
- Fixed `wpscan` failing on real, larger targets after the previous two
  fixes: verified live against `wordpress.org` with the tmpfs fix in
  place, no disk-space error this time, but the run hit `plugin
  \`wpscan\` exceeded timeout of 900 seconds` and was killed by
  slickchook's own container timeout before finishing. This is a
  direct consequence of the deliberate `ap,at` (enumerate-everything)
  default added earlier — a full plugin/theme enumeration plus the
  mandatory per-run database update can genuinely take well over 15
  minutes on a site with a large footprint. Raised `timeout_seconds`
  from 900 to 2700 (45 minutes) and documented in the description that
  narrowing `enumerate` (e.g. to `vp,vt`) is the way to get a faster
  run, not lowering this timeout.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean.
- Fixed `wpscan`'s full `ap,at` enumeration genuinely failing to finish
  even within the 2700-second timeout raised in the previous entry:
  live-verified against a real production WordPress site with a
  non-trivial plugin/theme footprint, the run still hit `plugin
  \`wpscan\` exceeded timeout of 2700 seconds` with zero JSON output
  produced. Root cause, confirmed against this image's real
  `wpscan --help` output (v4.1.0) rather than guessed: `ap`/`at` mode
  brute-forces WPScan's entire known plugin/theme slug list against
  the target instead of only what's detected, and WPScan's own
  concurrency default is only 5 threads (`-t`/`--max-threads`) - far
  too low to finish that volume of requests in time. Added a
  `max_threads` option (default 10, double WPScan's own default) to
  raise concurrency for a realistic chance of finishing; documented
  that it has no effect when `throttle` is also set, since `throttle`
  forces WPScan's internal max-threads to 1 regardless. Also replaced
  the manifest's stale "flags not yet verified against real --help"
  disclaimer, since they now have been.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean.
