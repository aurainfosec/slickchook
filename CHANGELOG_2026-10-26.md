# Changelog — 2026-10-26

- Fixed `PluginManager::cancel_run` (yesterday's R6 fix, PR #149) not
  actually stopping the real container, caught by live-verifying that fix
  on the real deployment rather than trusting the database result alone.
  - Cancelled a full-port-range `nmap` scan 3 seconds in: `list_runs_by_target`
    correctly showed `status: "cancelled"`, a real `finished_at`, and
    `exit_code: null` - the database side worked exactly as designed. But
    `docker ps` on the host showed the container still `Up` regardless.
  - Root cause: `cancel_run` called `Command::new("docker")` directly from
    the *main service process*, which - per this project's own documented
    privilege architecture - is deliberately never added to the `docker`
    group; only the setgid `slickchook-runner` binary can reach the Docker
    socket at all. The `docker kill` call failed with a permission error
    that `cancel_run`'s own best-effort error handling logged at `debug!`
    as an assumed-harmless "container may have already exited" - exactly
    the ambiguity that log line's own comment worried about, just
    triggered by the wrong cause.
  - Fixed by routing the kill through a new `slickchook-runner
    --kill-container <name>` mode instead of calling `docker` directly
    from the service process. The mode validates the name matches the
    exact shape `cancel_run` constructs (`slickchook-<run_id>`) before
    running `docker kill`, the same defense-in-depth posture
    `validate_runner_paths` already applies to
    `--manifest`/`--artifact-dir`.
  - Addendum added to SC-024 documenting the real finding and fix, rather
    than silently correcting the entry - the database-only verification
    gap is itself worth remembering, not just the bug it masked.
  - Version bumped to 0.3.7 (compiled Rust change - needs the full `.deb`
    rebuild/reinstall path).
  - Verified: `cargo build --locked --release --bins` and `cargo test --lib`
    (17/17), `cargo test --bin slickchook` (2/2), and `cargo test --bin
    slickchook-runner` (6/6, one new) clean. Still needs the same live
    re-check that caught the original bug: cancel another real running
    container and confirm via `docker ps` it's actually gone this time.

- Fixed cancellation not actually stopping a plugin container, and a
  cancelled or crashed run leaving no database record at all - identified
  as R6 in the independent security review (`SECURITY_REVIEW_2026-10-20.md`,
  not committed to this repository). Fourth increment in that review's own
  priority order (R7, R3, R5 already landed; this is P2).
  - `plugin_cancel` only called `AbortHandle::abort()` on the Rust task
    `start_plugin` spawned - that stops this process from waiting on the
    result, but the runner process and its real Docker container are a
    separate OS process tree entirely, untouched by aborting the task. The
    container kept running until its own independent
    `docker.timeout_seconds` elapsed regardless of the cancel request.
  - `run_plugin_with_input` only called `record_run` once, at the very
    end - a cancelled job's task (which never reaches that point) or a
    crash left its artifact directory on disk with no database row at all,
    invisible to `list_runs_by_target`/`purge_runs_by_target`.
  - Separately, `slickchook-runner`'s own timeout-exceeded `docker kill`
    had no timeout of its own and discarded its result - a hung `dockerd`
    could make the whole runner process hang indefinitely waiting on it,
    and a failed kill left nothing in the log explaining why the container
    kept running.
  - Fixed with a new migration (`003_cancellable_runs.sql`, applied
    automatically on next service start) making `finished_at` nullable and
    widening the status `CHECK` constraint to include `cancelled`.
    `run_plugin_with_input_and_run_id_hook` (what `run_plugin_with_input`
    now delegates to) records a `status: "running"` row the moment a run's
    `run_id` is generated, before the runner is even invoked, and fires an
    `on_run_id` hook synchronously at the same point. `start_plugin` uses
    that hook to learn the `run_id` immediately and store it in the job's
    shared state.
  - `plugin_cancel` now also calls the new `PluginManager::cancel_run`:
    sends `docker kill` directly to the run's deterministic container name
    (`slickchook-<run_id>`) and marks the row `cancelled` via
    `PostgresStore::mark_run_cancelled`, which only updates a row still
    `status = 'running'` - a job that happened to finish naturally right
    before cancellation landed keeps its real result instead of being
    overwritten.
  - `slickchook-runner`'s own timeout-exceeded `docker kill` is now bounded
    (10s) with its result actually checked and logged.
  - New SC-024 in `SECURITY_CONSIDERATIONS.md` documents the full finding
    and fix.
  - Version bumped to 0.3.6 (compiled Rust change plus a new database
    migration - needs the full `.deb` rebuild/reinstall path; the
    migration applies automatically the next time the service starts,
    no manual step needed).
  - Verified: `cargo build --locked --release --bins` and `cargo test --lib`
    (17/17, one new test exercising both the running→cancelled transition
    and the guard against overwriting an already-finished run) clean;
    `cargo test --bin slickchook` (2/2) and `cargo test --bin
    slickchook-runner` (5/5) unaffected. The new Postgres-backed test could
    not actually execute in this sandbox (no `TEST_POSTGRES_URL`/database
    available, same limitation as this project's existing
    `postgres_store_*` tests) - it compiles and documents the intended
    behavior but needs confirming against the real deployment's database.
    Also not independently exercised: a live `start_plugin` →
    `plugin_cancel` call against a real long-running container, confirming
    `docker ps` actually shows it stopped promptly.
