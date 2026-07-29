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
