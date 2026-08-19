# Changelog — 2026-08-19

- Expanded `virustotal-resolutions`' description to explain not just
  why historical resolution data matters (finding an origin IP behind
  a WAF/CDN) but how to actually test a candidate IP: connect directly
  to it while still presenting the real domain as the Host
  header/TLS SNI (e.g. `curl --resolve <domain>:443:<historic-ip>
  https://<domain>/`), since the origin often still serves the site
  based on virtual-host matching regardless of which IP the request
  arrived on - hitting the bare IP directly instead would likely miss
  the intended virtual host or TLS certificate entirely. Prompted by a
  direct request to verify this plugin's documentation actually
  covered the WAF-bypass testing methodology, not just the data
  source. Checked first whether `httpx`/`webfetch` already exposed a
  Host-header-override option to reference instead of describing raw
  curl usage - neither currently does, so this documents the general,
  standard technique directly rather than assuming a companion
  plugin capability that doesn't yet exist.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean (manifest description update only).

- Added a new `katana` plugin (ProjectDiscovery's next-generation
  crawler) - a modern, JS-aware complement to `gospider` for content
  discovery: it parses endpoints out of JavaScript files that a plain
  HTML crawler never sees, which matters on a modern JS-heavy site.
  References the official `projectdiscovery/katana:v1.7.0` Docker
  image directly (confirmed real via its Dockerfile/dockerhub-push CI
  workflow) - no custom build needed. Every flag verified against the
  real v1.7.0 binary's own `-h` output (downloaded the Linux release
  directly, no docker access in this sandbox) rather than guessed.
  Deliberately scoped to Katana's well-understood non-headless HTTP
  crawling mode for this first pass - the image bundles Chromium for
  headless crawling, but its exact chrome-binary-path requirements in
  this specific image haven't been verified live, and two real bugs
  shipped this week already from assuming an unverified detail worked
  (vt-cli's tag naming, then its init requirement) - not repeating
  that a third time on an unverified guess. Headless mode can be added
  later once actually tested.
  Flagged two options as genuine active side effects, not passive
  crawling, matching this fleet's usual credential/live-action
  callouts: `automatic_form_fill` (experimental - actually fills in
  and likely submits discovered forms, real side effects on the
  target) and `kb_validate_secrets` (sends live API calls to each
  detected secret's own provider to check if it's actually valid).
  `no_scope` is flagged as the one option that can take a crawl
  outside the authorized domain entirely.
  No `SKILL.md` change needed - Phase 3's existing generic "installed
  content-discovery/crawling plugins" wording already covers this
  addition without naming any specific tool, confirming the
  plugin-agnostic rewrite works as intended for a brand-new plugin.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean. Live verification (a real crawl) pending
  install, since this sandbox has no docker access.

- Fixed two real bugs in `katana` found on the very first live test
  against `scanme.nmap.org`:
  1. `known_files` is a single-choice value, not a comma-joinable
     list - passing `"robotstxt,sitemapxml"` errored outright
     (`invalid value ... allowed values are , all, robotstxt,
     sitemapxml`). The `--help` text's `(all,robotstxt,sitemapxml)`
     formatting looked identical to other options in this fleet that
     genuinely do accept a joined list (e.g. naabu's `ip_version`),
     but isn't one here - corrected the option's description to say
     so explicitly, confirmed live.
  2. Every call failed with `Could not read flags: cause="no such
     file or directory"` - Katana tries to read a default config file
     relative to `$HOME` at startup, which is unset under this
     fleet's usual unprivileged UID (no matching /etc/passwd entry) -
     the same class of bug already hit and fixed for
     `wpscan`/`shodan-host`/`subfaster`, just missed here since this
     plugin had been referencing the official image directly with no
     wrapper at all. Added a thin `Dockerfile`
     (`FROM projectdiscovery/katana:v1.7.0` + `ENV HOME=/tmp`) and
     changed `container` to the resulting local image - the smallest
     possible fix, no other customization needed.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean. Live re-verification (a real crawl succeeding
  end-to-end) pending redeploy.

- Documented every real bug found while building/fixing plugins this
  week directly in `PLUGINS.md`, so future plugin work benefits from
  them instead of repeating them:
  - Corrected the existing `$HOME`/read-only-filesystem guidance,
    which only described mounting a tool's specific `/.config`/`/.cache`
    subpath - added the simpler `ENV HOME=/tmp` fix (one setting
    handles any path, no need to discover the tool's exact subpath)
    for whenever a Dockerfile is actually controlled, reserving the
    specific-subpath-mount approach for pulled-image-only plugins with
    no Dockerfile at all.
  - Fixed a now-stale claim that "plugin containers always run as UID
    65534" (no longer true since `run_as_root` was added) and added a
    full explanation of when that field is actually warranted (`nmap`)
    versus not (`naabu`, which checks the `NET_RAW` capability
    directly and falls back gracefully), plus the artifacts-ACL
    dependency it has on the host.
  - Added a new "Verify against the tool's real behavior, not its
    docs or `--help` text" section covering every other bug class hit
    this week as a standing checklist: docs/generated help pages can
    be stale relative to an exact pinned ref (`virustotal-resolutions`);
    a tool can need a one-time init step before its real commands even
    exist (`vt-cli`, `shodan`); a flag that looks comma-combinable in
    `--help` text might be a strict single-choice enum instead
    (`katana`'s `known_files`); a release tag doesn't guarantee it's a
    valid Go module version (`vt-cli`'s unprefixed tags); the install
    step never runs the container for any plugin, so nothing is
    verified until a real live call happens; and correcting a wrong
    fix should be a new, honest entry, not a silent edit to history
    (the `shodan-host`/`shodan-search` `setuptools` episode).
  Documentation-only change, no build/test impact.

- Added a new `alterx` plugin (ProjectDiscovery's pattern-based
  subdomain permutation generator) - fills the gap between Phase 1
  passive discovery and Phase 2 active resolution: generates realistic
  candidate subdomains from the naming patterns in what's already been
  found (e.g. api-eu.example.com found -> api-us.example.com guessed),
  rather than blind brute-forcing, since candidates come from the
  actual input rather than a generic wordlist. Surfaced by researching
  what's trending in the recon community; independently corroborated
  as part of a commonly-cited "essential toolkit" alongside several
  tools already in this fleet.
  References the official `projectdiscovery/alterx:v0.1.0` Docker
  image (confirmed real via its Dockerfile/dockerhub-push CI workflow).
  Every flag verified against the real v0.1.0 binary's own `-h` output
  (downloaded the Linux release directly, no docker access in this
  sandbox).
  Proactively applied the `$HOME`-relative-default-config lesson just
  documented in `PLUGINS.md` from `katana`, rather than waiting to
  rediscover it live a third time: confirmed directly with the real
  binary that it panics outright (`$HOME is not defined`) with no
  `$HOME` set, and produces correct, sensible output once `HOME=/tmp`
  is set - added the same thin wrapper Dockerfile pattern from the
  start.
  `network = "none"` - this tool is pure local pattern-based text
  generation with no network activity of its own (confirmed by testing
  it fully locally), one of relatively few plugins in this fleet that
  never touches the network at all.
  Updated `SKILL.md` with a new, plugin-agnostic "between Phase 1 and
  Phase 2" section describing the permutation-generation role
  generically, per the standing plugin-agnostic-methodology rule.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean. Also live-tested the real binary directly
  (both the $HOME panic and the working HOME=/tmp case) before writing
  the manifest, catching the bug before it ever shipped rather than
  after. Full end-to-end verification via the actual MCP server still
  pending install, since this sandbox has no docker access.

- Fixed `alterx`'s Docker image not existing at all: install failed
  outright, `docker.io/projectdiscovery/alterx:v0.1.0: not found`.
  Confirmed live against Docker Hub's actual tag list: the official
  image was only ever published up to `v0.0.6` (March 2025) - `v0.1.0`
  was never pushed there. Checked the real `v0.0.6` binary's own
  `-h` output before just downgrading the tag, since that's a
  meaningfully older release: it's missing the entire pattern-mining
  feature set (`mode`/`min_distance`/`max_distance`/
  `pattern_threshold`/`quality_ratio`/`ngrams_limit`/`save_rules`) this
  manifest already relies on, since it predates that feature entirely.
  Rather than strip those options back out, switched to building from
  source pinned to the exact `v0.1.0` tag instead (matching the
  `subfaster`/`katana` precedent for a missing/inadequate official
  image) - confirmed the git tag is properly `v`-prefixed this time
  (unlike `vt-cli`'s mismatch), so the straightforward `go install
  .../alterx/cmd/alterx@v0.1.0` actually works.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean. Live install/build verification still pending.

- Exposed each run's exact `input_json` in `list_runs_by_target`'s
  output, so a caller can tell whether a prior run actually used the
  options it's about to use (safe to reuse) or different ones (worth
  re-running) without an extra `read_artifact` call just to parse the
  stderr's `args=[...]` log line to find out. The `plugin_runs` table
  already stored this for every run; it just wasn't surfaced in the
  summary. Added `input_json: serde_json::Value` to `RunSummary`
  (`src/storage.rs`) and the matching column to the query - no schema
  migration needed, `serde_json::Value` already implements `FromSql`
  for the existing `input_json JSONB` column (the same type already
  used to write it via `record_run`, so no new dependency either).
  Updated `SKILL.md`'s `list_runs_by_target` bullet to describe the
  new field and how to use it.
  Verified: `cargo build --locked --release --bins` and `cargo test`
  (14/14 - 12 lib + 2 main.rs) clean, including the real
  `postgres_store_round_trip_when_configured` test exercising
  `list_runs_by_target` against a real database. This is a core Rust
  change - redeploying needs the full `build-deb.sh`/`apt install`
  path, not a plugin-manifest reinstall. Live verification (a real
  `list_runs_by_target` call showing input_json) still pending
  redeploy.

- Added a new `cupp` plugin (CUPP - Common User Passwords Profiler) -
  builds a personalized password wordlist from real details about one
  specific, authorized individual (name, birthdate, partner/pet/child's
  name, employer), encoding the exact patterns people commonly build
  passwords from. A genuinely novel wrapper pattern for this fleet:
  CUPP's own CLI has no non-interactive/flag-based mode at all for
  this - confirmed by reading its source, the only other modes
  download a static leaked-credentials database or expand an existing
  wordlist file, neither of which is personalized profiling. Its real
  functionality is only reachable via an interactive stdin prompt
  sequence.
  Rather than skip the tool over this, wrote a small Python entrypoint
  that translates declared CLI options into the exact prompt-answer
  sequence CUPP's `-i` mode expects, then feeds it via stdin and execs
  the real tool. Confirmed the exact sequence by actually running the
  real script end-to-end with a synthetic stdin, not assumed from
  source alone - this caught a real gotcha: a final "Hyperspeed
  Print?" prompt lives in a helper function called at the very end,
  not in the main interactive() function itself, and is easy to miss
  from a source read alone. Birthdate fields are validated (exactly 8
  digits or empty) before construction, since a malformed value would
  otherwise trigger CUPP's own re-prompt loop in a way this wrapper's
  one-shot synthetic input can't recover from, desyncing every answer
  after it.
  No git tag exists for this repo at all - pinned by commit SHA
  instead, the only reproducible option available. GPLv3 licensed.
  `network = "none"` - the `-i` code path used here makes no network
  calls of its own (confirmed by reading the source: networking is
  only used by the other, unused modes).
  Updated `SKILL.md` with a role-based mention in Phase 5 alongside
  the new `cewl` plugin, per the standing plugin-agnostic-methodology
  rule.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean. Also ran the actual wrapper script end-to-end
  against the real cupp.py directly in this sandbox (both a
  successful full run producing a real 6531-word list, and the
  malformed-birthdate validation path failing cleanly) before writing
  the manifest - not just assumed the design would work. Full
  container-level verification via the actual MCP server still
  pending install, since this sandbox has no docker access.

- Substantially expanded `waymore` to actually use its main
  differentiator, prompted by a direct request to make it "look more
  extensively for historic information." The previous manifest
  hardcoded `-mode U --stream` (URLs only) and exposed only 4 options -
  meaning the entire reason to prefer this tool over a plain archived-URL
  lister (downloading the real archived page *content* - developer
  comments, removed parameters, old endpoints - not just URLs) was
  unused. Fetched the exact v8.9 source (`gh api
  .../xnl-h4ck3r/waymore/contents/waymore/waymore.py`, no local pip
  available in this sandbox) and read its real `argparse` definitions
  rather than trusting README/marketing text, per this week's standing
  "verify against real behavior" rule.
  Changed the default `mode` to `B` (URLs + downloaded responses) and
  exposed `~25` more real options: response-download tuning (`limit`,
  `capture_interval`, `timeout`, `processes`, `retries`), targeted
  filtering (`match`/`filter` by status code and MIME type,
  `keywords_only`, `regex_after` applied against actual downloaded
  content - directly useful for e.g. finding leaked private keys across
  archived pages instead of reviewing everything by hand),
  `output_inline_js` (pulls combined inline JS out of downloaded
  responses), per-source exclude flags, Common Crawl collection depth
  (`limit_collections`), and `check_only` (dry-run request/time
  estimate before committing to a large run).
  Found and fixed a real mechanical bug before it ever shipped: with
  `HOME=/tmp` already set, Waymore's own default output location for
  downloaded responses resolves to `/tmp/.config/waymore` - writable,
  but entirely outside `/artifacts`, so downloaded content would never
  have been retrievable via `read_artifact`/`list_run_artifacts`.
  Fixed by defaulting `-oU`/`-oR` to fixed `/artifacts/...` paths.
  Also confirmed via source that Waymore's three optional source API
  keys (URLScan, VirusTotal, Intelligence X) are read only from its own
  `config.yml`, never an environment variable - this fleet's usual
  `[[secrets]]` mechanism can't reach them without a new
  config-templating wrapper, which wasn't built here (out of scope for
  this pass) - documented clearly in the plugin description so this
  is a known, deliberate gap rather than a silent one.
  Set `limit`'s default to 200, well below Waymore's own default of
  5000 - real archive.org/URLScan rate limiting means a large response-
  download count can take a very long time regardless of container
  timeout, so raised `timeout_seconds` to 2700 (45 min) to give a
  `mode=B` run real headroom, matching the `wpscan`-precedent lesson
  that an exhaustive-by-default tool needs a generous timeout rather
  than a restrictive one paired with a short deadline.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean (manifest-only change, no core Rust touched).
  No docker access in this sandbox to build/run the image locally;
  live verification (a real `mode=B` run producing retrievable
  downloaded responses under `/artifacts`) pending install.

- Live-verified the `waymore` expansion above after redeploy: a real
  `mode=B` run against `scanme.nmap.org` (`limit=10`, `to_date=202501`)
  completed successfully and produced actual downloaded response files
  (HTML pages, a binary `.unknown` file, GhostArchive's own working
  file) under `/artifacts/responses/`, alongside `urls.txt` under
  `/artifacts/` directly - confirming the output-path fix actually
  works, not just compiles. Argv ordering also confirmed correct end
  to end (`-i ... -mode B -oU ... -oR ... -l 10 -to 202501 -lcc 1 -lr
  500`). Common Crawl and GhostArchive both logged non-fatal connection
  errors on this specific run (exit code 0 regardless) - a transient
  upstream/network condition on those two specific sources, not caused
  by this change; noted here rather than silently ignored, since a
  future recurrence is worth knowing this precedent exists.

- Added a new `dehashed` plugin (Dehashed's v2 breach-data search API -
  emails, usernames, passwords, hashed passwords, names, domains, IPs,
  phone numbers, addresses, VINs) - fulfilling a direct request for a
  plugin that "allows searches and suggests API searches to perform,
  given that API requests use credits and so should be done by a
  human." No official docs were fetchable (dehashed.com/docs
  Cloudflare-blocks automated fetches with a 403) and no official CLI
  exists - confirmed the real v2 endpoint (`api.dehashed.com/v2/search`),
  its single-header auth (`Dehashed-Api-Key`), request/response shape,
  and ~1-credit-per-call (not per-result-row) cost model by reading two
  well-established unofficial wrappers' actual source
  (`hmaverickadams/DeHashed-API-Tool`, `l4rm4nd/DehashedDumper`) rather
  than trusting marketing copy, per this week's standing "verify
  against real behavior" rule.
  Design directly encodes the credit-cost constraint rather than
  relying on documentation alone: makes exactly one API call per
  invocation with no automatic pagination at all (`page`/`size` are
  exposed so a human can deliberately fetch one additional page as its
  own separate, visible, credit-costing call) - going further than
  `DeHashed-API-Tool`'s own precedent, which already interactively
  confirms with a human before any call likely to exceed 25 automatic
  pages. Always returns Dehashed's own `balance` (credits remaining)
  and `total` (matching row count) unmodified, so the real cost and
  true result count are visible after every single call. The manifest
  description itself instructs never to call this speculatively or run
  a string of exploratory queries unprompted - propose the specific
  query text to the operator and only run the exact one they confirm.
  Also extended `SKILL.md`'s existing, plugin-agnostic §0
  "internet-wide/third-party database" scope-discipline bullet with
  this same cost-awareness rule generically (any plugin whose
  description says its queries cost real per-call money/credits),
  rather than writing a dehashed-specific mention, per the standing
  plugin-agnostic-methodology rule.
  Wrapped in a from-scratch, stdlib-only Python HTTP client
  (`entrypoint.py`) rather than a third-party dependency, since no
  official/established CLI or SDK exists to wrap.
  Deliberately left the `/v2/search-password` endpoint (a
  password-focused search variant Dehashed's API also exposes)
  unwired, since its exact request/response shape hasn't been verified
  against the real API yet - documented as a known, deliberate gap
  rather than guessing at its shape.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean (new plugin manifest, no core Rust touched); no
  plugin names leaked into `SKILL.md`'s edit (checked with `grep`). No
  docker access in this sandbox to build/run the image locally, and no
  real Dehashed API key available to test against live - both full
  container build and a real API call are pending install and a
  configured `DEHASHED_API_KEY`.

- Installed `dehashed` on the deployed host: the image built and
  installed cleanly. A real live MCP call against it was attempted for
  verification but was itself blocked by this session's own auto-mode
  tool-use classifier (a breach-data-search invocation is exactly the
  kind of thing that guardrail is meant to catch, even though this
  particular call would only have hit the plugin's own missing-secret
  error path, no real API key being configured yet) - by agreement with
  the operator, a successful build+install is treated as sufficient
  verification for now, with a real end-to-end call deferred until a
  real `DEHASHED_API_KEY` is configured and the operator runs (or
  explicitly authorizes) that specific call themselves.

- Added a new `github-code-search` plugin (wraps the official `gh`
  CLI's `search code` command, github.com/cli/cli, pinned to the real
  `v2.97.0` release tarball) - passively finds leaked secrets/
  credentials/internal references committed to an authorized
  organization's public (or, with a suitably-scoped token, private)
  GitHub repos. Confirmed GitHub's code search is only usable via the
  REST `/search/code` endpoint - its newer regex-capable search engine
  is web-UI-only, not exposed by any API (confirmed directly in `gh`'s
  own help text) - and confirmed several real, easy-to-assume-wrong
  constraints directly from GitHub's docs rather than guessing: only
  the default branch of each repo is indexed, files 384KB+ are
  excluded, no regex support at all, and code search specifically is
  rate-limited to 10 requests/minute (much stricter than GitHub's
  general API limit). `owner`/`repo` should always be set to the
  authorized engagement's own GitHub presence - without either, this
  becomes a bare internet-wide GitHub search, exactly the kind of
  third-party-database query this fleet's own scope-discipline rules
  require staying scoped on. `GH_TOKEN` is auto-detected by the `gh`
  CLI itself, so no wrapper/init step was needed here (simpler than
  `vt-cli`/`shodan`'s own secret-handling, which both needed one).
  Description documents several real, known-useful leaked-secret dork
  patterns (`filename:.env`, `extension:pem` + `"BEGIN RSA PRIVATE
  KEY"`, `filename:.npmrc` + `_authToken`, `xoxb-` for Slack tokens,
  etc.) sourced from established OSINT/secret-scanning writeups, not
  invented.
  No `SKILL.md` change needed - Phase 1's existing generic "installed
  plugin whose description indicates passive, no-touch" wording already
  covers this addition, matching the precedent already confirmed when
  `katana` shipped.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean (the existing `scan_plugins`-based tests already
  exercise every manifest under `plugins/`, including this one, and
  passed). No docker access in this sandbox to build/run the image
  locally; live verification pending install.

- Added a new `brave-dork-search` plugin - dork-style passive discovery
  of exposed files/content (backups, configs, leaked secrets) on one
  authorized domain, via the Brave Search API rather than scraping
  Google directly - a genuinely independent search index, avoiding the
  CAPTCHA/blocking problem raw Google-search scraping runs into from a
  single datacenter IP. Confirmed the real endpoint
  (`api.search.brave.com/res/v1/web/search`), its single-header auth
  (`X-Subscription-Token`), and - critically - which classic Google-dork
  operators are actually confirmed supported here by reading Brave's own
  query-syntax docs directly rather than assuming Google-dork parity:
  only `site:`, `filetype:`, quoted phrases, and `-exclusion` are
  documented as working; `intitle:`/`inurl:`/`intext:`, which most of the
  Google Hacking Database actually depends on, are not documented as
  supported and are assumed not to work - the plugin's description says
  so plainly rather than shipping unverified preset dorks built on
  operators that likely don't function against this API. Scope safety is
  enforced structurally, not just by documentation: `target` is a
  required, separate field, and the wrapper always prepends `site:
  <target>` to whatever dork text is given internally, so there is no way
  to invoke this plugin with an unscoped, whole-internet query at all -
  a stronger version of the scope-discipline pattern this fleet has so
  far only enforced via documentation (e.g. `shodan-search`).
  No official CLI/SDK exists for this API - wrapped in a from-scratch,
  stdlib-only Python HTTP client, matching `dehashed`'s pattern. Also
  confirmed (correcting an initial, misremembered claim from earlier
  research) that there is no genuinely free tier - it's $5/1000 requests
  with $5 of free credit applied monthly, and a credit card must be on
  file even to use that free credit.
  No `SKILL.md` change needed, same reasoning as `github-code-search`
  above.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean. No docker access in this sandbox to build/run
  the image locally, and no Brave API key available to test against
  live; both pending install and a configured `BRAVE_API_KEY`.

- Added a new `pagodo` plugin (github.com/opsdisk/pagodo v2.7.0,
  pinned by git tag - no pip package exists, it's a pair of scripts,
  not a library) - runs a small, bounded batch of real Google Hacking
  Database dorks against one authorized domain. Read its own upstream
  README before building anything here: pagodo scrapes Google's HTML
  search results directly via the `yagooglesearch` library, no real
  browser, and its own documentation explicitly acknowledges it will
  get rate-limited/CAPTCHA-blocked without a proxy pool - not wired up
  here, since this fleet has no established proxy-pool convention yet.
  This plugin's own description says so plainly, matching this week's
  standing rule to document real tool behavior rather than silently
  hide a limitation.
  Found a real sizing problem before shipping, not after: pagodo's own
  bundled GHDB corpus is 7,944 dorks total across 14 categories, and
  even its *smallest* single category (121 dorks) would take roughly 97
  minutes at pagodo's own default delay - already longer than any
  timeout this fleet uses anywhere. Added `max_dorks`, a bounding option
  that doesn't exist in pagodo itself, to the wrapper entrypoint: it
  slices the chosen category's dork file down to a small batch (default
  15, roughly 12-15 minutes including delays) - a human deliberately
  runs this again, or raises the value, to cover more of a category,
  rather than the container ever attempting a whole category
  automatically in one shot.
  The dork corpus itself is baked into the image at build time (`python
  ghdb_scraper.py -s -i`, run once during `docker build`) rather than
  scraped live on every run - ties dork-list freshness to a plugin
  rebuild/reinstall, not a live per-run dependency on exploit-db, and
  keeps every real run's own network activity limited to the actual
  Google queries. The wrapper discovers which category files actually
  got baked in dynamically at runtime (rather than hardcoding an
  assumed path/filename list) - an unrecognized `category` value fails
  with the real available list, so the manifest can never silently
  drift from what the image actually contains.
  No `SKILL.md` change needed, same reasoning as the two plugins above.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean. No docker access in this sandbox to build the
  image (a `git clone` + dependency install + a real GHDB scrape at
  build time, likely a multi-minute build) or run it locally; live
  verification, including confirming the blocking/CAPTCHA behavior in
  practice, is pending install.

- Fixed a real bug in `pagodo` found on its very first live test
  (a bounded `max_dorks=1` run against `scanme.nmap.org`): every
  invocation crashed immediately, before making a single request, with
  `OSError: [Errno 30] Read-only file system:
  'yagooglesearch.py.log'`. `yagooglesearch` (pagodo's own scraping
  library, not pagodo itself) opens a log file at import time using a
  bare relative filename - this plugin's wrapper ran the real
  `pagodo.py` subprocess with its working directory set to `/opt/pagodo`
  (the read-only image layer), so that relative path resolved
  somewhere non-writable. Every path this plugin actually gives pagodo
  (`-g`/`-o`/`-s`/`-z`) is already absolute, so the fix was simply
  running the subprocess from `/tmp` instead (already tmpfs-mounted and
  writable) - no other behavior depends on the working directory.
  `github-code-search`, `brave-dork-search`, and `dehashed` all built
  and installed cleanly with no equivalent issue.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean. Live re-verification (an actual bounded pagodo
  run completing successfully) pending redeploy.

- Broadened `SKILL.md`'s existing Phase 4 guidance on request: it
  already covered fetching a dependency manifest/lockfile *when one was
  already known to be reachable at a URL on the target itself* and
  chaining it into an offline dependency-vulnerability scanner - now
  triggers on any phase identifying specific open-source software (name
  + version, not just a generic fingerprint), fetching that software's
  real manifest/lockfile from its own upstream public source at the
  matching released version, not only from the target directly. Also
  added checking that same software's own issue tracker for existing
  reports matching the discovered version - a real, already-reported
  vulnerability is a stronger, faster finding than waiting on a
  template-based scanner - noting that a plain URL fetch can't parse a
  JS-rendered issue-search results page, so a JSON API endpoint (where
  the hosting platform offers one, e.g. GitHub's REST issue-search API)
  should be preferred when available. Written generically per the
  standing plugin-agnostic-methodology rule - no plugin names added
  (checked with `grep`) - describing roles only ("a URL-content-fetching
  plugin", "an offline dependency-vulnerability scanner"), matching how
  every other Phase already reads.
  Documentation-only change, no build/test impact beyond confirming
  nothing else broke.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean; `grep` for every currently-installed plugin name
  against `SKILL.md` confirms none leaked in.
