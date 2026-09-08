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

- Live-verified `dehashed` end to end for the first time with a real,
  human-confirmed query (`domain:tgh.co.nz`, per the plugin's own
  propose-then-confirm design) once a real API key was configured. The
  plugin correctly built the request and made exactly one call, but
  Dehashed's own API returned a real `HTTP 401`:
  `{"error": "You need a search subscription and API credits to use the
  API, please purchase a search subscription."}` - despite the account
  dashboard showing available credits.
  Investigated rather than assumed a plugin bug: confirmed the plugin's
  request format (endpoint `https://api.dehashed.com/v2/search`, header
  `Dehashed-Api-Key: <key>`) is byte-for-byte identical to
  `myOmikron/dehashed-rs`, an actively maintained real Rust client for the
  same v2 endpoint - ruling out a request-format bug here. The operator
  then confirmed directly against their own Dehashed account dashboard:
  it shows API credits but not search credits specifically - Dehashed
  genuinely tracks these as two separate balances, and the general
  credit-balance view showing a nonzero amount does not mean the
  `/v2/search` endpoint's own search-credit/subscription requirement is
  met.
  Documented this real, easy-to-misread gotcha directly in
  `dehashed.toml`'s description, so a future engagement hitting the same
  401 despite a nonzero credit balance knows to check specifically for
  search credits/an active search subscription rather than re-diagnosing
  the plugin. No code changes - this is a real account-side billing state,
  not something the plugin can detect or work around, and no further
  paid API calls were made once the account-side cause was confirmed, to
  avoid spending further credits against a call that would just repeat
  the same rejection.
  Verified: `cargo build --locked --release --bins` and `cargo test --lib`
  clean (manifest-only change, no build/test impact expected).
