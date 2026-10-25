# Changelog — 2026-10-25

- Added a new `h3fuzz` plugin, per direct request to add a plugin wrapping
  [Assetnote's h3fuzz](https://github.com/assetnote/h3fuzz) ("similar to
  kiterunner and ffuf"). h3fuzz is a content-discovery fuzzer forked from
  ffuf that negotiates HTTP/3 automatically (falling back to HTTP/2, then
  HTTP/1.1) while keeping ffuf's wordlist/matcher/filter flag surface and
  `FUZZ`-keyword model.
  - Confirmed directly from h3fuzz's pinned `v0.1.0` source
    (`pkg/ffuf/optionsparser.go`), not just its README/`-h` text, which
    flags actually exist and what they default to - the manifest's
    `match_codes` default note (`200-299,301,302,307,401,403,405,500`)
    and `rate_limit`'s "0 means unlimited" both come from reading
    `NewConfigOptions()` directly.
  - `target` is an option (`-u`), not a positional argument, since h3fuzz
    (like ffuf) requires the literal `FUZZ` keyword at an explicit
    injection point in the URL, a header, or the request body rather than
    always fuzzing a path segment - documented clearly in the manifest so
    a caller doesn't pass a bare target and get zero results.
  - Bundles the same three small reviewed wordlists already shipped with
    this fleet's `feroxbuster` plugin (duplicated into `plugins/h3fuzz/`,
    since plugin packages are self-contained) rather than h3fuzz's own
    bundled `final.txt` - confirmed by extracting the real `v0.1.0`
    release tarball that `final.txt` is 233,582 lines / ~2.7MB, squarely
    the kind of large, unreviewed wordlist `PLUGINS.md`'s "Wordlists"
    section already explains why SecLists itself is excluded fleet-wide.
  - Deliberately does not expose h3fuzz's acknowledged HTTP/3 high-rate
    ceiling-test policy (`-http3-high-rate`,
    `-http3-high-rate-acknowledge`, `-http3-target-rps`,
    `-http3-max-connections`, `-http3-streams`, `-http3-max-workers`) as
    plugin options. That mode is an explicit, operator-acknowledged
    load-testing feature documented in h3fuzz's own README (100k+ req/s
    local measurements); it's out of scope for a bounded recon plugin and
    - since `run_plugin` only accepts declared options/arguments - isn't
    reachable through this plugin regardless of caller input.
  - Follows the project's established Dockerfile conventions for a
    single-binary Go release tool: `apk --print-arch` for architecture
    detection rather than `ARG TARGETARCH` (see `PLUGIN_WORKFLOW.md`'s
    BuildKit-only-features note), downloading the real pinned `v0.1.0`
    Linux amd64/arm64 release tarball from GitHub Releases - matching the
    same pattern already used by `feroxbuster`.
  - `fixed_args` write structured JSON straight to
    `/artifacts/result.json` (`-of json -o /artifacts/result.json`)
    rather than relying on stdout scraping, plus `-noninteractive -s` to
    keep the run headless and `stdout.txt` clean.
  - Verified: `cargo build --locked --release --bins` and `cargo test
    --lib` (12/12, including the manifest-scanning test that parses
    `plugins/h3fuzz/h3fuzz.toml`) clean; `bash -n` on the Dockerfile's
    architecture-detection `case` statement. Could not build or run the
    actual container image in this sandbox (no `docker` socket access) or
    execute the downloaded `h3fuzz` binary directly (blocked as
    untrusted external code) - real-deployment verification (`list_plugins`
    showing the new plugin, a live `run_plugin` call against an authorized
    target, and `list_run_artifacts`/`read_artifact` retrieving
    `result.json`) is still needed per `PLUGIN_WORKFLOW.md` step 5 once
    this is installed on the deployment host.
