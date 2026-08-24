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
