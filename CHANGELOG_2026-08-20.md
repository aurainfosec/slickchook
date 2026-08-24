# Changelog — 2026-08-20

- Confirmed `github-code-search` live end-to-end after `GH_TOKEN` was
  configured: first attempt returned `HTTP 401: Bad credentials` from
  the real GitHub API - confirming the plugin's request construction
  and network wiring were already correct, the issue was purely an
  invalid token value in `/etc/slickchook/plugin-secrets.env`. After
  the operator fixed the token and restarted the service, a real
  authenticated search against `aurainfosec/slickchook` for `TODO`
  succeeded (exit code 0, clean JSON `[]` - no matches, as expected).
  No code change; verification-only.

- Added a new `sourcemap-scanner` plugin - discovers, fetches,
  reconstructs, and analyzes JavaScript source maps for one authorized
  target (a page URL, a direct `.js` URL, or a direct `.map` URL),
  per a direct request. Self-contained (a stdlib-only Python tool)
  rather than wrapping a third-party tool (`denandz/sourcemapper` was
  considered) - initial research into that tool was started but
  stopped by the operator before completing, and a follow-up request
  clarified the actual want was a plugin that itself searches for and
  analyzes source maps, with the MCP client deciding when to feed
  results into other plugins - building it directly gave full,
  verifiable control over the one part that mattered most: the
  path-traversal safety of reconstructing attacker-influenced
  `sources` array entries as real filesystem paths, which was flagged
  as an open, unverified question for the third-party tool and would
  otherwise have needed trusting its handling of that on faith.
  Every discovered source path is defused before being used as a
  filesystem path: any devtools-only pseudo-scheme prefix
  (`webpack://`, `ng://`, etc) is stripped, every `..`/empty segment is
  dropped, and the result is re-joined as strictly relative with a
  final canonicalize-and-containment check before any write. Tested
  directly against a battery of real path-traversal attempts
  (`../../../etc/passwd`, `webpack:///../../../etc/passwd`,
  `/etc/passwd`, deeply-nested `../` chains, a bare `..`, Windows-style
  paths) before shipping, not assumed safe - every case stayed
  contained or was rejected outright. Also directly exercised the
  HTML `<script src>` discovery, the `sourceMappingURL` comment
  regex (including the inline base64 `data:` URI form), and a full
  `sourcesContent` reconstruction + built-in pattern scan end-to-end
  locally (correctly flagged a planted fake AWS key), rather than only
  reasoning about the code.
  Runs a lightweight built-in scan against whatever text/paths are
  found for common sensitive patterns (AWS/Google API keys, Slack
  tokens, JWTs, private key headers, generic credential-looking
  assignments, internal-looking hostnames/IPs, absolute local
  filesystem paths revealing a real developer username/machine),
  written to `/artifacts/findings.json` - useful standalone, not only
  as a feeder for something else.
  Per the integration this was explicitly requested for: where
  `sourcesContent` was actually present, the real reconstructed source
  is written under this run's own `/input` staging directory (not just
  `/artifacts`) - `input_read_only = false`, matching `webfetch`'s own
  precedent for a plugin that needs to populate its own input dir -
  specifically so its `run_id` can be chained as `input_run_id` into
  `trufflehog` (already installed) for a deeper filesystem-based
  secret scan than this plugin's own regex pass, without this plugin
  needing to invoke that itself. Confirmed the actual mechanics of
  `input_run_id` chaining directly from `src/plugin_manager.rs`/
  `src/storage.rs` before designing around it: a chained run's `/input`
  mount is literally the *prior* run's own `/input` staging directory on
  disk (`resolve_completed_input` resolves to `<prior_artifact_dir>/input`),
  not its regular output artifacts - so a plugin meant to be chained
  from must write its useful output into its own `/input`, exactly
  mirroring `webfetch`'s existing convention, not a new pattern.
  Updated `SKILL.md`'s Phase 4 with a new, plugin-agnostic bullet
  describing this same chaining generically (checked with `grep` - no
  plugin names added).
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean; the path-sanitization and end-to-end source-map
  processing logic were both exercised directly with real adversarial
  and real-shaped inputs locally (see above), not just read for
  correctness. No docker access in this sandbox to build/run the image
  locally; live verification (a real page with an actual source map)
  pending install.

- Live-verified `sourcemap-scanner` after install with a bounded smoke
  test against `scanme.nmap.org` - exit code 0, correct real network
  fetch and argv construction, and the correct clean "no source maps
  discovered" exit path (expected: it's a bare static test page with
  no JS at all). Confirms container/network/argument mechanics work
  end-to-end; the "a map was actually found and reconstructed" code
  path still needs a real target that ships one, deferred until a
  specific authorized target is named for that.

- Added a new `saas-enum` plugin - checks whether an authorized
  organization has a live presence on common enterprise SaaS and
  e-commerce platforms via their predictable branded vanity-subdomain
  patterns (`{org}.slack.com` and similar), revealing what
  helpdesk/HR/CRM/collaboration/storefront systems an org actually
  runs. Research found no existing well-established tool does this
  (only generic OSINT roundups, not a maintained multi-platform
  checker) - built from scratch, same shape as `alterx`/`pagodo`.
  Every included platform's exists-vs-not-found signal was verified
  live before shipping, not assumed from a commonly-repeated pattern -
  this caught real problems: several plausible-looking assumed
  patterns for other platforms (okta, freshdesk, greenhouse, lever,
  bamboohr, docusign, workday, asana, bigcommerce, squarespace) turned
  out to give no signal at all, use a completely different URL shape
  (greenhouse/lever are a shared-domain path, not a per-org subdomain),
  or simply didn't resolve for any real store/org tried - all
  deliberately excluded and documented as such, rather than shipped as
  a check that always says "maybe".
  Included platforms, each confirmed with at least one real
  organization's vanity URL against an obviously-nonexistent control
  during development: `zendesk` (shopify), `atlassian` (spotify,
  netflix, airbnb), `slack` (slack), `pagerduty` (shopify, github,
  gitlab, netflix), `monday` (monday), `zoom` (salesforce, netflix),
  `sharepoint` (microsoft, netflix), `salesforce` (zoom), `gitbook`
  (gitbook), and `shopify` (allbirds, gymshark).
  Found and fixed one real bug caught only by this live testing, not
  by reasoning about the code: `monday`'s check initially assumed a
  real org always returns HTTP 406, but a live real-org test came back
  200 with a legitimate login-flow redirect instead - the actually
  reliable signal is purely whether the final URL names
  "slug_not_found", regardless of status code; fixed before shipping.
  `gitbook`'s result is explicitly flagged with an attribution-risk
  note - unlike every other platform checked here, gitbook lets any org
  name be claimed by anyone, so a hit there doesn't by itself confirm
  the claiming org is the one actually being investigated.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean. No docker access in this sandbox to build/run
  the image locally; live verification through the actual MCP server
  pending install (though every check function was already directly
  exercised against real platforms and real organizations locally
  during development, not just read for correctness).

- Added a new `cve-poc-research` plugin - looks up known CVEs, and
  critically real pointers to known public PoC exploit code, for a
  product/vendor discovered via a live version fingerprint (a service
  banner, an HTTP tech-detection hit) rather than a dependency
  manifest/lockfile, which the already-installed `osvscanner` plugin
  covers instead. Prompted directly by external feedback naming this
  exact gap: "version fingerprints piped to cve-poc-research;
  slickchook's osvscanner covers dependency CVEs only".
  Wraps `vulnx` (ProjectDiscovery, formerly `cvemap`) - confirmed the
  real, current v2.0.2 CLI directly by downloading its actual Linux
  release binary and testing it live (no docker access in this
  sandbox), rather than trusting docs alone, per this week's standing
  rule. This caught a real, meaningful mismatch: research had
  suggested top-level flags like `--product`/`--poc`; the real v2.0.2
  CLI is subcommand-based (`search`/`id`/`auth`/`filters`/...) with
  those as flags to `vulnx search` specifically - and its official
  Docker Hub image is stale (only `v1.0.0`/`latest`, last pushed
  2025-07-27, predating this entire subcommand restructure) - built
  from source pinned to the real `v2.0.2` git tag instead, confirming
  the exact go module path (`github.com/projectdiscovery/vulnx/v2`)
  directly from the repo's own `go.mod` rather than guessing.
  Confirmed live, directly against the real API, both open questions
  research had flagged as unresolved: (1) `pocs[].url` is real,
  populated data (actual GitHub PoC-repo URLs), not just a boolean -
  confirmed with a real result; (2) there is no version-level field
  anywhere in the schema at all (checked the full JSON shape of a real
  result, not just the documented filter list) - meaning this tool can
  only filter to a product/vendor name, never an exact version, so a
  result set is every CVE ever recorded against that product, not
  narrowed to what a specific fingerprinted version actually has -
  documented plainly in the manifest as a real, load-bearing
  limitation rather than implied to work more precisely than it does.
  Also confirmed live: works unauthenticated at 10 requests/minute
  despite `vulnx auth --help`'s own text claiming authentication is
  "required for all vulnx commands" (another docs-vs-real-behavior
  mismatch caught by testing, not reading); `PDCP_API_KEY` is read
  directly from the environment with no separate init/login step
  needed (confirmed via the binary's own embedded strings); and a
  boolean-style flag like `--poc`/`--kev` given as a bare flag (no
  `=true`, matching this fleet's own convention of only emitting the
  flag token when a boolean option is true) correctly AND-filters
  results exactly as expected even positioned before other flags, so
  no special wrapper was needed for that.
  No `$HOME` dependency found either (tested the real binary with
  `HOME` fully unset, not just set-to-invalid) - no wrapper Dockerfile
  needed for that lesson this time.
  Updated `SKILL.md`'s Phase 4 with a new, plugin-agnostic paragraph
  describing this live-fingerprint-driven role generically, explicitly
  distinguishing it from the existing dependency-manifest-driven
  paragraph (which deliberately excludes a "generic web server
  fingerprint" - exactly the case this new plugin covers) - including
  the version-matching-limitation caveat generalized so future
  sessions apply the same diligence even if this specific tool is ever
  swapped out. Checked with `grep` - no plugin/tool names added.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean (the `scan_plugins`-based tests exercise every
  manifest under `plugins/`, including this one, and passed). The
  underlying `vulnx` binary itself was extensively live-tested against
  the real API before writing the manifest (see above); the container
  build itself (a multi-stage Go build) is still untested, since this
  sandbox has no docker access.

- Live-verified `cve-poc-research` after install: a real search for
  `product=nginx` with `kev=true`/`poc=true` (both passed as bare
  boolean flags, matching this fleet's own convention) correctly
  returned exactly the 2 CVEs matching both filters, exit code 0 -
  confirming the bare-flag-AND-filtering behavior already confirmed
  locally before merging also holds true through the actual deployed
  container end-to-end.

- Added a new `url-export` plugin, per a direct request for "a way to
  create a list of identified domains and paths that can later be
  imported into something like burpsuite or similar." Deliberately
  scoped to export-only for this pass - the actual Burp-import half of
  that request is being verified separately (real Burp Suite import
  mechanics haven't been confirmed yet, so nothing about that is
  written down as fact until they are).
  Extracts, normalizes, and deduplicates URLs/domains/paths out of an
  arbitrary blob of already-gathered recon text into one clean,
  plain-text URL list - a self-contained stdlib-only Python tool,
  deliberately not tied to any one source plugin's own output format
  (every plugin in this fleet has a different shape: a plain URL list,
  a JSON array, bare hostnames with no scheme, etc) - hardcoding
  per-plugin parsing here would duplicate knowledge the calling AI
  already has about each plugin's own declared output, so this is a
  generic extract-anything-URL-shaped-and-clean-it-up pass instead.
  `default_scheme` additionally treats bare hostname-only lines (e.g.
  from a subdomain-enumeration-only source) as URLs.
  `include_only_domain` filters the result to a given domain (or its
  subdomains) - keeps the exported list properly scoped even if the
  raw pasted-in text happened to contain an incidental out-of-scope
  URL (e.g. a citation/reference link), matching this fleet's own
  scope-discipline expectations for anything leaving a run.
  Verified directly, not just read for correctness: tested the real
  extraction/normalization logic against a deliberately messy sample
  mixing a plain URL list, a JSON array, bare hostnames, trailing
  comma/quote punctuation, mixed case, and an intentional duplicate -
  confirmed correct deduplication, punctuation stripping, case-
  insensitive scope matching, and correct rejection of a
  non-domain-looking token; also ran the actual CLI end-to-end
  (argument parsing through to both `/artifacts/urls.txt` and
  `/artifacts/urls.json` output) with the `default_scheme` and
  `include_only_domain` options combined, not just the extraction
  function in isolation.
  Added a short, plugin-agnostic mention to `SKILL.md`'s §5 describing
  this export role generically as a separate, smaller step from the
  full-archive collection process already documented there - checked
  with `grep`, no plugin/tool names added.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean. No docker access in this sandbox to build/run
  the image locally; live verification through the actual MCP server
  pending install.

- Live-verified `url-export` after install: a real run against a
  deliberately messy mixed sample (a plain URL, a bare hostname, a
  JSON array, a duplicate, and an out-of-scope citation URL) correctly
  deduplicated, added the `https://` scheme to the bare hostname, and
  dropped the out-of-scope URL - exit code 0, both `urls.txt`/
  `urls.json` landed correctly under `/artifacts/`, matching exactly
  what local testing had already confirmed before merging.

- Corrected `url-export`'s manifest description, which had asserted
  (from memory, explicitly flagged at the time as unverified and
  pending separate research) that "Burp doesn't have a simple native
  bulk-URL-list import." Real research against PortSwigger's own
  documentation shows that claim was wrong: Burp Suite's Target >
  Scope > Advanced scope control has a native **Load** button that
  imports a plain-text file in exactly the one-URL-per-line format
  this plugin already produces, directly into scope - no edition
  restriction noted. Documented the real, verified mechanics in
  `RUNNER.md` (a new "Importing exported URLs into Burp Suite"
  section): the Load button populates *scope* (the allow-list filter),
  not the *Site Map* itself, since the Site Map only fills in as Burp
  actually observes real traffic; the BApp Store extension **Add to
  SiteMap+** (confirmed real and currently listed) is the option when
  Site Map population specifically is wanted instead; and a
  curl-through-the-proxy replay is documented as a universal fallback
  that works with any intercepting proxy, not just Burp, with the
  practical caveats about `-k`/Burp's CA cert and the Intercept toggle
  not needing to be off. This is a correction to a explicitly-flagged
  unverified claim, not a silent edit - the original wording was
  written knowing it was unconfirmed, and is now replaced with what
  was actually confirmed.
  Documentation-only change, no build/test impact beyond confirming
  nothing else broke.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean.

- Added a new `cloud-storage-enum` plugin - checks for AWS S3 buckets,
  Google Cloud Storage buckets, and Azure Blob Storage containers
  belonging to an authorized organization, given a brand keyword.
  Prompted by a direct follow-up request after recommending it as the
  strongest remaining fleet gap - distinct from the already-installed
  `saas-enum` (which only checks enterprise SaaS vanity domains, not
  raw cloud storage).
  Confirmed `initstring/cloud_enum` (MIT) is real and actively
  maintained (last pushed 2026-07-09) before building anything - it
  already implements exactly this, including an S3 rate-limit
  ('Slow Down') breakout already built in. Deliberately doesn't wrap
  its full CLI, though: its own `run_all()` per provider also probes
  several unrelated non-storage services (Firebase RTDB, App Engine,
  Cloud Functions, AWS Apps, Azure VMs/websites/databases/Key Vault)
  with no flag to scope those out - out of the requested storage-only
  scope, and more requests against a target's cloud footprint than
  asked for. Instead, imports and calls its own real
  `check_s3_buckets`/`check_gcp_buckets`/`check_storage_accounts`/
  `brute_force_containers` functions directly as a library, confirmed
  by reading the exact source at a pinned commit
  (`d3dba3716e10262f0c18a30f381a5eb617d7afd5`) - both the latest git
  tag (`v0.2`) and the PyPI-published package (`0.0.1`) are stale
  relative to the real current source's own `pyproject.toml` version
  (`0.8`), confirmed live rather than assumed, so pinned by commit
  instead of either.
  Confirmed real, per-provider detection signals directly from the
  library's own response-handling logic (not assumed to be uniform
  across providers): S3 and GCS both give a clean three-state signal
  (404 not-found, 403 exists-but-not-listable, 200 exists-and-listable)
  - including the important nuance that 403 only means "not listable,"
  not necessarily fully private, since S3/GCS separate object-read
  permission from bucket-list permission. Azure is genuinely messier -
  confirmed no equivalently clean "exists but private" signal for
  containers specifically - documented as a real, honest limitation
  rather than implied to work as cleanly as S3/GCS.
  `fuzz.txt` (the library's own real, bundled 306-entry mutation/
  brute-force wordlist - not invented for this plugin) isn't declared
  as package data anywhere upstream (no `MANIFEST.in`, no
  `package_data`/`include-package-data` in `pyproject.toml`) -
  confirmed it wouldn't survive a plain `pip install`, so fetched
  separately at the same pinned commit and committed into this
  plugin's own directory, declared via `[[assets]]` matching
  `feroxbuster`'s existing bundled-wordlist convention (which also
  gets real install-time verification, not just documentation).
  `quickscan` skips all ~1800 keyword+mutation combinations for a fast
  bare-keyword-only sanity check first. Set `timeout_seconds` to 1800
  given the realistic default-run request volume across three
  providers plus any found Azure accounts' own container
  brute-forcing.
  No `SKILL.md` change needed - same reasoning as `saas-enum`, which
  this plugin closely parallels in shape (queries third-party cloud
  infrastructure using target-derived candidate names, not the
  target's own infrastructure directly) - Phase 1's existing generic
  wording already covers it.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean (the `scan_plugins`-based tests exercise every
  manifest under `plugins/`, including this one's `[[assets]]`
  declaration, and passed). Every relevant function's exact real logic
  (status-code branches, mutation combinatorics, brute-force scoping)
  was read directly from the pinned commit's source before designing
  around it. Full runtime verification (the actual Python import chain
  and a real HTTP check) couldn't be exercised in this sandbox - no
  `pip` available at all here (a first for this session; every other
  plugin so far could at least be partially tested locally), so this
  one relies more heavily on source-level verification than most;
  container build and live verification are both still pending
  install.
