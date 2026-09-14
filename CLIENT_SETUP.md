# Setting up an MCP AI client against Slickchook

This covers the client side of a Slickchook deployment: getting an AI
client's MCP connection working against a deployed instance, loading the
recon methodology into it, and the handful of tasks that come up
repeatedly once it's connected. For the server side (installation, the
Caddy TLS front-end, plugin secrets), see `RUNNER.md`. For the intended
architecture this setup implements, see `ARCHITECTURE.md`.

## 1. Resolve the hostname

Caddy's TLS site block is keyed to a specific hostname (`CADDY_HOSTNAME`
at install time, default `slickchook`), not a bare IP. TLS clients only
send an SNI extension for a hostname, never a literal IP address, so
connecting straight to the VM's IP fails the handshake outright — there's
no SNI for Caddy to match a certificate against.

Give the client machine an `/etc/hosts` entry (or a real DNS record)
mapping that hostname to the server's actual address:

```
203.0.113.10   slickchook
```

## 2. Establish TLS trust

The certificate at `/etc/caddy/tls/server.crt` is self-signed — there's no
public CA to fall back on, so this step decides the whole security of the
connection.

**Recommended: pin it at provisioning time**, matching `RUNNER.md`'s own
stance on this. Whatever already has an authoritative channel to both
sides (you, provisioning the client and having deployed the server) can
just hand the client the real thing directly:

```bash
# From a channel you already trust (direct SSH/console to the slickchook
# host, not the network path the MCP client will actually use):
scp slickchook-host:/etc/caddy/tls/server.crt ./slickchook.crt
# or, if you only need to confirm/compare rather than transfer the file:
ssh slickchook-host 'sha256sum /etc/caddy/tls/server.crt'
```

Configure the client to trust *only* this specific certificate for this
specific connection — not by adding it to the system-wide trust store
(that would make the client trust every instance signed the same way,
which is more than this one pairing needs). Exactly how depends on the
client: some accept a path to a CA/cert bundle in their MCP server
config, others expose a certificate-pinning or fingerprint-pinning option
specifically. Whichever mechanism it offers, feed it the file or the
fingerprint fetched above — never a value read off the same network path
the real connection will use.

**Weaker fallback: trust-on-first-connection.** If there's genuinely no
side-channel available, accept the certificate presented on the first
real connection and pin its fingerprint for every connection after that
(the same model SSH host keys use). This is weaker than provisioning-time
pinning for exactly the reason `RUNNER.md` already states: whatever
intercepts that first connection is trusted forever after, with no way to
tell the difference from a legitimate first connection. Only fall back to
this if provisioning-time pinning genuinely isn't available, and treat
that first connection as the actual point of trust decision — don't do it
somewhere routine or low-attention.

The certificate is generated once and left in place across reinstalls, so
a pin made now keeps working. It only changes if `/etc/caddy/tls/server.*`
is deleted to force reissuance — re-pin if that ever happens.

## 3. Point the client at the MCP endpoint

```
https://slickchook:8443/sse
```

This is an SSE transport: the client opens that URL as a long-lived event
stream, which the server responds to with a session endpoint; JSON-RPC
requests (`initialize`, then `tools/call` for each tool) are POSTed to
that session endpoint, and responses stream back over the original SSE
connection rather than as each POST's own response body (see
`ARCHITECTURE.md`'s client-connection diagram for the full picture — this
is what makes a long-running plugin call return its final result
asynchronously rather than blocking the request that started it).

To sanity-check the endpoint independent of any particular client:

```bash
curl -N -k --resolve slickchook:8443:<server-ip> https://slickchook:8443/sse
```

(drop `-k` once the certificate is actually pinned rather than just
skipped — this is a smoke test, not how a real client should connect.)

## 4. Load `SKILL.md` into the client's context

Slickchook doesn't serve `SKILL.md` over the MCP protocol itself — there's
no resource or prompt endpoint for it, only the plain repo file. Loading
it is a step you do once per client, using whatever mechanism that
client offers for standing instructions/system context: a system prompt,
a project's custom instructions, an auto-loaded instructions file, or
similar. Copy `SKILL.md`'s current content in verbatim; don't paraphrase
it, since its phrasing (the scope-discipline gates in particular) is
deliberately precise.

`SKILL.md` is written to stay plugin-agnostic — it describes methodology
by capability role, not by naming specific plugins — so it doesn't need
reloading just because a plugin was added or changed. It only needs
reloading if `SKILL.md` itself changes (a new phase, a new scope-discipline
category, etc.) or if you're setting up a new client for the first time.

There's no one path, because there's no one client — each one has its own
place for standing instructions, and only some of them are even a file on
disk. A few common ones, as of this writing (check that client's current
docs if this has changed):

- **Claude Code**: a `CLAUDE.md` file, auto-loaded at session start. Put
  it at `~/.claude/CLAUDE.md` (user-level — applies everywhere this client
  runs, the natural choice since `SKILL.md` is plugin-agnostic methodology,
  not something scoped to one project) or `<project-root>/CLAUDE.md` if
  you only want it active while working in one particular repo/workspace.
- **Cursor**: a Project Rule under `.cursor/rules/`, e.g.
  `.cursor/rules/slickchook-skill.mdc`, with `alwaysApply: true` in its
  frontmatter so it's always in context rather than fetched on demand. The
  older single-file `.cursorrules` at the project root still works too.
- **Windsurf**: `.windsurfrules` at the project root.
- **GitHub Copilot Chat**: `.github/copilot-instructions.md` at the
  repository root.
- **Claude Desktop / claude.ai Projects**: no file path — this is a text
  field ("Custom instructions") in a Project's settings in the web/desktop
  UI. `claude_desktop_config.json` is unrelated to this step; that file
  only registers MCP servers, it doesn't carry instructions.
- **A custom client built directly on the Claude API**: no file either —
  pass `SKILL.md`'s content as (or appended to) the `system` parameter of
  the Messages API call.

## 5. Refresh the plugin list after a redeploy

A running MCP client session caches its tool list from the `initialize` /
`tools/list` exchange it did when the session opened. Slickchook doesn't
send a `tools/list_changed` notification when the plugin roster changes
on the server side (confirmed from source — there's no such notification
anywhere in the MCP server implementation), so an existing session
genuinely will not see a newly-installed or newly-updated plugin on its
own.

After any `slickchook-plugin install` on the server (a new plugin, or a
changed manifest/Dockerfile for an existing one), the client needs a
**fresh MCP session** to pick it up: disconnect and reconnect (a new
`/sse` connection, followed by a fresh `initialize` and `tools/list`),
rather than continuing to use the one already open. There's no partial-
refresh option — it's a full reconnect or nothing.

## 6. Common tasks once connected

These all build on the same small set of tools — `run_plugin` /
`start_plugin` / `plugin_status`, `list_runs_by_target`, `list_run_artifacts`,
`read_artifact` — already covered in `SKILL.md`. This section is about how
to point them at five things people actually ask for at the end of an
engagement.

### Reviewing (and optionally clearing) prior history for a target

Before starting fresh work against a target, check what's already there:

```json
{"target": "example.com", "limit": 50}
```
called against `list_runs_by_target` — returns every recorded run for an
exact target match, most recent first, each with its plugin, status,
exit code, timestamps, and the exact input it was called with. If prior
runs exist, that's a real decision point: keep them (new runs just add
alongside), or call `purge_runs_by_target` to delete them first — only
ever the latter with the human's explicit confirmation, since it's
irreversible; never call it automatically.

### Timeline

`list_runs_by_target`'s own output *is* the timeline — it's already
sorted most-recent-first and each record carries `started_at`/
`finished_at`, `plugin`, `status`, `exit_code`, and `input_json` (what was
actually run, so a later reviewer isn't left guessing). There's no
separate "generate a timeline" tool; the client assembles the readable
narrative from this data (chronological order, which phase each run
belongs to, what it found) — the raw material is complete, the write-up
is a synthesis step.

### Recon details / attack surface report

Same starting point as the timeline — `list_runs_by_target` for the
target(s) in scope, then `read_artifact` against the runs that matter —
but this is entirely a client-side synthesis task, not a built-in export.
Slickchook has no "attack surface report" tool; it hands back raw run
records and raw artifact content, and `SKILL.md`'s own §4 ("Summarize
findings and recommended actions") is the guidance for turning that into
the actual write-up: what was found, across which phases, what's worth a
human's attention first and why.

### An importable Burp Suite URL list

Run a URL-export-capable plugin (see `PLUGINS.md`) against the URLs
discovered so far, then retrieve its output via `list_run_artifacts` /
`read_artifact`. The result is a plain-text list, one URL per line —
`RUNNER.md`'s own "Importing exported URLs into Burp Suite" section
covers exactly how that file goes into Burp (native bulk scope import,
the **Add to SiteMap+** BApp, or a universal proxy-replay fallback) — see
that section rather than duplicate it here.

### The full artifact archive

Walk every run recorded for the in-scope target(s) — `list_runs_by_target`
to enumerate them, `list_run_artifacts` for each `run_id` to see what it
wrote, `read_artifact` to pull each file back. Nothing is filtered or
summarized in this path; it's the complete, unmodified set of whatever
each plugin actually wrote under `/artifacts` for every run, kept for
reference and hand-off. This is `SKILL.md`'s §5 in practice — the
companion, raw-data step to the summary/report above, not a replacement
for it.
