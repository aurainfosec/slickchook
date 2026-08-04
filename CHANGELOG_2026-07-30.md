# Changelog — 2026-07-30

- Added a `[[assets]]` table to the plugin manifest schema so a plugin can
  declare a file its image must bundle at a known path (a wordlist, a
  route corpus, anything a `Dockerfile` needs to produce) as a first-class,
  validated part of the manifest, rather than something only a
  `Dockerfile`'s own `COPY`/build steps happen to remember. Each entry has
  an optional `source` (a path relative to the plugin package directory,
  for a local `Dockerfile` build that copies it in), a required `dest`
  (the absolute path it must exist at inside the image), and a
  `min_size_bytes` (default 1, i.e. merely non-empty). `slickchook-plugin
  install` now enforces both ends automatically: for a package with a
  `Dockerfile`, every declared `source` must exist (and meet its size
  floor) in the staged package *before* the image is built, so a package
  copied or checked out without a required bundled file fails
  installation immediately with a clear error instead of silently
  producing a broken image; separately, after the image is built (or, for
  a plugin using a pulled upstream image with no `Dockerfile`, once that
  pull is confirmed), every declared `dest` is verified to actually exist
  at that size *inside the resulting image itself* (via `docker
  create`/`docker cp`, which works regardless of whether the image has a
  shell), catching a `Dockerfile` whose `COPY` drifted out of sync with
  the manifest too. If the post-build check fails, the previous working
  image (already re-tagged before the new build, mirroring the existing
  service-validation rollback) is restored before installation aborts.
  `source` is intentionally optional: `kiterunner`'s route corpus is
  fetched from a remote URL *during* its own `Dockerfile` build rather
  than copied from a package file, so its `[[assets]]` entry only
  declares `dest`, still catching a build-time fetch that silently failed
  or returned something truncated.
  Motivated by a suspected failure class in `feroxbuster`'s deployment: a
  plugin's container image can be rebuilt or redeployed from a package
  directory that no longer matches whatever bundled file an earlier
  (possibly AI-assisted) session had actually gotten into the image, with
  nothing in the manifest schema to catch the drift. Retrofitted both
  `feroxbuster` (`[[assets]]` for its `wordlist.txt`, `source` set) and
  `kiterunner` (`[[assets]]` for its downloaded route corpus, `source`
  omitted) as real, concrete examples.
  New `slickchook --print-assets <manifest>` mode backs the installer's
  checks, reusing the exact `load_manifest`/schema validation the service
  itself uses rather than re-parsing TOML in `bash`.
  Verified for real: `cargo test` (manifest validation, duplicate-`dest`
  rejection); the installer's `check_assets_in_package` against a package
  missing its declared source, one below the size floor, and a valid one;
  `check_assets_in_image` (extracted verbatim from
  `scripts/slickchook-plugin`, not a reimplementation) against three real
  Docker images — one with a valid bundled file, one missing it entirely,
  one present but under the declared minimum size — all producing the
  expected pass/fail outcome against a live Docker daemon.
- (While updating the artifact documentation above) Fixed a stale line in
  `PLUGINS.md` still describing runs as recorded in
  `artifacts/plugin_runs.sqlite3`, left over from before the SQLite
  backend was removed; it now correctly says the PostgreSQL `plugin_runs`
  table.
