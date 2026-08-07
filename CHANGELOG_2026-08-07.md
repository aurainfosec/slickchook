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
