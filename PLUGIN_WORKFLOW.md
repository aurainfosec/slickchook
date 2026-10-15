# Plugin-building workflow

A proposed end-to-end outline for building or changing a Slickchook
plugin, from a known-good baseline through to verifying the change from a
real MCP client. This formalizes the process this project has actually
used throughout its development so far - it's not a new process, just a
written-down one, open to refinement.

## 0. Start from a stable deployment

Before starting new work, confirm the currently-deployed instance is
actually healthy - not just "the process is running," but genuinely
reachable end to end:

```
systemctl status slickchook caddy --no-pager
```

and a real MCP round-trip through the actual TLS front-end (not a
loopback shortcut - see "Test from an MCP AI client" below for why that
distinction matters):

```
curl -sk --resolve <hostname>:8443:127.0.0.1 https://<hostname>:8443/sse
```

Starting from a confirmed-good baseline matters for two reasons: it gives
a real rollback point, and it stops a *pre-existing* instability from
being misread as caused by the change you're about to make. (This
happened for real during this project's development: a backend restart
loop that predated a plugin install was initially hard to distinguish
from the redeploy itself, until the systemd journal timestamps made the
actual sequence clear.)

## 1. Clone or sync the repository locally

```
git clone https://github.com/aurainfosec/slickchook.git   # first time
git checkout main && git pull --ff-only                    # afterwards
git checkout -b plugin/<name>                               # or fix/..., docs/...
```

One branch per logical change, named for what it does
(`plugin/ike-scan`, `docs/architecture-diagrams`, `fix/commix-stdin`).

## 2. Implement and verify locally, before opening a PR

Write or edit the plugin's `<name>.toml` manifest and, if needed, its
`Dockerfile`/entrypoint script. `PLUGINS.md` covers the manifest schema,
the secrets/assets conventions, and - importantly - its own checklist for
verifying a wrapped tool's *real* behavior from its actual source or a
real run, rather than trusting its `--help` text or README. That checklist
has caught real bugs on nearly every plugin built so far; it's not
optional polish.

If the change touches `SKILL.md`, keep it plugin-agnostic: describe
methodology by capability role ("a template-based vulnerability scanner,"
"a WAF/CDN fingerprinting plugin"), never by hardcoding a specific plugin
name. The reason is structural, not stylistic - the MCP client is meant
to enumerate what's actually available and how to use it by calling
`list_plugins` and reading each plugin's own manifest description, which
is always current by construction. A static prose reference to a named
plugin in `SKILL.md` is a second copy of that information that can
silently drift out of sync as plugins are added, renamed, or changed -
self-documenting, enumerable plugins are the point; don't undermine that
by re-describing individual plugins in fixed documentation elsewhere.

Before a PR is opened, it should already be clean:

```
cargo build --locked --release --bins
cargo test --lib
```

`cargo test --lib` includes a manifest-scanning test that parses every
plugin directory under `plugins/` - a malformed or schema-invalid TOML
file fails the whole suite, so this alone catches a large class of
mistakes without needing Docker at all. Where the actual container can't
be built or run locally (no `docker` socket access in some environments),
verify what can be verified without it: read the wrapped tool's real
source rather than assume from its docs, and test any wrapper logic
(argument rewriting, config-file generation, etc.) directly against a
stand-in in place of the real binary.

If a `Dockerfile` is involved, avoid BuildKit-only syntax: `ARG
TARGETARCH` and `COPY --chmod=...` are the two that have already caused a
real, hard-to-reproduce failure (see `CHANGELOG_2026-10-15.md`'s
BuildKit-only-features entry and the linked issue). The `.deb`'s declared
dependency (`docker.io | docker-ce`) lets `apt` resolve `docker.io` on
Ubuntu, which has no `buildx` and therefore runs the legacy builder -
`ARG TARGETARCH` silently expands to empty instead of failing, and `COPY
--chmod` is rejected outright. A plugin built and tested with Docker
Desktop or `docker-ce` plus `buildx` (buildx is on by default there) will
look completely fine locally and only break on the installer's own
deployment path. Detect the architecture at build time instead (`apk
--print-arch` for an Alpine base image), and split `COPY --chmod=...`
into a plain `COPY` followed by its own `RUN chmod`.

## 3. Submit a pull request

One logical change per PR. Include a changelog entry in the day's
`CHANGELOG_<date>.md` describing what changed, why, and how it was
verified - correcting an earlier entry means writing a *new* entry, never
silently editing history. Wait for review and merge; this project treats
merge as a real human-in-the-loop gate, not a formality.

## 4. Update the local repository and local deployment after merge

```
git checkout main && git pull --ff-only
```

Then redeploy, matching the size of the change (`RUNNER.md` covers both
paths in detail):

- **Manifest, Dockerfile, or entrypoint script only** - a lightweight,
  scoped update:
  ```
  sudo slickchook-plugin install plugins/<name>
  ```
- **Core Rust/service change** - a full rebuild and reinstall via the
  `.deb` package (`RUNNER.md`'s recommended path; `install-ubuntu.sh` is
  for local dev-iteration on a host you already control, not this step):
  ```
  cargo build --locked --release --bins
  ./scripts/build-deb.sh
  sudo apt install ./dist/slickchook_*_$(dpkg --print-architecture).deb
  ```

Either path restarts the `slickchook` service, which drops any live MCP
session. Confirm the service actually settled afterward (a fresh
`Active: active (running) since <just now>` timestamp, no flapping)
before treating the redeploy as done - and avoid stacking multiple
redeploys back-to-back without letting the service settle in between,
since each one orphans whatever session a client had open.

## 5. Test from an MCP AI client

This is the step that actually exercises the real deployment, as opposed
to a host-side diagnostic shortcut. The distinction matters: earlier
diagnostic work this session used a hand-rolled script talking to the
backend directly over loopback HTTP, bypassing the TLS front-end
entirely - useful for fast iteration, but it can't catch a problem
isolated to the TLS/reverse-proxy layer, or anything that only shows up
over the actual client-facing path.

The real test target is:

- an MCP client equipped with this project's own `SKILL.md` methodology
  (so it exercises the plugin the way an operator actually would - as
  part of a phase in the recon workflow, not in isolation)
- running in its own container, matching the intended architecture (see
  `ARCHITECTURE.md`) rather than colocated with the slickchook host
- connecting to the slickchook MCP endpoint over the network, through
  Caddy's TLS front-end (`https://<hostname>:8443`) with the correct SNI
  hostname - not a loopback/backend-direct shortcut

From that client, confirm:

1. `list_plugins` shows the new/changed plugin, with the manifest's real
   description and options rendering correctly - this is the client
   discovering the plugin's capability directly from its own
   self-description, not from a separate written-down reference that
   could be stale.
2. `run_plugin` (or `start_plugin` / `plugin_status` for a long-running
   one) against an authorized test target produces the expected real
   output.
3. `list_run_artifacts` / `read_artifact` retrieve the run's actual
   artifacts correctly.

If this step finds a real bug, fix it via a **new** PR and changelog
entry - loop back to step 1 - never a silent edit to an already-merged
change.

## 6. Iterate

Repeat from step 1 for the next change, starting again from a confirmed
stable baseline.
