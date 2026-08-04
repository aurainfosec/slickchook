# Changelog — 2026-07-30

- Fixed a resource leak in `slickchook-runner`'s own timeout handling.
  `run()` wraps the `docker run` child process in `tokio::time::timeout`,
  but when that timeout elapsed the awaited future was simply dropped;
  Tokio does not kill a child process on drop unless `kill_on_drop` is set,
  so the underlying `docker run` process (and its container) was
  orphaned and kept running in the background, completely disconnected
  from Slickchook's own bookkeeping. The runner itself then exited
  quickly and deliberately with status 1, which is why timed-out runs were
  recorded as `status: "failed", exit_code: 1` rather than a hang or
  `status: "error"` — from `plugin_manager.rs`'s point of view the runner
  subprocess completed normally, it just reported a controlled failure.
  Found by investigating a real `amass` run against `example.com`
  (`c373ca40-03ef-4f33-9363-f1a44e6904c9`) whose duration matched the
  manifest's `timeout_seconds` (900s) almost exactly, corroborating a
  client-reported "silent hang to 900s timeout" independent of the
  already-fixed `/.config` permission issue. Every container is now
  started with a deterministic `--name` (derived from its run ID), and if
  the runner's timeout fires it explicitly `docker kill`s the container by
  that name before returning the timeout error — killing the orphaned
  `docker run` client process alone would not have stopped the container
  itself, only the daemon-side kill does; `--rm`'s auto-removal is a
  daemon-side flag on the container, so it still fires correctly even
  though the original `docker run` client may have already been abandoned.
  Affects every plugin that can time out, not just `amass`. Verified
  against a live Docker daemon: a manifest that sleeps past a 3-second
  timeout is killed and leaves no container behind (`docker ps` empty
  afterward), where it previously kept running for its full sleep
  duration, orphaned.
- Fixed a second, independent bug found while investigating the same area:
  files a plugin container writes directly into its `/artifacts` mount
  (as opposed to `stdout.txt`/`stderr.txt`, which `plugin_manager.rs`
  writes host-side from captured output) could become permanently
  unreadable by the Slickchook service afterward. Root cause: plugin
  containers run as UID 65534, and some tools restrict permissions on
  their own output as they write it — confirmed against a real, live
  `testssl` run on production (`9b2c499d-2365-46aa-ba54-a9a0307c78cf`):
  `list_run_artifacts` correctly listed `report.html` and `result.json`,
  but `read_artifact` failed to open either one with a raw OS-level error,
  while `stdout.txt`/`stderr.txt` from the same run opened fine. A
  pre-set default ACL granting the service user access wouldn't reliably
  survive this either: POSIX recomputes a file's ACL mask from whatever
  permissions a tool's own `chmod` call requests, which caps (and can
  zero out) any named-user ACL entry — so the fix has to run after the
  tool, not before. `slickchook-runner` now runs one more short-lived
  container after every plugin run (covering success, failure, and the
  timeout-kill path above), as the same UID that owns the files, to make
  everything under `/artifacts` readable again — only a file's owner or
  root may relax its permissions.
  First implementation used `chmod -R a+rX /artifacts`, which real-Docker
  testing caught as broken: the recursive chmod's first target is the
  mount point itself, which is never owned by UID 65534 (it's created by
  the Slickchook service process, in this fix's own test *and* in real
  production identically), so that chmod call failed with "Operation not
  permitted" and the whole recursive walk aborted before ever reaching
  the actual files inside. Fixed by using
  `find /artifacts -mindepth 1 -exec chmod a+rX {} +` instead, which never
  touches the mount point itself, only what the plugin container actually
  created inside it (which it does own). Best-effort: a fixup failure is
  logged, not propagated, since the plugin's own result matters more than
  this cleanup step.
  Verified against a live Docker daemon: a container that writes a file
  and `chmod 600`s it (mirroring `testssl.sh`'s behavior) — the file was
  unreadable (`600`, owned by `nobody`) before this fix, and is `644` and
  readable immediately afterward with no fixup error logged.
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
- Fixed a real bug in the `[[assets]]` feature above, found the first time
  a plugin with an omitted `source` (`kiterunner`) actually went through
  the installer: `--print-assets` and the installer's asset checks used a
  tab as the field delimiter, but bash's `read` treats tab as "IFS
  whitespace" even when `IFS` is set to just a tab — meaning a *leading*
  empty field (an omitted `source`, printed first) gets silently stripped
  rather than preserved, shifting `dest` and `min_size_bytes` left by one
  field. For `kiterunner` this meant `source` became its `dest` path and
  `dest` became its `min_size_bytes`, producing the confusing error
  `kiterunner declares asset source `/opt/kiterunner/routes-small.kite`
  (for 1024) but it is missing from the package`. `feroxbuster` (whose
  `source` is set, not omitted) never hit this, which is why it wasn't
  caught earlier. Switched the delimiter to the ASCII unit separator
  (`\x1f`), which isn't "IFS whitespace" and so preserves empty fields
  regardless of position. Verified with a direct parse of both plugins'
  real `--print-assets` output post-fix (`kiterunner`'s `source` now
  reads back empty rather than shifted) and by re-running
  `check_assets_in_package` against `kiterunner`'s real package
  directory.
- Added `slickchook-plugin install --force-rebuild`, which passes
  `--no-cache` to `docker build` for a package with a `Dockerfile`.
  Without it, an install whose `Dockerfile` and build context are
  unchanged correctly reuses Docker's cached layers — expected, not a
  bug, since the resulting image is identical either way — but this
  gives an explicit way to bypass that cache when specifically
  diagnosing or ruling out a suspected stale image.
