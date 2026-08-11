# Changelog — 2026-08-11

- Fixed `run_plugin` and `start_plugin`'s declared MCP tool schema
  rejecting real strict clients: both tools' `input` parameter was typed
  `serde_json::Value` in `RunPluginRequest`, which schemars renders as
  the bare JSON Schema boolean `true` ("anything goes") rather than an
  object schema. That's legal per draft-07/2020-12, but Claude Code's
  own MCP client validator rejects a boolean sub-schema at
  `tools.N.inputSchema.properties.input`, so `tools/list` failed
  entirely for any strict client - this had been invisible all session
  because every live verification so far went through hand-rolled
  scripts calling the raw JSON-RPC endpoint directly, which never
  validate the server's advertised schemas the way a real client does.
  Changed `input` to `serde_json::Map<String, serde_json::Value>`,
  matching how it's actually used (plugin input is always a JSON
  object of named option/argument values, never a bare scalar/array),
  which makes schemars emit a proper `{"type": "object", ...}` schema.
  Updated both call sites to wrap the map back into a `Value::Object`
  before handing it to `PluginManager::run_plugin_with_input`, whose
  signature is unchanged.
  This is a core binary change, not a plugin manifest - redeploying it
  needs a full package rebuild/reinstall
  (`cargo build --locked --release --bins && ./scripts/build-deb.sh &&
  sudo apt install ./dist/slickchook_0.3.0_<arch>.deb`), not the
  lightweight `slickchook-plugin install` used for manifest-only
  changes elsewhere in this log.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean. Live verification against a real strict MCP
  client still pending the redeploy above.
