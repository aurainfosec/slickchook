# Changelog — 2026-09-10

- Extended the `testssl` plugin's documentation and option surface, per
  direct request, so an MCP client knows how to gather enough
  information to complete two standard TLS-configuration report
  tables (a protocol-support table across SSLv2 through TLS 1.3, and a
  weak-cipher-component table broken down by key exchange, encryption
  algorithm, mode, and MAC/HMAC).
  Read testssl.sh's own real source directly rather than assume field
  names from its docs: confirmed the six protocol findings
  (`SSLv2`/`SSLv3`/`TLS1`/`TLS1_1`/`TLS1_2`/`TLS1_3`, each with a
  `finding` of exactly `"offered"` or `"not offered"`) are always
  produced by the default battery, needing no new option - documented
  the exact ids and their meaning (`TLS1` is TLS 1.0) directly in the
  manifest.
  Confirmed the weak-cipher-component table is a different story: the
  default battery only reports broad historical cipher-category buckets
  (NULL/EXPORT/LOW/3DES/etc.), not a per-suite breakdown by individual
  key exchange, encryption algorithm, mode, or MAC. Building that table
  needs testssl's own `--each-cipher`/`--cipher-per-proto` flags, which
  this manifest didn't expose at all - added both as new options
  (`each_cipher`, `cipher_per_proto`). Confirmed from source exactly what
  those produce: one `cipher_<hexcode>` finding per accepted cipher
  suite, whose `finding` is a formatted text line (not separate
  structured fields) combining the cipher's name with `Kx=`/`Enc=`
  substrings - documented precisely how to parse that text into each
  table row, including two details easy to get wrong without reading
  source: the negotiated Diffie-Hellman bit-length for `DHE` key
  exchange is appended directly onto the same `Kx=DHE` substring (e.g.
  `Kx=DHE 1024`), not a separate field; and CBC mode / MD5/SHA1 MAC
  aren't labeled fields at all - only visible in the cipher suite's own
  name (no GCM/CCM/POLY1305/CHACHA20 marker means CBC; a trailing
  `-MD5`/`-SHA` suffix names the MAC, distinct from `-SHA256`/`-SHA384`).
  Also flagged that ChaCha20-Poly1305 appearing as "weak" in this
  specific table reflects one compliance framework's own approved-
  algorithm list, not a general cryptographic weakness - so the
  manifest tells the client not to editorialize that it's actually fine
  elsewhere, rather than silently contradict the table it's building.
  Raised `timeout_seconds` from 900 to 1500 to accommodate
  `each_cipher`/`cipher_per_proto` testing every locally-known cipher
  suite individually, which meaningfully extends a run beyond the
  default battery's own budget.
  Manifest-only change, no Dockerfile/image change (this plugin already
  wraps the official `drwetter/testssl.sh` image directly).
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12, including the existing dedicated
  `scans_testssl_plugin_manifest` test) clean.
