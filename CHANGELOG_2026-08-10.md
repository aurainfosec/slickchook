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
