# Changelog — 2026-09-07

- Clarified `RUNNER.md`'s framing of `scripts/install-ubuntu.sh` per a
  direct request: it's now documented explicitly as a development-
  iteration tool for a host you already control (rebuilding/
  reinstalling core binaries quickly while working on Slickchook
  itself, or on core changes needed alongside plugin work, without
  repackaging a `.deb` each time), not a recommended alternative
  deployment method for a genuinely new installation - the `.deb`
  package remains the recommended path there, matching what this
  session's own established redeploy workflow has always actually
  done (lightweight `slickchook-plugin install` for manifest-only
  plugin changes, full `build-deb.sh`+`apt install` for any core Rust
  change - `install-ubuntu.sh` was never actually used for a redeploy
  this whole session). Also notes plugin-only development doesn't need
  this script at all - `slickchook-plugin install plugins/<name>`
  already covers that directly, pointing to `PLUGINS.md`.
  Left the script's actual documented behavior/capabilities unchanged
  (it can still perform a genuinely fresh install, and remains
  idempotent for a later re-run against an already-installed host) -
  this is a recommendation/framing change, not a new restriction the
  script itself doesn't already accept.
  Documentation-only change, no build/test impact beyond confirming
  nothing else broke.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean.
