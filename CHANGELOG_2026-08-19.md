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
