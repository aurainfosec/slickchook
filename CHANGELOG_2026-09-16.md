# Changelog — 2026-09-16

- Removed the `gospider` plugin, per direct request, after confirming
  it never had a working build on this deployment: per
  `CHANGELOG_2026-08-10.md`, its image "has never actually been built
  on this server (`docker` tried to pull it from Docker Hub instead of
  building locally)", and its real `--help` output was never verified
  against the actual binary as a result. No changelog entry since then
  ever recorded it getting fixed, and it was still on the
  never-installed list as of this week's plugin audit. `katana`
  (added 2026-08-19) already covers the same content-discovery role
  and better - it's JS-aware, which a plain HTML crawler like gospider
  never was - so this isn't a coverage gap, just removing a plugin
  that was never confirmed to actually work.
  Also updated `katana`'s own manifest description, which referenced
  gospider by name (as the tool it complements/improves on) - reworded
  to describe the capability generically ("a plain HTML crawler")
  rather than point at a plugin that no longer exists.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean.

- Reformatted the `testssl` plugin's `description` field for
  readability, per direct request after a broader "make our
  documentation more concise and clearer" discussion identified it as
  the single worst offender fleet-wide: a 6,931-character block of
  unbroken prose on one line, grown from several documentation PRs
  this week. Converted it to a TOML multi-line string broken into
  eight clearly-labeled paragraphs (general usage, the origin-IP
  false-negative/origin_ip explanation, the two-separate-calls
  requirement, call 1's protocol-table parsing rule, the NIST
  tick/cross rendering rule, call 2's cipher-table format, the
  per-row classification rules, and the cipher_per_proto/each_cipher
  performance note) instead of one continuous paragraph - same content,
  restructured for scannability, nothing added or removed. Confirmed
  via `tomllib`/`cargo test` that the parsed string value is unchanged
  in substance (6,891 characters, no stray trailing newline from the
  multi-line syntax, one clean paragraph break between each section).
  No other plugin was touched in this pass - it was scoped to this one
  file as a first test case before deciding whether to repeat it
  elsewhere.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12, including `scans_testssl_plugin_manifest`) clean.
