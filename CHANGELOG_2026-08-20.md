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
