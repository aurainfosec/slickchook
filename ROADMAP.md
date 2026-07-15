# Slickchook roadmap

Updated: 2026-07-15

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

- [ ] Add a storage abstraction and PostgreSQL backend; retain SQLite only as a
  documented single-node development mode.
- [ ] Implement the PostgreSQL repository backend against
  `migrations/postgres/001_initial.sql` and add an explicit backend selection
  configuration; do not silently migrate or delete existing SQLite data.
- [ ] Migrate plugin-run and artifact metadata to PostgreSQL with an audited,
  repeatable migration tool.
- [ ] Persist MCP sessions, authenticated subject/client ownership, expiry, and
  resumable event history.
- [ ] Migrate the HTTP transport to Streamable HTTP with `Mcp-Session-Id` and
  `Last-Event-ID` replay support while retaining compatibility during rollout.
- [ ] Implement MCP OAuth 2.1 authorization discovery, PKCE, bearer validation,
  audience checks, and scoped permissions (`read`, `run`, `cancel`, `admin`).
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
