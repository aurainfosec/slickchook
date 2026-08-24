# Changelog — 2026-08-24

- Added a new `waf-fingerprint` plugin - identifies which WAF/CDN (if
  any) fronts an authorized target, via `wafw00f`
  (github.com/EnableSecurity/wafw00f). Per a direct request to first
  check how effective the tool actually is before building around it -
  researched this honestly rather than assuming the most famous name
  in this space is automatically the right choice:
  - Confirmed actively maintained (last pushed 2026-04-19, latest
    release v2.4.2 in January 2026 with new WAF signatures added as
    recently as that month) - not stagnant.
  - Confirmed a real, important fact that was not previously
    documented anywhere and changes how this plugin must be scoped:
    reading its own source directly shows it is NOT a passive
    fingerprint - alongside passive header comparisons, it sends
    genuinely attack-shaped payloads (an XSS string, a time-based
    blind SQLi string, an LFI path-traversal string, an XXE payload,
    and an OS-command-injection-looking string) as part of its
    detection technique. Nothing is actually exploited, but these will
    very plausibly appear in a target's own WAF/SIEM logs as attempted
    attacks - documented prominently in the manifest, including the
    exact payloads, not glossed over as "just a fingerprint."
  - Confirmed a genuinely mixed, honest effectiveness picture rather
    than presenting it as infallible: its own commit/issue history
    shows a real, ongoing pattern of detection gaps being found and
    fixed (a FortiGuard false-negative fix in January 2026, a Google
    Cloud Armor detection gap in 2024, historical misses on
    StackPath/Siteground/F5 BIG-IP ASM) - documented as a real
    limitation, not implied to work more reliably than it does.
  - Checked whether the already-installed template-based vulnerability
    scanner's own bundled WAF-detection capability makes this
    redundant before building it - confirmed it has a single-request,
    single-payload-type WAF-detection template, meaningfully narrower
    than wafw00f's own multi-payload-type, 173-signature approach
    (some WAFs only react to certain payload categories) - genuinely
    complementary, not redundant, and documented as such.
  - Confirmed the PyPI-published package version matches the latest
    GitHub release exactly (`2.4.2` on both) - a plain version pin is
    safe here, unlike several other tools this fleet has had to pin by
    commit instead due to a stale package index.
  Added a new, generically-worded §0 scope-discipline category to
  `SKILL.md` for this real pattern - "anything whose detection
  technique itself sends deliberately attack-shaped payloads" -
  distinct from the existing credentialed-action and
  internet-wide-database categories, since it's a genuinely different
  kind of risk (log/alert noise on the target, not credential misuse
  or cost) that could apply to a future plugin too, not just this one.
  Checked with `grep` - no plugin/tool names added.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean. No docker access in this sandbox to build/run
  the image locally; live verification pending install.

- Live-verified `waf-fingerprint` after install with two real targets:
  `scanme.nmap.org` correctly returned `detected: false` (a bare test
  page with no WAF); `www.cloudflare.com` correctly returned `detected:
  true, firewall: Cloudflare`, with `trigger_url` showing the exact
  attack-shaped payloads (XSS/SQLi/path-traversal) documented in the
  manifest, sent as real URL parameters. Exit code 0 on both, clean
  JSON output matching the design exactly.

- Added a new `email-security-posture` plugin - checks an authorized
  domain's SPF, DKIM, and DMARC DNS records for common
  misconfigurations, purely passive DNS lookups only, no contact with
  the domain's actual mail infrastructure. Closes one of two remaining
  gaps identified in this session's own "what's still missing"
  review.
  Verified real record formats directly via live DNS-over-HTTPS
  queries against real domains (google.com/microsoft.com/github.com)
  before writing detection logic, rather than assuming textbook
  formats: confirmed real SPF records (including one split across
  multiple concatenated TXT strings due to the 255-byte string limit -
  `b"".join(rdata.strings)` handles this correctly), real DMARC
  records with p=/pct=/rua=/ruf=/fo= fields, and a real, important
  DKIM nuance - Microsoft 365 tenants often publish DKIM as a CNAME to
  a `<selector>-<domain>._domainkey.<tenant>.onmicrosoft.com` target
  rather than a direct TXT record, confirmed live that a standard
  TXT-type resolution already follows this transparently with no
  special handling needed.
  DKIM selectors have no canonical discoverable location in DNS at
  all - checks a list of selectors publicly documented as defaults by
  widely-used providers (Google Workspace, Microsoft 365, SendGrid,
  Mailchimp/Mandrill, Constant Contact, ProtonMail) plus generic
  self-hosted conventions, honestly documented as a best-effort guess
  rather than an exhaustive/authoritative check - a `selectors` option
  lets a caller add a real known selector from an actual email's
  DKIM-Signature header.
  Directly exercised the SPF/DMARC analysis logic (not just read for
  correctness) against the real record text captured from those live
  DNS queries, plus a battery of edge cases: a bare `all` mechanism
  (implicit `+`, distinct from an explicit `+all`), multiple SPF
  records, a missing `all` mechanism entirely, and a DMARC record with
  `pct<100` and no `rua=` - all produced the expected findings.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean. No docker access in this sandbox to build/run
  the image locally; live verification through the actual MCP server
  pending install.

- Added a new `jwt-analyzer` plugin - decodes a JWT's header/claims and
  offline-tests it for common weaknesses, via `jwt_tool`
  (github.com/ticarpi/jwt_tool). Closes the second of two remaining
  gaps identified in this session's own "what's still missing"
  review: several already-installed plugins (`dehashed`,
  `sourcemap-scanner`, `trufflehog`) flag "found a JWT-looking string"
  as a raw regex match, but nothing previously decoded or analyzed one.
  Researched honestly rather than assuming this well-known name is
  automatically the right, current choice: confirmed it is NOT
  actively maintained in the strict sense (last real commit/release
  v2.3.0, 2025-05-01, over a year stale) - but still the standard,
  most-referenced tool for this with no better-maintained alternative
  found. No PyPI package exists for it at all under either plausible
  name - built from source, pinned to that exact release tag, since
  there's nothing to check for staleness against a package index this
  time (the tag itself already is the latest real release).
  Confirmed directly from its own source, not assumed, that it is
  fully offline/local when its `-t`/`-ju` flags are omitted - grepped
  every network-call site in the file to confirm those two flags are
  the *only* ones that trigger any request at all - neither is exposed
  here, so `network = "none"` and this plugin can never reach a live
  target regardless of input.
  Also confirmed from source that its general interactive
  tamper/claim-injection modes (`-T`/`-I`) are built entirely around
  live `input()` prompts - dozens of them - which would simply fail in
  this fleet's non-interactive containers; deliberately not exposed.
  Its dedicated, non-interactive exploit/crack/sign flags (`-X`/`-C`/
  `-S`) cover the same practical ground (alg:none, null signature,
  psychic-signature CVE-2022-21449, blank-password, inline-JWKS
  injection, key-confusion, HMAC dictionary cracking, re-signing) and
  were confirmed clean of any interactive prompts by the same source
  read, not assumed safe by association.
  Uses jwt_tool's own real bundled wordlist of common/weak JWT signing
  secrets by default for cracking, matching this fleet's established
  preference for a tool's own real data over an invented list.
  This plugin never submits a crafted token anywhere itself - it only
  ever produces output (decoded claims, a crafted tampered token, a
  cracked secret); using one against a live target is left as a
  separate, deliberate follow-up call needing the same authorization
  as any other active test.
  Added a short, plugin-agnostic paragraph to `SKILL.md`'s Phase 4
  describing this "found a JWT, now analyze it" follow-up generically,
  alongside the existing dependency-manifest/CVE-lookup/source-map
  follow-ups already there - checked with `grep`, no plugin/tool names
  added.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean. No docker access in this sandbox to build/run
  the image locally; live verification pending install.
