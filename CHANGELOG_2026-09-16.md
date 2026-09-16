# Changelog — 2026-09-16

- Removed the `gospider` plugin, per direct request, after confirming
  it never had a working build on this deployment: per
  `CHANGELOG_2026-08-10.md`, its image "has never actually been built
  on this server (`docker` tried to pull it from Docker Hub instead of
  building locally)", and its real `--help` output was never verified
  against the actual binary as a result. No changelog entry since then
  ever recorded it getting fixed, and it was still on the
  never-installed list as of this week's plugin audit. `katana`
  (added 2026-08-19) already covers the same content-discovery role
  and better - it's JS-aware, which a plain HTML crawler like gospider
  never was - so this isn't a coverage gap, just removing a plugin
  that was never confirmed to actually work.
  Also updated `katana`'s own manifest description, which referenced
  gospider by name (as the tool it complements/improves on) - reworded
  to describe the capability generically ("a plain HTML crawler")
  rather than point at a plugin that no longer exists.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean.
