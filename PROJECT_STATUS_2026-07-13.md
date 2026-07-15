# Slickchook Project Status

**Date:** 2026-07-13

## Project summary

Slickchook is an early but buildable Rust prototype for running security and reconnaissance tools as Docker-based plugins through an MCP server.

The code compiles successfully, and all four existing unit tests pass. However, the repository has no commits yet: every project file is currently untracked on the `master` branch.

## Currently implemented

- HTTP MCP server using SSE, bound to `127.0.0.1:8080`.
- Three MCP tools:
  - `health`
  - `list_plugins`
  - `run_plugin`
- TOML-based plugin discovery under `plugins/*/*.toml`.
- Plugin inputs supporting:
  - Named options and CLI flags
  - Positional arguments
  - Required and default values
  - Rejection of undeclared input fields
- Separate `slickchook-runner` binary for privilege-separated Docker access.
- Restricted container execution:
  - Read-only root filesystem
  - Non-root container user
  - All capabilities dropped by default
  - Optional `NET_RAW` capability only
  - `no-new-privileges`
  - CPU, memory, process, network, tmpfs, and timeout controls
  - Only the current artifact directory mounted writable
- Path validation preventing manifests and artifact directories from escaping their configured roots.
- Per-run output capture:
  - Full stdout and stderr files
  - Bounded 64 KiB previews in MCP results
  - Additional files created by the plugin
- SQLite run recording in `artifacts/plugin_runs.sqlite3`.
- Two plugin manifests:
  - `testssl`: a real TLS scanning plugin producing JSON and HTML reports.
  - `example`: a documentation/sample manifest.
- Deployment guidance for running the MCP server as an unprivileged service account while granting Docker access only to the runner.

## Still needs to be enabled or completed

- **Docker execution is not currently usable by this account.** Docker is installed, but access to `/var/run/docker.sock` is denied. The setgid runner/service-account setup documented in `RUNNER.md` must be applied before plugins can run.
- **The `example` plugin is not functional.** Its image, `example/tool:latest`, is only a placeholder.
- **The `testssl` image must be available.** Docker needs registry/network access to pull `drwetter/testssl.sh:latest`, unless it is preloaded.
- **Production deployment is not installed.** There is no systemd unit or deployment automation in the repository; the documentation only provides a partial service configuration.
- **Configuration is hard-coded.** Bind address, port, plugin directory, artifact directory, and database path cannot currently be set through CLI arguments or environment variables. Only the runner path supports an environment override.
- **No authentication or transport security is implemented.** This is less urgent while bound to localhost, but must be added or provided by a trusted reverse proxy before exposing the server remotely.
- **Plugin reload exists internally but is not exposed.** Manifests are loaded at startup; changes require restarting the server unless a reload MCP tool or file watcher is added.
- **Run history has no read API.** Runs are inserted into SQLite, but there is no MCP tool for listing, inspecting, or deleting previous runs.
- **Artifact retrieval is path-based only.** The response returns filesystem paths; there is no MCP resource or HTTP endpoint to fetch artifact contents.
- **Testing is limited.** Argument construction and a few validation rules are covered, but the MCP transport, runner CLI, path security, SQLite recording, timeout behavior, artifact handling, and real Docker execution lack automated tests.
- **Operational polish is missing.** There is no root README, CI configuration, logging configuration, graceful run cancellation, retention policy, database migration mechanism, or pinned container image digest.

## Overall assessment

The core execution architecture is present and passes its current tests. The main blocker to an end-to-end working deployment is enabling controlled Docker access for `slickchook-runner`. After that, `testssl` should be the first genuinely usable plugin.
