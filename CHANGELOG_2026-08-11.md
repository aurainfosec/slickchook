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

- Fixed a packaging bug that broke every plugin run host-wide: after the
  binary redeploy above (the first real dpkg-level reinstall in a long
  time - previous redeploys this session only touched plugin Docker
  images via `slickchook-plugin install`, never the core package), a
  connected client reported all 12 in-flight jobs across 8 different
  plugins failing identically with `permission denied while trying to
  connect to the docker API at unix:///var/run/docker.sock`.
  `deploy/slickchook.service` documents the intended design in its own
  comment - "The runner relies on its setgid bit to obtain Docker
  socket access" - so only `slickchook-runner` can reach the Docker
  daemon, not the main service user, a deliberate least-privilege
  boundary. But `scripts/build-deb.sh` never actually applied that:
  `slickchook-runner` was installed with plain `install -m 0755`
  (root:root) and nothing in `postinst` ever set `docker` group
  ownership or the setgid bit. Someone had applied that by hand on this
  host at some point outside of packaging entirely - consistent with
  dpkg's own record showing this host's last tracked install as
  version 0.1.0, far older than the 0.3.0 binary that had actually been
  running. Reinstalling the package overwrote that manual fix by
  replacing the binary file outright.
  Added `chgrp docker` + `chmod 2755` on `/opt/slickchook/slickchook-runner`
  to `postinst`, applied unconditionally on every install/upgrade (not
  just guarded on first-install) since installing the binary file fresh
  resets both every time.
  Verified: `bash -n scripts/build-deb.sh` and a full `./scripts/build-deb.sh`
  rebuild, confirming the fix is present in the built package's
  extracted `postinst` (`dpkg-deb -e`) before shipping it.

- Added `-Pn` support to `nmap` (new `no_ping` option) and broadened
  its NSE script support per a connected AI client's feedback while
  testing real scans. The client reported two gaps: no way to skip
  Nmap's host discovery when ICMP/host-discovery probes are filtered
  (a common real-world condition that otherwise makes Nmap wrongly
  report a live host as down and skip it entirely), and no way to run
  NSE scripts beyond the existing hardcoded SSH-only preset.
  Renamed `ssh_checks` to `nse_scripts`, keeping the same default
  (`ssh2-enum-algos,ssh-auth-methods`) so existing behavior is
  unchanged by default, but now accepting any of Nmap's own
  script/category names (vuln, default, http-title, specific CVE
  scripts, etc.) - this plugin uses the stock instrumentisto/nmap image
  with no custom Dockerfile, so Nmap's full built-in script library
  already ships under /usr/share/nmap/scripts; no bundling or install
  step needed for any of Nmap's own stock scripts. Documented that
  Nmap validates script names itself at scan time (errors clearly on
  an unknown one), since slickchook's install step never runs the
  container and so can't catch a typo'd script name any earlier.
  Updated SKILL.md's now-renamed `ssh_checks` reference and added
  guidance to only reach for broader NSE categories when the
  engagement calls for it, since they're much noisier/slower than the
  SSH-only default.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean.

- Added `purge_runs_by_target`, a new MCP tool to let an operator delete
  old artifacts on request rather than leaving them to accumulate
  forever - there was previously no cleanup mechanism of any kind.
  Deliberately not an automatic/silent GC (e.g. a systemd-timer
  age-based sweep): the request was specifically for something closer
  to Burp Suite's "delete this project / keep it / just continue"
  choice, and since slickchook is a headless daemon with no terminal UI
  to show that prompt in, the choice is instead surfaced through the
  connected AI client - it calls list_runs_by_target when a target with
  existing history is about to be (re)engaged, asks the operator
  whether to delete/keep/continue, and only calls
  purge_runs_by_target if the operator says delete. The tool's own
  description states this requirement explicitly (irreversible, human
  confirmation required, never call automatically) so any MCP client
  reading it has that context without relying on SKILL.md alone.
  Implementation: `PostgresStore::delete_runs_by_target` (src/storage.rs)
  deletes matching `plugin_runs` rows and returns their `artifact_dir`s,
  then removes each directory on disk, reusing the same
  `canonical_contained` symlink-escape protection `resolve_run_artifact_dir`
  already uses. A row still counts as deleted once its DB entry is gone
  even if the directory removal itself fails (reported in the summary's
  `errors`, not silently dropped) - matches `list_runs_by_target`'s own
  guarantee that a purged run won't show up again either way.
  Updated SKILL.md with the keep/delete/continue guidance next to the
  existing `list_runs_by_target` bullet.
  Verified: `cargo build --locked --release --bins` and `cargo test`
  (14/14 - 12 lib + 2 main.rs, including `registers_all_mcp_tools` now
  covering the new tool) clean. Live verification pending a full binary
  redeploy (same `build-deb.sh`/`apt install` path as the schema and
  docker-permission fixes earlier today).
