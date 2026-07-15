# Changelog — 2026-07-15

## Perimeter enumeration runtime fixes

- Added a constrained writable `/tmp` tmpfs to HTTPX after live testing found
  its resolver cache could not be created on the read-only container root.
- Fixed the Feroxbuster image Dockerfile to create `/opt/slickchook` before
  copying its bounded package wordlist. Full SecLists content remains opt-in
  rather than being embedded in privileged images.
- Made the Feroxbuster image build assert that its wordlist exists and moved
  the file to `/wordlist.txt` to avoid ambiguous nested-directory COPY behavior.
- Hardened stdout/stderr artifact recording against symlink or non-regular-file
  interference and improved write error reporting.

## Fixed

- Enabled the MCP tools capability in the server's initialization response.
- Attached `#[tool_router]` to the implementation that defines `run_plugin`,
  `list_plugins`, and `health`, so those tools are registered with the router.
- Attached `#[tool_handler]` to the `ServerHandler` implementation, so
  `tools/list` and `tools/call` requests are dispatched through that router
  instead of falling back to rmcp's default handlers.
- Declared the `run_plugin` arguments as an rmcp `Parameters` schema so the
  router accepts the existing `{ "plugin": ..., "input": ... }` MCP payload.
- Added an idempotent Ubuntu installation script and systemd unit template that
  install the runner as `root:docker` with mode `2750`, grant only the service
  account execution via ACL, configure artifact ACL inheritance, and verify the
  resulting deployment.
- Extended the installer to recognize existing deployments, preserve runtime
  data, back up files changed by an update, normalize legacy accounts and Docker
  group membership, stop managed services safely, and reject unmanaged port
  conflicts.
- Added containerized plugins for Nmap TCP connect scanning, ProjectDiscovery
  Naabu CONNECT scanning, and a focused `rawping` check that requests only the
  `NET_RAW` capability.
- Fixed Docker bind mounts by passing the runner's validated canonical artifact
  directory instead of a relative path that Docker treated as a volume name.
- Added pinned ProjectDiscovery Nuclei and HTTPX plugins with bounded resources,
  update checks disabled, and no additional Linux capabilities.
- Decoupled plugin deployment from core installation. The core installer now
  installs a standalone `slickchook-plugin` manager, while independent plugin
  packages can be installed, updated, validated, rolled back, listed, and
  removed without access to the core or runner source repositories.
- Added independently buildable packages for Medusa single-credential checks,
  bounded Feroxbuster content discovery, and Waymore archive enumeration, plus
  a pinned official Subfinder plugin for passive bug-bounty enumeration.
- Extended `slickchook-plugin` to build package-owned Dockerfiles with local
  image tagging and rollback tags, avoiding unreviewed third-party tool images.

## Tests

- Added checks that the server advertises tool support and registers all three
  MCP tools.
- Verified the rebuilt server over its SSE transport: initialization advertised
  `tools`, `tools/list` returned all three tools, and `tools/call` successfully
  invoked `health`.
- Added manifest-loading checks for the Nmap, Naabu, and rawping plugins and
  their intended capability policies.
- Extended manifest checks to cover the pinned Nuclei and HTTPX images.
## Plugin runtime filesystem fixes

- Added writable, constrained `/tmp` tmpfs to the Naabu profile, fixing its
  `Could not create runner: mkdir /tmp/...: read-only file system` failure.
- Added constrained `/.config` tmpfs mounts to Subfinder and Nuclei, whose
  non-root runtime defaults otherwise attempt to create configuration under
  the read-only container root.
- The Feroxbuster package includes its wordlist in the image; reinstalling the
  package rebuilds the image so older deployments receive that file.
## Perimeter enumeration plugins

- Added independently installable `dnsx`, `katana`, and `gowitness` plugin
  packages for DNS verification, bounded web crawling, and visual web triage.
- Pinned dnsx and Katana image tags; all three profiles use non-root containers,
  resource limits, and constrained writable temporary filesystems.

## Deployment packaging and offline analysis

- Fixed cross-run analyzer handoff validation so staged input may come from a
  completed run while remaining confined to the shared artifacts root.
- Documented and enforced the runner deployment mode `2751 root:docker`,
  preserving least privilege without adding the service account to `docker`.
- Updated the Retire.js image to bundle its JavaScript and npm vulnerability
  databases and use them offline at runtime.
- Documented independent plugin repositories, self-contained plugin packages,
  local image builds, and destination-server installation steps.

All implementation, installer, plugin, and documentation changes must add a
dated entry to this changelog file.

- Corrected Retire.js `--jsrepo` to reference the bundled database files
  explicitly rather than their containing directory.
- Switched the bundled JavaScript database to the legacy `jsrepository.json`
  format required by the pinned Retire.js 5.2.7 release.
- Pinned both bundled Retire.js databases to the matching `v5.2.7` source tag
  to prevent schema drift from the upstream `master` branch.
- Documented Retire.js database supply-chain controls in
  `SECURITY_CONSIDERATIONS.md`.
- Corrected the Retire.js source tag URL to the repository's `5.2.7` tag name
  (without the `v` prefix), fixing image builds that previously returned 404.
- Upgraded the Retire.js image to `5.4.3` so its bundled databases match the
  current repository schema.
- Removed the unreliable Retire.js plugin and added an offline OSV-Scanner
  plugin with a build-time database refresh and network-disabled runtime.
- Fixed the OSV-Scanner image build to produce a deterministic binary path and
  use the supported offline-database download command.
- Corrected the builder to use `go install` for the version-pinned module, then
  copy the resulting binary into the image.
- Added the required `--offline` flag to the OSV database download step.
- Corrected the database refresh to use OSV-Scanner's standalone download
  command rather than invoking a source scan without package inputs.
- Configured the OSV image cache directory explicitly and seeded the build-time
  database download with a minimal package source.
- Changed the OSV build seed to a valid npm lockfile and explicit source scan.
- Passed the seed lockfile explicitly with `-L` so OSV-Scanner does not rely on
  directory source discovery during the image build.
- Corrected the database refresh to use `--offline-vulnerabilities` and a seed
  lockfile containing a real npm dependency.
- Enabled Naabu `NET_RAW` support and a declared CONNECT/SYN scan-type option;
  resource limits and argument validation remain enforced.
- Added independently installable Kiterunner (small route corpus) and
  GraphQL-check plugins with bounded resources and explicit targets.
- Documented Kiterunner corpus sizing and the requirement to review large
  route corpora separately.
- Updated manifest tests for Naabu's newly enabled `NET_RAW` scan-type option.
- Corrected the Kiterunner image build to install its `cmd/kiterunner` CLI
  package rather than the library module root.
- Added a constrained `/.cache` tmpfs to Kiterunner for its non-root runtime
  cache initialization.
- Corrected the GraphQL checker image to use the only published
  `graphql-cop` package version, `0.0.1`.
- Changed the GraphQL checker entrypoint to invoke the package as a Python
  module because it does not publish a `graphql-cop` console script.
- Replaced the unusable GraphQL package wrapper with a self-contained,
  dependency-free introspection checker using Python's standard library.
- Set explicit non-root read/execute permissions on the GraphQL checker script.
- Made GraphQL checker output handle non-JSON HTTP responses and report status
  and content type cleanly.
- Recorded the explicit accepted risk that artifact contents, including
  screenshots and fetched data, are report information and may be sensitive.
- Documented screenshot prompt-injection controls and marked Gowitness capture
  as explicit opt-in untrusted report data.
- Added the planned screenshot-sanitization workflow to the roadmap and
  security considerations, including hidden/encoded text detection controls.
- Added the sandboxed `screensanitize` plugin for metadata-stripped image
  re-encoding, OCR, encoded-text indicators, and prompt-injection-like markers.
- Added a PostgreSQL-first multi-client MCP roadmap covering durable sessions,
  resumable events, Streamable HTTP, OAuth 2.1, scopes, and isolation tests.
- Added the initial PostgreSQL schema for plugin runs, MCP sessions, and
  resumable events; SQLite remains the active backend until the repository
  abstraction is implemented.
- Added an explicit `scripts/migrate-postgres.sh` schema application tool with
  URL validation and fail-fast `psql` behavior.
- Added explicit storage-backend configuration parsing, retaining SQLite as the
  default until the PostgreSQL repository implementation is complete.
- Wired backend selection into `PluginManager` so an accidental PostgreSQL
  activation fails clearly instead of silently using SQLite.
- Added the initial `RunStore` interface and portable stored-run value used by
  the upcoming SQLite and PostgreSQL adapters.
- Added the first SQLite `RunStore` adapter with canonical artifact containment
  checks; PluginManager wiring remains the next step.
- Selected `tokio-postgres` for the PostgreSQL adapter to keep the persistence
  layer SQL-first and lightweight; PostgreSQL remains opt-in until integration
  tests pass.
- Added the `async-trait` dependency as preparation for the asynchronous
  persistence interface; no runtime behavior changes yet.
- Removed the duplicate PluginManager schema initializer; schema ownership now
  resides exclusively in the active store adapter.
- Completed the `RunStore` interface definition with schema initialization,
  completed-input lookup, and run-record persistence operations.
- Expanded stored-run fields to preserve container, exit code, and lifecycle
  timestamps required by the existing PluginManager schema.
- Made the build-time seed scan tolerate exit code `1`, which means the
  database loaded successfully and findings were reported.
