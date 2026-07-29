# slickchook

Slickchook is an MCP (Model Context Protocol) server that lets an AI agent
drive real security and reconnaissance tools — nmap, testssl, Nuclei,
Subfinder, Feroxbuster, OSV-Scanner, gowitness, Kiterunner, websocat, a
GraphQL introspection checker, and more — against authorized targets, with
every invocation sandboxed, validated, and logged. It is a controlled
execution layer between an MCP client and a fleet of Dockerized pentest
tools, not a general-purpose shell.

## Architecture

**Two-binary privilege split.** `slickchook` is the long-running,
unprivileged MCP server (Rust, [`rmcp`](https://crates.io/crates/rmcp) over
SSE) and never touches Docker directly. `slickchook-runner` is a separate
binary installed `root:docker` with the setgid bit set — the only process
with Docker socket access. The server invokes it as a subprocess per run
with structured argv, never a shell, so there is no command-injection
surface between the two. See [`RUNNER.md`](RUNNER.md) for the deployment
model and enforcement details.

**Plugins are data, not code.** Each plugin is a TOML manifest declaring a
container image, fixed/default arguments, explicitly declared `options` and
`arguments` (anything undeclared is rejected), and a Docker policy: network
(`none`/`bridge` only), memory/CPU/pids/timeout limits, tmpfs mounts, and an
allow-listed capability (`[]` or `NET_RAW`). The runner re-validates this
manifest independently before touching Docker and enforces a read-only root
filesystem, non-root user, `--cap-drop ALL`, and `no-new-privileges` on
every container. Plugin packages are installed and versioned independently
of the core server — see [`PLUGINS.md`](PLUGINS.md).

**Per-run isolation and artifacts.** Every `run_plugin` call gets a UUID,
its own `artifacts/<run_id>/` directory (the only writable mount inside the
container), and captured `stdout.txt`/`stderr.txt` plus any files the tool
wrote. A read-only `/input` staging mount lets one run's output feed a
later run via `input_run_id`, confined and canonicalized to the artifact
root — the basis of fetch-then-analyze pipelines (e.g. `screensanitize`,
`osvscanner`).

**MCP surface:** `run_plugin` (synchronous), `start_plugin` /
`plugin_status` / `plugin_cancel` (asynchronous jobs), `list_plugins`,
`health`, `list_runs_by_target` / `list_run_artifacts` / `read_artifact`
(scoped artifact retrieval).

**Persistence.** `PostgresStore` records run metadata (plugin, container,
target, status, exit code, timestamps, input JSON) to PostgreSQL, required
via `DATABASE_URL`, aimed at a future durable, multi-client MCP deployment.
Each plugin's declared `target` input is indexed, so runs and their
artifacts can be looked back up by target later — not just returned inline
by the `run_plugin` call that produced them.

## Security model

Trust boundaries and open concerns are tracked as a living document in
[`SECURITY_CONSIDERATIONS.md`](SECURITY_CONSIDERATIONS.md). In short: the
MCP client and all plugin parameters are untrusted; a plugin package is
untrusted until the privileged installer has validated and copied it;
plugin containers remain untrusted even once installed; the setgid runner
and Docker socket access are the hard privilege boundary. Artifacts
(screenshots, fetched pages, tool output) are treated as sensitive report
data by design, not a bug to fix.

## Status and roadmap

Slickchook is under active development. See
[`ROADMAP.md`](ROADMAP.md) for planned work — multi-stage recon
orchestration, a safer static-analysis pipeline, and a multi-client MCP
deployment (PostgreSQL-backed sessions, Streamable HTTP, OAuth 2.1) — and
the dated `CHANGELOG_*.md` files for a record of what has shipped.

## Getting started

For a production deployment on Ubuntu Server, see
[`RUNNER.md`](RUNNER.md). For plugin installation and authoring, see
[`PLUGINS.md`](PLUGINS.md).

For local development:

```bash
cargo build
cargo test
```

Storage-layer tests require a reachable PostgreSQL instance: set
`TEST_POSTGRES_URL` to a database the test suite can create and drop tables
in, or that test is skipped.
