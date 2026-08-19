# Changelog — 2026-08-18

- Expanded the `nmap` plugin from ~6 options to nearly all of Nmap's
  real functionality, per an explicit request to add everything.
  Confirmed every flag live against the exact pinned 7.98 binary
  (downloaded the matching Ubuntu .deb + its shared-library
  dependencies directly, since docker isn't available in this sandbox)
  rather than guessed - this caught two real mechanical issues that
  would otherwise have shipped broken:
  1. Most of Nmap's actual functionality (`-sS`/`-sU`/every other scan
     technique besides connect scan, `-O` OS detection, `-f`
     fragmentation, several discovery probes) requires literal EUID 0
     inside the container, not just the `NET_RAW` Linux capability this
     manifest already granted - confirmed via Nmap's own "requires root
     privileges" errors under the fleet's usual unprivileged UID 65534.
     Added a new `DockerPolicy.run_as_root` field (`src/plugin_manager.rs`)
     and wired it through `slickchook-runner.rs` (both the main container
     run and its artifact-permission-fixup follow-up, which now also runs
     as root to match whichever UID the plugin container ran as) - a
     deliberate, scoped exception to this fleet's unprivileged-by-default
     container policy, set only for this plugin's manifest.
  2. Nmap's `-PS`/`-PA`/`-PU`/`-PY`/`-PO` discovery-probe flags require
     their port-list argument concatenated directly onto the flag with no
     space (`-PS80,443`), confirmed live: passing it space-separated (as
     this manifest's `flag`+value option rendering always does) gets
     silently misparsed as an extra scan target instead of the flag's own
     argument. These five are exposed as bare boolean options (Nmap's own
     default probe ports only) rather than accepting a custom port list,
     since a concatenated-value option isn't mechanically expressible in
     this schema - documented in both the plugin description and each
     option, rather than silently omitted or silently broken.
  Removed the previously-hardcoded `fixed_args = ["-sT"]`, replacing it
  with an explicit `connect_scan` option alongside every other scan
  technique (`syn_scan`, `udp_scan`, `ack_scan`, `window_scan`,
  `maimon_scan`, `null_scan`, `fin_scan`, `xmas_scan`, `sctp_init_scan`,
  `sctp_cookie_echo_scan`, `ip_protocol_scan`, `idle_scan`,
  `ftp_bounce_scan`, `list_scan`) - omitting all of them now lets Nmap
  pick its own default (SYN scan, now that root is granted) rather than
  always forcing connect scan.
  Also added: OS detection (`os_detection`/`osscan_limit`/`osscan_guess`),
  version-detection tuning (`version_intensity`/`version_light`/
  `version_all`/`version_trace`), the full timing/performance family
  (`min_rate`/`max_rate`/`host_timeout`/`scan_delay`/`max_scan_delay`/
  `min_rtt_timeout`/`max_rtt_timeout`/`initial_rtt_timeout`/`max_retries`/
  `min_hostgroup`/`max_hostgroup`/`min_parallelism`/`max_parallelism`),
  firewall/IDS evasion and spoofing (`fragment_packets`/`mtu`/`decoys`/
  `spoof_source`/`interface`/`source_port`/`proxies`/`data_hex`/
  `data_string`/`data_length`/`ip_options`/`ttl`/`spoof_mac`/
  `bad_checksum`/`send_eth`/`send_ip`), host-discovery probe types
  (`syn_ping`/`ack_ping`/`udp_ping`/`sctp_ping`/`ip_protocol_ping`/
  `icmp_echo_ping`/`icmp_timestamp_ping`/`icmp_netmask_ping`/
  `never_resolve_dns`/`always_resolve_dns`/`dns_servers`/`use_system_dns`/
  `traceroute`), port-selection shortcuts (`exclude_ports`/
  `fast_port_scan`/`sequential_ports`/`top_ports`/`port_ratio`), script
  tuning (`script_args`/`script_trace`), output formats (`xml_output`/
  `normal_output`/`grepable_output` - each must be set to an explicit
  `/artifacts/...` path to actually be retrievable afterward, since only
  that path is writable and persisted), `ipv6`, and `aggressive`
  (Nmap's own `-A` bundle).
  Explicitly not added: `-iL`/`-iR`/`--excludefile` (file/random-target
  input doesn't fit this plugin's single-authorized-target model),
  `--script-args-file`/`--script-updatedb`/`--script-help` (file-based or
  purely informational, not part of an actual scan run), `--resume`/
  `--append-output` (don't fit the one-shot, fresh-artifact-directory
  container model), `--stylesheet`/`--webxml`/`--no-stylesheet`/`-oS`/
  `-oA` (low value in a headless/API context), `--datadir`, `-d` (debug
  logging, very noisy), `--iflist`/`--noninteractive` (informational/
  no-op here), and `--privileged`/`--unprivileged` (redundant now that
  `run_as_root` controls this at the docker-policy level instead).
  Bumped `memory` 512m→1g and `timeout_seconds` 600→1800 given the much
  wider range of scan combinations now possible (e.g. `aggressive` mode,
  wide `udp_scan` port ranges); added a small `/tmp` tmpfs defensively.
  Updated `SKILL.md`'s Phase 2 guidance to describe the expanded
  capability and reiterate that broader/noisier options are for when an
  engagement specifically calls for them, not a routine default.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean. Also spot-checked several realistic
  manifest-generated argv combinations (default call, and a heavier
  service-detection + timing + XML-output combo) against the real
  extracted binary directly, confirming they parse and run as expected.
  This is a core Rust change (new `DockerPolicy` field, runner changes)
  as well as a manifest change - redeploying needs the full
  `build-deb.sh`/`apt install` path, not just `slickchook-plugin
  install`. Live verification of root-gated features (SYN scan, OS
  detection, etc.) actually working end-to-end inside the real
  container is still pending that redeploy, since this sandbox has no
  docker access to confirm beyond the argument-parsing/mechanics level.

- Fixed the artifact-permission-fixup step failing outright for
  root-mode plugins (`nmap`, from the change above): live-verified a
  real `-sS`/`-sV`/`-O` scan against `scanme.nmap.org` after the
  redeploy - the scan itself succeeded perfectly (real open ports, OS
  fingerprint, NSE script output), but the fixup step logged `find:
  /artifacts: Permission denied` even running as root. Root cause,
  confirmed against the real host rather than guessed: `/var/lib/
  slickchook/artifacts` carries a POSIX ACL (`setfacl -m u:65534:rwx`,
  in `scripts/install-ubuntu.sh`) granting the fleet's usual
  unprivileged uid explicit access, but no equivalent entry for uid 0 -
  and a `--cap-drop ALL` container's root has no `CAP_DAC_OVERRIDE`, so
  it's still subject to normal permission bits and falls through to
  `other::---` with nothing granted.
  This also surfaced a second, more serious latent bug while tracking
  it down: `scripts/build-deb.sh`'s postinst has **no ACL setup at
  all** for this directory, unlike `install-ubuntu.sh`. This host had
  clearly been bootstrapped via `install-ubuntu.sh` at some point (the
  same shape of gap as the runner setgid bug found earlier this
  session) - a genuinely fresh `.deb`-only deployment would have no
  ACL whatsoever, meaning even the ordinary unprivileged uid 65534
  case would fail on literally the first plugin run, not just the new
  root-mode one.
  Added the uid 0 grant to `install-ubuntu.sh`'s existing setfacl
  calls, and added the full ACL setup (both uid 65534 and uid 0, plus
  the matching default-ACL entries so new per-run subdirectories
  inherit them) to `build-deb.sh`'s postinst, which never had it.
  Verified: `bash -n` on both scripts, and a full `./scripts/build-deb.sh`
  rebuild confirming the `setfacl` calls are present in the built
  package's extracted postinst (`dpkg-deb -e`). Live re-verification
  (confirm the fixup succeeds cleanly end-to-end) pending redeploy.

- Expanded the `naabu` plugin from 5 options to nearly all of Naabu's
  real functionality, matching the same "add everything" scope as the
  `nmap` expansion earlier today. Researched against Naabu's actual
  source (`pkg/runner/options.go`/`validate.go`, `pkg/scan/option.go`,
  `pkg/privileges/privileges_linux.go`, `pkg/runner/nmap.go`) rather
  than guessed from prose, since no local Go toolchain/docker was
  available in this sandbox to run the real binary directly.
  Corrected a real, pre-existing bug in this manifest's own
  description: it claimed "unprivileged TCP CONNECT mode" while the
  actual default has always been `scan_type=s` (SYN).
  Notable findings baked into the new manifest:
  - `scan_type` takes the literal strings `s`/`c`, not the words
    `SYN`/`CONNECT`, despite what naabu's own help text implies -
    confirmed against `pkg/scan/option.go`'s `TypeSyn`/`TypeConnect`
    constants.
  - Unlike `nmap`, SYN scan here checks for the `NET_RAW` Linux
    capability directly (not just root) and gracefully falls back to
    CONNECT with a log line rather than hard-failing - most of Naabu's
    functionality doesn't need the `run_as_root` treatment `nmap`
    needed. The one exception: `discovery_only` (`-sn`,
    host-discovery-only mode) is confirmed in Naabu's own
    `ValidateOptions` to hard-error ("sudo access required") without
    real root. Set `run_as_root = true` anyway (cheap now that the ACL
    fix from the `nmap` work is already generic, not nmap-specific) so
    that works too, and so the new `nmap_cli` option's internal nmap
    follow-up also gets full privileged functionality (e.g. OS
    detection), not just Naabu's own scan.
  - `nmap_cli` (e.g. `"nmap -sV"`) runs a real nmap invocation as a
    follow-up pass on Naabu's discovered ports, merging service/
    version/OS results back in - the official `projectdiscovery/naabu`
    image bundles nmap + nmap-scripts specifically for this, so no
    extra Dockerfile work was needed.
  - `passive` (Shodan InternetDB, confirmed no API key required - a
    free public endpoint distinct from full Shodan) is a genuinely
    no-touch discovery alternative, worth calling out given this
    fleet's existing OSINT-first phasing.
  - `service_discovery`/`service_version` are deliberately not
    exposed: confirmed in Naabu's own source that these currently
    return a hard "not implemented" error - present as flags upstream,
    non-functional, not a gap on our part.
  - ProjectDiscovery Cloud Platform upload flags (`-auth`/
    `-dashboard`/`-team-id`/etc.) are deliberately excluded entirely -
    silently uploading authorized scan data to a third-party SaaS by
    default would be a real data-handling concern, not just low value.
  - Added `-no-stdin`/`-no-color` to `fixed_args` alongside the
    existing `-silent`: this fleet's runner never pipes stdin to a
    plugin container, and color codes are pure noise in a captured
    artifact.
  Bumped `memory` 512m→1g, `timeout_seconds` 600→1800, and `/tmp`
  tmpfs 64m→128m given the much wider scan surface and the added
  `nmap_cli` follow-up weight.
  Updated `SKILL.md`'s Phase 2 guidance (the old "unprivileged TCP
  CONNECT" description was stale there too) and the §3 tunables
  bullet to mention `naabu`'s own `threads`/`rate`.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean. This is a manifest-only change (no core Rust
  touched - `run_as_root`/the ACL fix already exist from the `nmap`
  work) - `slickchook-plugin install plugins/naabu` is sufficient, no
  full package rebuild needed. Live verification (a real scan
  exercising `scan_type=s` and `nmap_cli`) still pending that install,
  since this sandbox has no docker access to confirm beyond the
  option-schema level.

- Rewrote `SKILL.md` to remove every hardcoded plugin name from the
  methodology (§0, §2-5), replacing tool-specific references with
  capability-role descriptions the AI matches to whatever's actually
  installed via `list_plugins`. Reason: the installed plugin set varies
  per instance and changes over time (a tool gets swapped for a
  faster/better alternative - `subfinder`→`subfaster` earlier this
  session is exactly this - or only a subset of plugins is installed
  at all), so hardcoding "run `subfinder`, `amass`, `sublert`..." bakes
  in an assumption that breaks the moment that specific set isn't what
  a given deployment has.
  Checked first that this wouldn't lose any real knowledge: nearly
  every tool-specific nuance previously duplicated in `SKILL.md` (the
  `wpscan` flagged-vs-full-list distinction, `graphqlcheck`'s
  `guess_fields`, `gowitness`'s own explicit "always follow with
  screensanitize" instruction, `medusa`'s credential-testing framing,
  `shodan-search`'s scope requirement, `naabu`'s `nmap_cli`/`passive`)
  already lives in that plugin's own manifest `description` - exactly
  where this whole session's self-documentation effort has been putting
  it. `SKILL.md` now explicitly defers to `list_plugins`' descriptions
  as the authoritative source for tool-specific behavior, rather than
  duplicating (and risking drift from) what's already there.
  Two patterns that were previously named to two specific plugins
  (`medusa` for credential-testing, `shodan-search` for internet-wide-
  database queries) are now described generically by the underlying
  property ("performs a live authenticated action" / "queries a
  shared/third-party database not scoped to one target") so the same
  scope-discipline rule keeps applying correctly to whatever plugin(s)
  match that description in the future, not just these two by name.
  Verified: swept the rewritten file for every current plugin name
  (`grep` across the full fleet) - zero hits. Documentation-only
  change, no build/test impact.

- Added a new `virustotal-resolutions` plugin for origin-IP/WAF-bypass
  discovery: looks up VirusTotal's historical DNS resolution records
  for an authorized domain, surfacing an IP it resolved to before it
  went behind a WAF/CDN (e.g. RedShield) - useful for checking whether
  that origin is still directly reachable. Researched real alternatives
  first (SecurityTrails, Farsight DNSDB, Microsoft Defender TI/
  PassiveTotal, Censys, BinaryEdge, CIRCL, Onyphe, ViewDNS.info) before
  picking VirusTotal: it has a genuine free-tier API (4 req/min, 500/day)
  returning real dated historical resolutions, unlike most of the
  alternatives surveyed (DNSDB/Defender TI are enterprise-sales-only,
  BinaryEdge shut down entirely in March 2025, Censys's free tier
  excludes historical data, CIRCL requires manual vetting with no
  self-serve signup).
  This also investigated why `shodan-search` had seemed flaky in
  testing: confirmed (via Shodan's own current billing/API docs, and
  corroborated by multiple open issues on the exact Python library our
  plugin shells out to) that `/shodan/host/search` costs a query
  credit per call, and bare free-tier API keys get zero query credits -
  so a 403 there is expected account-tier behavior, not a bug in the
  plugin. Not changed in this PR; noted here since it's why this
  research happened in the first place.
  Built from source (`go install github.com/VirusTotal/vt-cli/vt@v1.3.1`,
  the official VirusTotal CLI, pinned rather than `@latest`) since no
  official Docker image exists. Its `--apikey` flag only accepts the
  key as a literal CLI argument (confirmed by reading vt-cli's own
  source - it never binds Viper's automatic environment-variable
  support), which would otherwise leak the key into this fleet's own
  argv logging (`stderr.txt`) - wrapped in an entrypoint script that
  reads `VT_API_KEY` from the environment instead and passes it
  internally, the same pattern already used for
  `wpscan`/`shodan-host`/`shodan-search`.
  Updated `SKILL.md`'s Phase 2 with a role-based (not name-specific)
  mention of this capability, consistent with the plugin-agnostic
  rewrite just landed.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean. Live verification (a real domain lookup)
  pending install, since this sandbox has no docker access.

- Fixed `virustotal-resolutions`' Docker build failing outright on
  install: `go install github.com/VirusTotal/vt-cli/vt@v1.3.1` errored
  `invalid version: unknown revision vt/v1.3.1`. Root cause, confirmed
  live: vt-cli's actual git tags are `1.3.1` etc., without the `v`
  prefix Go's module system requires to recognize a tag as a version
  at all - the assumption that it followed standard Go tagging
  convention was wrong. Pinned to the exact commit the `1.3.1` tag
  points to instead (`c7e4cdbde9778757444d96e6321785fa6972beed`) -
  equally reproducible, sidesteps the tag-naming mismatch entirely.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean (manifest description update only touched
  prose). Live build/install verification still pending.
