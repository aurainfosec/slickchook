# Changelog — 2026-10-15

- Added a `LICENSE` file (GNU General Public License v3.0 or later,
  full canonical FSF text), per direct request. No license file or
  mention existed anywhere in the repo before this - confirmed by
  checking for an existing `LICENSE`, and for any license references
  in `README.md`/`Cargo.toml`, before adding one.
  Used the "or later" variant (`GPL-3.0-or-later` in `Cargo.toml`'s new
  `license` field) rather than a strict GPL-3.0-only - the FSF's own
  recommended default for a new project, and the more common choice in
  practice - since the request didn't specify "only"; flagged here so
  this is easy to correct if a strict version-3-only license was
  actually intended.
  Added a short "## License" section to `README.md` pointing at the
  new file, and noting explicitly that this license covers this
  repository's own code and plugin manifests - not the third-party
  tools each plugin wraps or references (nmap, testssl.sh, Nuclei, and
  the rest), which each keep their own separate licenses.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean - `Cargo.lock` needed no changes, since a
  package's own `license` field is metadata, not a dependency
  resolution input.
