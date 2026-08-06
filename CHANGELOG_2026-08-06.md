# Changelog — 2026-08-06

- Fixed the `dnsx` plugin, reported by a client as broken with `-d flag is
  dnsx's brute-force mode (needs -w/wordlist), not simple single-host
  resolution`. Two separate, compounding bugs, both traced to actual
  upstream source rather than guessed at:
  - The manifest mapped its `target` option to `-d` (bruteforce-domain
    mode, requires `-w`) instead of `-l` (simple host/subdomain
    resolution) — matching the client's report exactly. Also fixed
    `record_type`, which mapped to a `-query <type>` flag that does not
    exist in dnsx at all; replaced with the real per-record-type boolean
    flags (`-aaaa`, `-cname`, `-ns`, `-txt`, `-mx`, `-srv`, `-ptr`,
    `-soa`, `-caa`, `-any`, `-all`).
  - Even after that fix, `-l <literal-host>` produced no output at all
    under the plugin's real Docker hardening flags (silent, clean exit,
    no error) — and, confusingly, sometimes not even without them,
    contradicting an earlier apparent success. Read the actual dnsx
    source at the pinned `v1.2.2` tag (`internal/runner/{options,runner}.go`)
    to find the real cause rather than continuing to guess from empirical
    trial and error alone: `-auth` defaults to `"true"`, unconditionally
    triggering an interactive "Enter PDCP API Key" prompt on every
    invocation; with no TTY (as in any non-interactive `docker run`) this
    either fails loudly (`Could not read input from terminal: open
    /dev/tty`) or, apparently, silently aborts the rest of execution
    depending on circumstances neither traced not worth fully
    characterizing given the fix. Separately, and independently: v1.2.2
    never actually implemented passing a literal host string to `-l` at
    all — GitHub's v1.3.0 release notes explicitly list "Added inline and
    comma-separated host input support" as a *new* feature, confirming
    the one early "successful" test earlier in this investigation was a
    fluke of unsupported/undefined behavior, not a working code path.
    Docker Hub's `projectdiscovery/dnsx` was never updated past `v1.2.2`
    (confirmed via matching image digests for `latest` and `v1.2.2`), so
    fixing this required building `dnsx` locally from the v1.3.0 GitHub
    release binary (`plugins/dnsx/Dockerfile`, matching the existing
    `feroxbuster`/`kiterunner` local-build pattern) rather than simply
    bumping a pinned upstream tag. `fixed_args` now includes `-auth=false`
    (note: `=`, not a space — a bare space-separated `-auth false` still
    triggered the interactive prompt, likely due to how goflags'
    `DynamicVar` flag type binds its value) and `-duc` to keep every run
    fully non-interactive and network-independent of ProjectDiscovery's
    own infrastructure.
  Verified for real, repeatedly: 3 consecutive runs against the actual
  built image, using the exact Docker hardening flags production uses
  (`--user 65534:65534 --read-only --cap-drop ALL --security-opt
  no-new-privileges`, resource limits, the manifest's own `tmpfs`), all
  producing correct, fresh DNS results with no flakiness.
