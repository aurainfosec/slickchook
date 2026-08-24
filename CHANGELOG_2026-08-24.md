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
