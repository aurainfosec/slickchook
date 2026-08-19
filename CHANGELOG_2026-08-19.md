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

- Fixed two real bugs in `katana` found on the very first live test
  against `scanme.nmap.org`:
  1. `known_files` is a single-choice value, not a comma-joinable
     list - passing `"robotstxt,sitemapxml"` errored outright
     (`invalid value ... allowed values are , all, robotstxt,
     sitemapxml`). The `--help` text's `(all,robotstxt,sitemapxml)`
     formatting looked identical to other options in this fleet that
     genuinely do accept a joined list (e.g. naabu's `ip_version`),
     but isn't one here - corrected the option's description to say
     so explicitly, confirmed live.
  2. Every call failed with `Could not read flags: cause="no such
     file or directory"` - Katana tries to read a default config file
     relative to `$HOME` at startup, which is unset under this
     fleet's usual unprivileged UID (no matching /etc/passwd entry) -
     the same class of bug already hit and fixed for
     `wpscan`/`shodan-host`/`subfaster`, just missed here since this
     plugin had been referencing the official image directly with no
     wrapper at all. Added a thin `Dockerfile`
     (`FROM projectdiscovery/katana:v1.7.0` + `ENV HOME=/tmp`) and
     changed `container` to the resulting local image - the smallest
     possible fix, no other customization needed.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean. Live re-verification (a real crawl succeeding
  end-to-end) pending redeploy.

- Documented every real bug found while building/fixing plugins this
  week directly in `PLUGINS.md`, so future plugin work benefits from
  them instead of repeating them:
  - Corrected the existing `$HOME`/read-only-filesystem guidance,
    which only described mounting a tool's specific `/.config`/`/.cache`
    subpath - added the simpler `ENV HOME=/tmp` fix (one setting
    handles any path, no need to discover the tool's exact subpath)
    for whenever a Dockerfile is actually controlled, reserving the
    specific-subpath-mount approach for pulled-image-only plugins with
    no Dockerfile at all.
  - Fixed a now-stale claim that "plugin containers always run as UID
    65534" (no longer true since `run_as_root` was added) and added a
    full explanation of when that field is actually warranted (`nmap`)
    versus not (`naabu`, which checks the `NET_RAW` capability
    directly and falls back gracefully), plus the artifacts-ACL
    dependency it has on the host.
  - Added a new "Verify against the tool's real behavior, not its
    docs or `--help` text" section covering every other bug class hit
    this week as a standing checklist: docs/generated help pages can
    be stale relative to an exact pinned ref (`virustotal-resolutions`);
    a tool can need a one-time init step before its real commands even
    exist (`vt-cli`, `shodan`); a flag that looks comma-combinable in
    `--help` text might be a strict single-choice enum instead
    (`katana`'s `known_files`); a release tag doesn't guarantee it's a
    valid Go module version (`vt-cli`'s unprefixed tags); the install
    step never runs the container for any plugin, so nothing is
    verified until a real live call happens; and correcting a wrong
    fix should be a new, honest entry, not a silent edit to history
    (the `shodan-host`/`shodan-search` `setuptools` episode).
  Documentation-only change, no build/test impact.
