# Changelog — 2026-07-16

- Clarified that changelog entries describe Slickchook implementation and plugin
  changes only; target-specific assessment findings belong in assessment reports.
- Added a roadmap item for authenticated, target-scoped artifact listing and
  bounded artifact reads as server-side MCP capabilities.
- Expanded plugin documentation with explicit upstream-image pull, local-image
  build, smoke-test, installation, restart, and verification commands.
- Added `scripts/build-deb.sh` for creating a native `dpkg-deb` deployment package
  from release binaries without embedding PostgreSQL credentials.
- Fixed Debian package construction by including the TOML conffile payload and
  treating the generated PostgreSQL environment file as runtime state.
- Documented the source-checkout, Debian-package, and protected live plugin
  directories and their corresponding installation commands.
- Corrected plugin documentation to reflect that `slickchook-plugin install`
  automatically builds reviewed Dockerfiles, while upstream images are pulled
  explicitly.
- Updated the plugin installer to verify declared container images before
  activation and print an actionable pull/build command when an image is absent.
- Hardened `.gitignore` for generated Debian artifacts, Docker/cloud credentials,
  SSH keys, local caches, crash dumps, and additional secret-like files.
