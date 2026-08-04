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
