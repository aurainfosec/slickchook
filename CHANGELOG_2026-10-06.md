# Changelog — 2026-10-06

- Added a new `httprequest` plugin, per direct request: sends one
  HTTP(S) request to an authorized URL using any method (GET, POST,
  PUT, DELETE, PATCH, HEAD, OPTIONS, etc.), via real curl. Fills a real
  gap the existing `webfetch` plugin doesn't cover - `webfetch` only
  ever does a plain GET (via `wget`) for passive offline analysis, with
  no way to set a method, send a body, or set custom headers. This
  plugin exists specifically so a finding like the one surfaced in an
  earlier session (a content-discovery plugin finding a `POST` route)
  can actually be followed up with a real, controlled request, not just
  observed.
  Documented directly in the manifest, matching `SKILL.md` §0's
  existing "performs a live action against a real service" category:
  a non-GET request (especially POST/PUT/DELETE/PATCH) can be a real,
  state-changing action against the target, not just observation -
  being in scope for an engagement isn't the same as that specific
  action being intended.
  `headers` accepts multiple custom headers at once (one "Name: Value"
  pair per line) despite this fleet's schema only allowing one value
  per declared option - reused the exact wrapper pattern already
  established by `psk-crack`/`sippts-dcrack` for the same underlying
  limitation: a thin `entrypoint.py` accepts the whole blob as its own
  `--slickchook-headers` option and turns it into repeated real `-H`
  flags before execing curl with everything else passed through
  verbatim.
  `data` is sent via curl's `--data-raw` rather than plain `-d`, so an
  arbitrary request body is always sent exactly as given, never
  misread as "read from this local file" just because it happens to
  start with `@` or `+` (a real curl footgun avoided deliberately, not
  by accident).
  Response headers and body are always written to separate artifacts
  (`response_headers.txt`, `response_body`) since the body may be any
  content type including binary - stdout carries a short status/
  timing/size/content-type summary via curl's own `-w` write-out, so
  the common case doesn't need to open an artifact at all.
  `insecure` (skip TLS verification) and `basic_auth` (a real
  credential retained in run history) both carry the same cautions
  already documented on other plugins with equivalent options.
  Built on `alpine:3.21` + the distro's own `curl` package (not a
  third-party image) - a small, well-understood base for a tool this
  simple.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12, including `scans_network_plugin_manifests`, which
  parses this new manifest along with the rest of the fleet) clean.
  Also compiled `entrypoint.py` locally (`python3 -m py_compile`) and
  exercised its header-blob-to-`-H`-flags parsing logic directly in
  this sandbox (no Docker access here) - confirmed blank lines are
  skipped and surrounding whitespace trimmed correctly before ever
  building the image. Live verification (a real request against an
  authorized target) still pending redeploy, since this sandbox has no
  Docker access to build/run the actual image.
