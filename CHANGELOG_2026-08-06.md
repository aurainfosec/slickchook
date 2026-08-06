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
- Fixed a real, universal (not plugin-specific) side effect of the
  post-run artifact permission fixup from PR #15, noticed while
  confirming the `dnsx` fix above worked end-to-end: every run gets a
  default `<artifact_dir>/input` directory created host-side by the
  slickchook service itself (`plugin_manager.rs`, unconditionally,
  whether or not the run actually uses input chaining) — owned by the
  `slickchook` service user, not the plugin container's UID 65534. The
  fixup's `find /artifacts -mindepth 1 -exec chmod a+rX {} +` walked
  into this directory too and failed trying to chmod something it was
  never going to own, logging `artifact permission fixup failed: chmod:
  /artifacts/input: Operation not permitted` on *every single run*,
  regardless of plugin. Harmless in practice (the slickchook service
  already owns that directory and can read it fine on its own, and the
  plugin's own actual output still got fixed up correctly regardless),
  but noisy and confusing — this exact log line is what led to
  wondering whether `dnsx` was still broken after the fix above, when
  it wasn't. `/artifacts/input` and everything under it is now excluded
  from the fixup's scope entirely.
  Verified for real: a container that writes a file and `chmod 600`s it
  (mirroring `testssl.sh`) alongside a host-created `input` subdirectory
  (mimicking the always-created staging directory) — the output file is
  still correctly fixed to readable, the `input` directory is left
  completely untouched, and no fixup error is logged.
- Investigated a client report that "large result payloads (e.g.
  waymore's) are serialized with unescaped control characters, producing
  invalid JSON that breaks strict parsers" — **could not reproduce it,
  and traced the full pipeline from producer to wire format without
  finding a defect anywhere**, documented here so this specific report
  isn't re-investigated from scratch if it resurfaces:
  - Ran a real, large `waymore` job (`-mode U --stream`) against
    `example.com` to completion. The resulting `stdout.txt` genuinely
    contains raw control bytes (confirmed via `read_artifact`: at least
    one NUL (`0x00`) and one ETX (`0x03`)), tracing back to `waymore`'s
    own source (`xnl-h4ck3r/waymore` `v8.9`, `linksFoundAdd`/
    `linksFoundResponseAdd` in `waymore.py`): archived URLs are
    URL-decoded and printed to stdout completely unsanitized, so a
    percent-encoded control byte (e.g. `%00`) in an archived URL decodes
    straight through with no filtering on waymore's end.
  - That confirms the client's premise that the *data* can genuinely
    contain control characters, but every layer downstream of it turned
    out to handle that correctly:
    - Slickchook's own code has no manual JSON string construction
      anywhere (checked `main.rs`/`plugin_manager.rs` directly) — every
      response goes through `serde_json::to_string`/`to_string_pretty`,
      which escapes every control byte per the JSON spec regardless of
      value.
    - `rmcp`'s SSE handler (pinned `0.5.0`,
      `transport/sse_server.rs::sse_handler`) serializes each JSON-RPC
      message with compact `serde_json::to_string` (not pretty — so no
      raw newlines from formatting whitespace either) before handing it
      to `axum::response::sse::Event::data(...)`.
    - `axum`'s `Event::data`/`write_buf` (pinned `0.8.9`,
      `axum/src/response/sse.rs`) operates at the raw byte level via
      `memchr::memchr2_iter(b'\n', b'\r', buf)`, correctly re-prefixing
      every line of a multi-line payload with `data: ` per the SSE spec
      — a real newline byte anywhere in the payload would still be
      framed correctly, not that one should ever appear here given the
      compact JSON above.
  - Verified the actual, complete real-world response (not a
    hand-built test case): every nesting layer of the ~65 KB truncated
    `stdout_preview` from the real `waymore` run — outer JSON-RPC
    envelope, the `start_plugin`/`plugin_status` job wrapper, and the
    innermost `PluginRunResult` — parsed successfully with a strict
    JSON parser.
  - If this resurfaces against a current build, the likely places left
    to look are the *client's own* MCP parsing implementation, or
    something specific to a payload/environment not reproduced here —
    not Slickchook's serialization or the pinned `rmcp`/`axum` SSE
    framing, which are now confirmed correct for arbitrary byte content.
- Fixed a real, live regression of the original `feroxbuster` wordlist
  bug — found while re-verifying `feroxbuster` was still deployed
  correctly, via a real `run_plugin` call that failed with `Could not
  open /wordlist.txt`, despite the manifest and installed image looking
  correct. Root cause, confirmed by inspecting the deployed image
  directly: `/wordlist.txt` genuinely exists at the right size, but with
  mode `640` owned `root:root` — unreadable by the plugin container's
  actual runtime UID `65534`, which is neither the owner nor in the
  `root` group.
  `slickchook-plugin install`'s own `normalize_package()` chmods every
  file in a staged package to `0640` before a `Dockerfile` ever builds
  (a deliberate host-side control, keeping installed packages
  root-owned and non-group-writable) — a plain `COPY` in a `Dockerfile`
  preserves that mode into the image unless the `Dockerfile` explicitly
  overrides it. `feroxbuster`'s `Dockerfile` was the one place this
  happened: `graphqlcheck`, `screensanitize`, and `sublert` all already
  `RUN chmod` their own `COPY`'d script immediately afterward (checked
  every plugin `Dockerfile`'s `COPY` instructions directly to confirm
  this is the only affected instance, not a wider regression).
  Fixed the `Dockerfile` (`RUN chmod 0644 /wordlist.txt`), and,
  separately, closed the actual gap that let this ship unnoticed:
  `check_assets_in_image`'s existing checks (`docker create`/`docker
  cp`) read a file via the daemon's own access, which bypasses Unix
  permission checks entirely — confirming a file exists and is the
  right size says nothing about whether the plugin's actual runtime UID
  can read it. Added a real runtime check (`docker run --user
  65534:65534 --entrypoint sh <image> -c "test -r <dest>"`) alongside
  the existing ones.
  Verified for real: the new check correctly fails against the
  already-built, still-broken image with a clear, actionable message
  before any Dockerfile fix was applied, confirming it actually catches
  this class of bug rather than being a theoretical improvement.
- Fixed `sublert` never actually querying the authorized target domain
  at all, reported by a client as returning a "502 Bad Gateway" and
  initially suspected to be nothing but crt.sh's own infrastructure
  being unreliable. A real `run_plugin` call reproduced a failure, but
  the actual stderr showed the container invoked as `["-d",
  "example.com"]` and `collect_ct.py` raising `HTTPError 404` — not 502.
  `sublert.toml` declared `target` as an `[[options]]` entry with `flag
  = "-d"`, so the runner correctly passed `-d example.com`, but
  `collect_ct.py` (this plugin's bespoke script, not the real upstream
  `sublert` tool) reads its domain from `sys.argv[1]` positionally, with
  no flag parsing at all — so `domain` was always the literal string
  `"-d"`, and every run queried crt.sh for the nonsense domain `%.-d`
  instead of the real target, regardless of crt.sh's own health.
  Separately confirmed crt.sh itself is also genuinely, intermittently
  returning 502 right now for well-formed queries too (tested directly:
  identical 502 for both the malformed `%.-d` query and a correctly
  formed `%.example.com` query in the same few seconds) — so the
  client's "unreliable upstream" read isn't wrong about crt.sh's current
  state, it just wasn't the only thing broken here, and this second bug
  would keep `sublert` permanently non-functional even once crt.sh
  recovers.
  Fixed by changing `target` from an `[[options]]` entry with `flag =
  "-d"` to an `[[arguments]]` entry with `position = 1`, matching how
  `collect_ct.py` actually consumes its input — confirmed against
  `plugin_manager.rs`'s `build_container_args`, which renders a
  flag-less `[[arguments]]` entry as a single bare positional token,
  unlike `[[options]]` which always emits the flag as a separate
  preceding token.
  Note for redeployment: this is a manifest-only change with no
  Dockerfile/image change, but the running server only ever reads
  manifests from `/var/lib/slickchook/plugins/` at startup (no
  hot-reload) — `sudo slickchook-plugin install
  plugins/sublert` is still required to pick it up, exactly as for an
  image change.
- Improved `SKILL.md` (doc-only, no code changes): documented that
  `read_artifact` returns an image artifact as a real MCP image content
  block alongside its metadata (not just base64 text) and that
  `list_run_artifacts` now lists nested files recursively; added
  guidance to skip `rawping` as a routine step (its ICMP echo is often
  filtered by the same infrastructure that would flag it, and rarely
  adds information `naabu`/`httpx` haven't already provided) rather than
  running it reflexively; noted `dnsx`'s `all` option covers every
  record type in one call instead of one call per type; and added
  parallelizing `read_artifact` calls during artifact collection to the
  existing efficiency guidance.
