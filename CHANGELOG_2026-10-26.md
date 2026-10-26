# Changelog — 2026-10-26

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
