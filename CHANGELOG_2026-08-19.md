# Changelog — 2026-08-19

- Expanded `virustotal-resolutions`' description to explain not just
  why historical resolution data matters (finding an origin IP behind
  a WAF/CDN) but how to actually test a candidate IP: connect directly
  to it while still presenting the real domain as the Host
  header/TLS SNI (e.g. `curl --resolve <domain>:443:<historic-ip>
  https://<domain>/`), since the origin often still serves the site
  based on virtual-host matching regardless of which IP the request
  arrived on - hitting the bare IP directly instead would likely miss
  the intended virtual host or TLS certificate entirely. Prompted by a
  direct request to verify this plugin's documentation actually
  covered the WAF-bypass testing methodology, not just the data
  source. Checked first whether `httpx`/`webfetch` already exposed a
  Host-header-override option to reference instead of describing raw
  curl usage - neither currently does, so this documents the general,
  standard technique directly rather than assuming a companion
  plugin capability that doesn't yet exist.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean (manifest description update only).

- Added a new `katana` plugin (ProjectDiscovery's next-generation
  crawler) - a modern, JS-aware complement to `gospider` for content
  discovery: it parses endpoints out of JavaScript files that a plain
  HTML crawler never sees, which matters on a modern JS-heavy site.
  References the official `projectdiscovery/katana:v1.7.0` Docker
  image directly (confirmed real via its Dockerfile/dockerhub-push CI
  workflow) - no custom build needed. Every flag verified against the
  real v1.7.0 binary's own `-h` output (downloaded the Linux release
  directly, no docker access in this sandbox) rather than guessed.
  Deliberately scoped to Katana's well-understood non-headless HTTP
  crawling mode for this first pass - the image bundles Chromium for
  headless crawling, but its exact chrome-binary-path requirements in
  this specific image haven't been verified live, and two real bugs
  shipped this week already from assuming an unverified detail worked
  (vt-cli's tag naming, then its init requirement) - not repeating
  that a third time on an unverified guess. Headless mode can be added
  later once actually tested.
  Flagged two options as genuine active side effects, not passive
  crawling, matching this fleet's usual credential/live-action
  callouts: `automatic_form_fill` (experimental - actually fills in
  and likely submits discovered forms, real side effects on the
  target) and `kb_validate_secrets` (sends live API calls to each
  detected secret's own provider to check if it's actually valid).
  `no_scope` is flagged as the one option that can take a crawl
  outside the authorized domain entirely.
  No `SKILL.md` change needed - Phase 3's existing generic "installed
  content-discovery/crawling plugins" wording already covers this
  addition without naming any specific tool, confirming the
  plugin-agnostic rewrite works as intended for a brand-new plugin.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean. Live verification (a real crawl) pending
  install, since this sandbox has no docker access.
