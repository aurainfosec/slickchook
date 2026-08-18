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
