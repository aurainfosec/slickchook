# Changelog — 2026-08-07

- Fixed `screensanitize`'s OCR step failing on every single screenshot,
  discovered live the moment the `gowitness` → `screensanitize` pipeline
  fix from 2026-08-06 let OCR actually run for the first time:
  `tesseract` exited non-zero on every image because Alpine's
  `tesseract-ocr` package ships the OCR engine with zero bundled language
  data (`tesseract --list-langs` reported 0 available languages,
  confirmed live). Every prior run had silently swallowed this — the
  script redirected `tesseract`'s stderr to `/dev/null` and caught the
  failure in a generic `except Exception`, so `sanitized.json` only ever
  showed an opaque `"error": "Command '[...]' returned non-zero exit
  status 1."`, never the actual reason.
  Fixed by adding `tesseract-ocr-data-eng` to `plugins/screensanitize/Dockerfile`,
  and by no longer discarding `tesseract`'s real stderr on failure —
  `sanitize.py` now surfaces the actual error message (e.g. a missing
  language data file) in `sanitized.json` directly, so a future OCR
  failure is diagnosable without another live debugging round-trip.
  Verified for real: a live `run_plugin` call chaining `screensanitize`
  onto a real `gowitness` screenshot of `example.com` previously produced
  `"error": "Command '['tesseract', ...]' returned non-zero exit status
  1."` with no OCR text and no flags; this is documented in
  `SECURITY_CONSIDERATIONS.md`'s SC-017 as the fourth compounding bug in
  that same investigation (after the `/input` wiring, the sanitized-image
  write path, and artifact-retrieval recursion/image-content-type fixes
  from 2026-08-06).
- Fixed a *fifth* bug in the same screenshot investigation, found while
  independently re-verifying `read_artifact` against the raw screenshot
  itself: it failed outright with `failed to open
  .../input/<file>.jpeg`. Root cause: the post-run artifact permission
  fixup's `find` command (added 2026-08-06 to stop `/artifacts/input`
  producing a nuisance chmod error on every run, back when that
  directory was always empty) excluded not just the bare `/artifacts/input`
  directory entry but *everything under it* (`find -path`'s `*` matches
  `/` too, so `! -path /artifacts/input/*` excluded the whole subtree
  recursively) — meaning a fetcher plugin's actual output written there
  (`webfetch`, and now `gowitness`) never got the same host-readability
  fixup as every other plugin's output, leaving it permanently unreadable
  by the `slickchook` service user. Fixed by excluding only the bare
  `/artifacts/input` path itself, so its contents are still recursed
  into and chmod'd like any other plugin output.
  This was a latent bug for `webfetch` too, just never noticed, since
  nothing had previously called `read_artifact` directly on a fetcher's
  own written file (`webfetch`'s output is normally only ever consumed
  downstream, inside a container, by a chained analyzer like
  `osvscanner`) — `gowitness` writing a screenshot there is what finally
  made a human want to `read_artifact` that file directly.
  Verified for real: a real `gowitness` screenshot's raw file, previously
  failing to open via `read_artifact`, now returns a proper MCP image
  content block.
  Separately, restored two unit tests (`validates_secret_env_var_names`,
  `reports_missing_required_secrets_only`) for the 2026-08-06
  plugin-secrets feature that were present in the original PR but went
  missing from `main` somewhere in a later merge — the underlying feature
  code was unaffected, only its test coverage was silently lost. Found
  while re-running `cargo test --lib` and noticing 10 tests instead of
  the expected 12.
