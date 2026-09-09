# Changelog — 2026-09-09

- Added the `ike-scan` plugin, wrapping Debian bookworm's `ike-scan`
  package (1.9.5-1+b1, confirmed matching royhills/ike-scan's real latest
  git tag exactly - not stale) to discover and fingerprint IKE
  (ISAKMP/IPsec VPN) endpoints: Main Mode probing by default, optional
  Aggressive Mode, backoff-pattern implementation fingerprinting
  (`showbackoff`), and Aggressive Mode pre-shared-key hash capture
  (`pskcrack`) for offline cracking with the separately-packaged
  `psk-crack` utility (not yet built as its own plugin - see below).
  Read ike-scan's real C source directly (royhills/ike-scan) rather than
  assume from its man page: confirmed the default Main/Aggressive Mode
  path uses a plain UDP socket, not a raw socket (raw sockets are only
  used for a separate source-IP-spoofing feature this plugin doesn't
  expose) - so the only real elevated-privilege need is binding to the
  default UDP source port 500, a privileged port below 1024. This fleet's
  `cap_add` allowlist only supports `NET_RAW`, which doesn't help here,
  so `run_as_root = true` is set as a deliberate, scoped exception -
  matching the exact same kind of exception nmap's own manifest already
  documents, for a different underlying reason (nmap's raw-packet
  techniques need literal EUID 0; ike-scan just needs to bind a low
  port).
  Documented the well-known real Aggressive-Mode PSK-hash-disclosure
  technique (`aggressive` + `id` + `pskcrack`) as meaningfully more
  sensitive than a passive fingerprint - deliberately eliciting crackable
  key material from the target - needing the same authorization
  awareness §0 already requires for other higher-sensitivity techniques,
  even though no attack payload is sent. Also documented `ikev2` as
  upstream's own stated "experimental, not extensively tested, only the
  default proposal" limitation, and that `vendor`/`trans` can each only
  be supplied once per call through this schema even though ike-scan
  itself supports repeating either flag.
  No Docker build/live verification yet - this sandbox has no `docker`
  socket access, matching how every other plugin this session has been
  verified: build/install/live-test happens after merge, on the real
  deployed host.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12, including the existing `scan_plugins`-based manifest
  scan which parses every plugin directory) clean.
