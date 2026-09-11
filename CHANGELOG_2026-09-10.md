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

- Corrected the `testssl` plugin's documentation above, after a live
  scan against a real target (following the redeploy of the previous
  entry) caught two real errors in what had just shipped:
  1. The claim that the six protocol findings "are always produced by
     the default full battery... and need no extra option" turned out
     to be false once `each_cipher` is also set. A real scan with
     `each_cipher: true` came back with `protocols`, `vulnerabilities`,
     `serverDefaults`, `fs`, `serverPreferences`, `browserSimulations`,
     `rating`, and `grease` all completely empty - only `pretest` and
     `cipherTests` had content. Traced this to testssl.sh's own
     dispatch logic: `count_do_variables` counts how many `do_*` test
     flags are set, and `set_scanning_defaults` (which turns on
     `do_protocols`, `do_server_defaults`, `do_vulnerabilities`, etc.)
     only runs when that count is exactly 0. `--each-cipher` sets
     `do_allciphers=true` on its own, so the count is 1 and the rest of
     the default battery never runs. The manifest now documents that
     building the full compliance report needs **two separate calls**:
     one plain call (no `each_cipher`/`cipher_per_proto`) for the
     protocol table, and one with `each_cipher` (or `cipher_per_proto`)
     set for the cipher-detail table.
  2. The claim that each cipher finding's text contains literal
     `Kx=`/`Enc=` substrings was also false - testssl's own
     `neat_list()` formatting function explicitly strips those
     prefixes before building the line. The real format (confirmed
     against the actual JSON from the live scan above) is a plain
     space-padded columnar line with no field prefixes: hexcode, the
     OpenSSL cipher name, the key-exchange column, the encryption
     algorithm alone (no parentheses or folded-in bit count), the bit
     strength as its own column, then the RFC-style name last - e.g.
     `x35     AES256-SHA                        RSA        AES
     256      TLS_RSA_WITH_AES_256_CBC_SHA`. This also caught a third,
     smaller error the same live data exposed: testssl's own
     key-exchange label for ephemeral/forward-secret Diffie-Hellman is
     `EDH`, not `DHE` as the previous entry assumed - confirmed from
     source (`kx[i] == "Kx=EDH"` guards used throughout testssl.sh).
     The manifest now tells the client to read CBC mode and MD5/SHA1
     MAC from the trailing RFC-style name column (which spells them
     out directly, e.g. `_CBC_`, `_SHA`) rather than from the
     encryption-algorithm column, and to treat an `EDH` key-exchange
     column as the report's "DHE" row.
  Also added, per direct request: guidance for rendering the
  protocol-support table as pass/fail marks instead of raw
  offered/not-offered text, per NIST SP 800-52 Rev. 2 (which
  deprecates SSLv2/SSLv3/TLS 1.0/TLS 1.1 and treats TLS 1.2/TLS 1.3 as
  currently secure): a cross when a deprecated protocol is offered (or
  a currently-secure protocol is not offered), a tick when a
  deprecated protocol is not offered (or a currently-secure protocol
  is offered).
  Manifest-only change, no Dockerfile/image change. Verified: `cargo
  build --locked --release --bins` and `cargo test --lib` (12/12)
  clean.
