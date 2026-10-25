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
    untrusted external code).

- Fixed `h3fuzz`'s `autocalibrate` (`-ac`) option silently doing nothing,
  found via live deployment verification (merged PR #144, then installed
  and run for real against an authorized target) rather than assumed
  working from the manifest alone. `run_plugin` succeeded (`exit_code:
  0`) but `stderr` showed: `Skipping strategy "basic" because of error:
  open /.config/h3fuzz/autocalibration/basic.json: no such file or
  directory`.
  - Root cause confirmed directly from h3fuzz's own `v0.1.0` source
    (`pkg/ffuf/util.go`'s `CheckOrCreateConfigDir`/`ReadDefaultConfig`,
    `pkg/ffuf/constants.go`'s `CONFIGDIR = filepath.Join(xdg.ConfigHome,
    "h3fuzz")`): with `$HOME` unset (this fleet's usual unprivileged
    container UID has no matching `/etc/passwd` entry), `xdg.ConfigHome`
    resolves to the bare `/.config`, which isn't writable by that UID;
    `mkdir` fails, and `ReadDefaultConfig` explicitly swallows that error
    (`// Try to create configuration directory, ignore the potential
    error`). Autocalibration then silently has no strategy file to run
    instead of erroring loudly - the exact same `$HOME`-unset failure
    class already documented for this fleet's `alterx` plugin, just a
    different symptom (quiet no-op instead of a startup panic).
  - Fix: added `ENV HOME=/tmp` to the plugin's `Dockerfile` final stage
    (the same fix already used by `alterx` and `feroxbuster`, and the
    manifest's own `docker.tmpfs` already provides a writable `/tmp`).
    Documented the mechanism directly in `h3fuzz.toml`'s description so
    a future reader doesn't have to rediscover it.
  - Verified: `cargo build --locked --release --bins` and `cargo test
    --lib` (12/12) clean. Could not rebuild the image in this sandbox (no
    `docker` socket access); needs reinstalling
    (`sudo slickchook-plugin install plugins/h3fuzz` followed by
    `sudo systemctl restart slickchook.service`) and a repeat live
    `run_plugin` call with `autocalibrate=true` to confirm the strategy
    file now loads instead of being skipped.
