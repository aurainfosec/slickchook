# Changelog — 2026-07-29

- Fixed a panic on every graceful shutdown (or startup-error unwind) when using
  the PostgreSQL storage backend: `PostgresStore` no longer owns a private
  Tokio runtime that could be dropped from inside the server's own async
  context; it now reuses the ambient runtime's `Handle` throughout.
- Removed the SQLite run-store backend and the `SLICKCHOOK_STORAGE_BACKEND`
  selector. PostgreSQL, configured via `DATABASE_URL`, is now the only
  supported backend; the server fails fast with a clear error if it is unset.
- Removed the now-single-implementor `RunStore` trait in favor of calling
  `PostgresStore` directly.
- Removed the `rusqlite` dependency and the sqlite artifact-permission step
  from the Ubuntu installer.
- Updated `README.md`, `RUNNER.md`, `PLUGINS.md`, `SECURITY_CONSIDERATIONS.md`,
  and `ROADMAP.md` to describe PostgreSQL as the required backend instead of
  an opt-in, staged one.
- Storage-layer tests now require a reachable PostgreSQL instance
  (`TEST_POSTGRES_URL`) to exercise any coverage; there is no longer a
  zero-dependency embedded backend to test against locally.
- Added target-scoped artifact retrieval: a new indexed `target` column on
  `plugin_runs` (extracted from each run's declared `target` input) plus
  three MCP tools — `list_runs_by_target`, `list_run_artifacts`, and
  `read_artifact` — so past runs and their artifacts for a given target can
  be looked up later, not just returned inline by the `run_plugin` call that
  produced them. `read_artifact` is bounded to 5 MiB per read, confines reads
  to the requested run's own artifact directory (canonicalized, defeating
  symlink escapes the same way the existing fetch/analyze handoff does),
  labels returned content as untrusted report data, and audit-logs each read.
  Added migration `002_add_run_target.sql`, applied automatically at startup
  and idempotent against an already-populated database.
- Fixed a bug (found while adding the above) where a run that errored before
  or during execution (as opposed to running and exiting non-zero) could
  never be persisted at all: `record_run` wrote `status = 'error'`, but the
  original CHECK constraint only allowed `'running', 'success', 'failed'`,
  so the INSERT silently failed. The same migration widens the constraint to
  include `'error'`.
- Generalized `scripts/migrate-postgres.sh` to apply every
  `migrations/postgres/*.sql` file in order instead of hardcoding
  `001_initial.sql`, so future migrations aren't silently skipped.
- These new tools share the same (currently nonexistent) authentication
  boundary as the rest of the MCP surface — see `SECURITY_CONSIDERATIONS.md`
  SC-012.
