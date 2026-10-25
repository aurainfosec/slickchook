# Changelog — 2026-10-25

- Fixed `write_artifact`'s dangling-symlink bypass, identified as R3 in the
  independent security review (`SECURITY_REVIEW_2026-10-20.md`, not
  committed to this repository). Second increment in that review's own
  priority order (R7 first, this is P2). Spot-checked and confirmed real
  against the live source before fixing, via a direct regression test
  reproducing the planted-dangling-symlink scenario.
  - `write_artifact` only called `fs::symlink_metadata` inside
    `if path.exists()` - which returns `false` for a dangling symlink, so a
    plugin container (the only other writer to a run's `/artifacts` bind
    mount) could plant `stdout.txt`/`stderr.txt` as a symlink to a
    nonexistent path outside the run directory, skip the check entirely,
    and have the subsequent `create(true).truncate(true)` open follow it
    and create the destination as the slickchook service user. Even for an
    already-existing regular file, the check and the open were separate
    operations - a window for a concurrent writer to swap it for a symlink
    in between.
  - Fixed by replacing the whole check-then-open sequence with one atomic
    `create_new(true)` open (`O_CREAT|O_EXCL`): `stdout.txt`/`stderr.txt`
    always land in a brand-new artifact directory, so anything already at
    that path - symlink, dangling or not, regular file, FIFO - means it was
    planted, and `create_new` now refuses it outright in a single
    indivisible syscall rather than leaving any check-then-act window at
    all.
  - Corrected SC-006 in `SECURITY_CONSIDERATIONS.md`, which had claimed
    this was already mitigated - the review demonstrated that claim was
    stronger than the code supported. New SC-022 documents the real finding
    and fix.
  - Version bumped to 0.3.4 (compiled Rust change - needs the full `.deb`
    rebuild/reinstall path, not a plugin-only install).
  - Verified: `cargo build --locked --release --bins` and `cargo test --lib`
    (15/15 - three new tests, including a direct regression test for the
    exact dangling-symlink scenario that confirms both the original bug's
    mechanism and that the fix closes it) clean.

- Bounded several previously-unbounded host-side resource exposures
  identified as R7 in an independent security review of this codebase
  (`SECURITY_REVIEW_2026-10-20.md`, not committed to this repository -
  reviewed a different, git-less snapshot of the source). Working through
  the review's own findings in its stated priority order; this is the first
  (P1) increment. Spot-checked the review's claims against the current
  source before trusting them - e.g. confirmed `slickchook-runner`'s
  `docker run` invocation really did use `Command::output()`'s unbounded
  buffering before changing it.
  - `slickchook-runner` now captures a plugin container's stdout/stderr
    through a bounded, fully-draining reader
    (`SLICKCHOOK_MAX_PLUGIN_OUTPUT_BYTES`, default 256 MiB per stream)
    instead of `Command::output()`. The stream is still drained to
    completion past the cap (so a chatty container never blocks on a full
    pipe) - bytes beyond the cap are discarded, and a visible
    `[slickchook runner: output truncated...]` marker is appended so
    truncated output never looks like a complete, unremarkable result.
  - `httprequest` gained curl's `--max-filesize 104857600` (100 MiB) and
    `webfetch` gained wget's `--quota=100m` - tool-level caps on the two
    concrete target-to-unbounded-file routes the review named
    (`/artifacts/response_body`, `/input/index.html`).
  - `PluginManager` gained a global concurrency limiter
    (`tokio::sync::Semaphore`, `SLICKCHOOK_MAX_CONCURRENT_RUNS`, default 8)
    held for a run's full duration, shared by `run_plugin` and
    `start_plugin` alike.
  - `list_artifacts` refuses to walk past 50,000 entries in one run's
    artifact directory - a hard safety backstop, not a tuning knob.
  - Each run's total artifact bytes are measured after completion and
    flagged (`artifact_bytes`/`artifact_oversized` on the result,
    `SLICKCHOOK_MAX_ARTIFACT_BYTES`, default 1 GiB) - flag-only, never
    deleted, per explicit instruction not to risk destroying a legitimately
    large run's output. A real preventive disk quota needs filesystem-level
    support this application can't provide on its own (e.g. an XFS project
    quota on the artifacts volume).
  - `start_plugin`'s async job map now evicts finished jobs after
    `SLICKCHOOK_JOB_RETENTION_SECONDS` (default 3600s) if a caller never
    polls them with `plugin_status`.
  - While restructuring that same job bookkeeping for eviction, also fixed
    the review's R9: the task was spawned before its map entry existed, so
    a very fast completion could finish and look itself up before being
    inserted, silently discard its own result, and leave a
    permanently-"running" phantom entry behind. Each job's state now lives
    behind its own `Arc<Mutex<JobState>>` handed to its task before that
    task is spawned, so timing can no longer cause a dropped result.
  - All new defaults are generous and operator-overridable via environment
    variables (see `RUNNER.md`'s new "Host-side resource bounds" section)
    rather than hardcoded, per explicit instruction to not break
    legitimate plugin functionality while still bounding genuine abuse.
  - New SC-021 in `SECURITY_CONSIDERATIONS.md` documents the full finding
    and fix; bumped "Last reviewed" to today.
  - Version bumped to 0.3.3 (compiled Rust change - needs the full `.deb`
    rebuild/reinstall path per `RUNNER.md`, not a plugin-only
    `slickchook-plugin install`).
  - Verified: `cargo build --locked --release --bins` and `cargo test --lib`
    (12/12) clean; new `cargo test --bin slickchook-runner` (5/5) exercises
    the bounded reader's truncation/non-truncation/exact-boundary behavior
    directly; `cargo test --bin slickchook` (2/2) confirms the job
    bookkeeping change didn't regress tool registration. Not independently
    load-tested against a real excessive-output target in this sandbox (no
    Docker socket access) - needs confirming on the real deployment that a
    deliberately large response actually gets capped end to end.

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
