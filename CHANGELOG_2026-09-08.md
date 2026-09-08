# Changelog — 2026-09-08

- Added `ARCHITECTURE.md`, collecting the ASCII diagrams worked out directly
  with the operator: deployment architecture with privilege boundaries
  (client container -> TLS -> Ubuntu Server VM -> unprivileged daemon ->
  setgid slickchook-runner -> root dockerd -> further-reduced per-run
  plugin container), the MCP client connection workflow (SSE handshake +
  JSON-RPC, async responses over the same stream), and the recon workflow
  (§0 scope discipline through Phases 1-5 into §4 findings summary and §5
  full artifact download/export), plus a higher-altitude companion view of
  the latter emphasizing the convergence into summary and download.
  Documentation-only change, no code/behavior impact.
  Verified: `cargo build --locked --release --bins` and `cargo test --lib`
  clean.
