# Slickchook roadmap

Updated: 2026-07-16

This roadmap covers Slickchook server, runner, plugin, persistence, security,
and deployment capabilities only. It does not contain findings or work items
for any assessed target environment.

## Attack-surface orchestration

- [ ] Add a workflow/DAG runner for staged reconnaissance.
- [ ] Parallelize independent passive discovery plugins.
- [ ] Deduplicate domains, IPs, ports, URLs, and findings between stages.
- [ ] Feed discovered assets automatically into DNS, port, HTTP, and service probes.
- [ ] Crawl only confirmed HTTP(S) services.
- [ ] Prioritize API, OpenAPI/Swagger, GraphQL, login, admin, and JavaScript surfaces.
- [ ] Add authenticated testing using short-lived, redacted credentials or API keys.
- [ ] Add per-host concurrency and rate-limit budgets.
- [ ] Stop or back off after repeated timeouts, 429s, or 503s.
- [ ] Cache DNS, HTTP, certificate, and technology results with timestamps.
- [ ] Track plugin/image versions, digests, options, target, and provenance.
- [ ] Emit a normalized attack-surface inventory alongside raw artifacts.
- [x] Add server-side artifact listing and bounded read APIs scoped by
  target/run (`list_runs_by_target`, `list_run_artifacts`, `read_artifact`),
  with canonical-root checks, size limits, audit logging, and explicit
  untrusted-content labeling. Not yet gated by authentication — see the
  authorization item below; these tools currently share the same trust
  boundary as `run_plugin`.
- [ ] Support incremental rescans based on changed assets.
- [ ] Add scheduled monitoring and change detection.

## Plugin quality and safety

- [ ] Verify image availability and CLI compatibility during plugin installation.
- [ ] Add plugin smoke tests for image entrypoints and required files.
- [ ] Redact secrets from inputs, logs, database records, and artifacts.
- [ ] Preserve least privilege for the MCP service and runner.

## Static web-content analysis

- [ ] Add an authorized URL fetcher that stores immutable per-run assets.
- [ ] Add a read-only `/input` staging mount for analyzer plugins.
- [ ] Keep analyzers network-disabled and non-root.
- [ ] Never execute downloaded JavaScript or HTML.
- [x] Add offline OSV-Scanner dependency analysis.
- [x] Add bounded Kiterunner API route discovery and GraphQL checks.
- [x] Add a screenshot sanitization/inspection stage: strip metadata, re-encode,
  detect hidden or encoded text, and emit attacker-controlled structured output.

## Multi-client MCP deployment

- [x] Add a PostgreSQL-backed run store implementing
  `migrations/postgres/001_initial.sql`; SQLite has been removed and
  PostgreSQL is now the sole, required backend.
- [ ] Persist MCP sessions, authenticated subject/client ownership, expiry, and
  resumable event history.
- [ ] Migrate the HTTP transport to Streamable HTTP with `Mcp-Session-Id` and
  `Last-Event-ID` replay support while retaining compatibility during rollout.
- [ ] Implement MCP OAuth 2.1 authorization discovery, PKCE, bearer validation,
  audience checks, and scoped permissions (`read`, `run`, `cancel`, `admin`);
  gate the artifact-read tools (`list_runs_by_target`, `list_run_artifacts`,
  `read_artifact`) behind this once it lands.
- [ ] Enforce per-subject job/artifact isolation, concurrency quotas, and
  reconnect-safe cancellation semantics.
- [ ] Add restart, reconnect, concurrent-client, token-expiry, and authorization
  regression tests before enabling remote multi-client deployment.
- [ ] Add JSluice/LinkFinder endpoint extraction.
- [ ] Add SecretFinder secret-pattern analysis with redaction.
- [ ] Add source-map discovery and static inspection.
- [ ] Enforce asset count, response-size, and total-download limits.
- [ ] Add cleanup and cancellation tests for staged assets.
- [ ] Refactor run persistence to support safe fetch-to-analyzer handoff by `input_run_id`.
- [ ] Reject unknown, incomplete, expired, cross-root, and symlinked input runs.
- [ ] Add end-to-end fetcher-to-read-only-analyzer integration tests.
