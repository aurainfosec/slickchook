# Security considerations

This is a living record of Slickchook security assumptions, controls, and open
concerns. Update it whenever a security-relevant behavior is introduced,
discovered, fixed, or deliberately accepted.

Last reviewed: 2026-08-06

## Trust boundaries

- The MCP client and all plugin parameters are untrusted.
- A plugin package and its manifest are untrusted until the privileged plugin
  installer has copied and validated them.
- Installing a plugin that contains a `Dockerfile` is a privileged operation.
  The build uses the Docker daemon and must be treated as installing trusted
  code, not merely configuration.
- Installed manifests are intended to be controlled by `root` and readable,
  but not writable, by the `slickchook` service account.
- The setgid runner and access to the Docker socket are privilege boundaries.
  Compromise of either can provide control equivalent to Docker daemon access.
- Plugin containers are untrusted even after their manifests are approved.

## Existing controls

- The runner invokes Docker with structured process arguments; neither MCP
  parameters nor manifest arguments are evaluated by a host shell.
- MCP input keys must be declared by the plugin manifest. Values must be JSON
  scalars, and user-supplied strings starting with `-`, containing NUL, or
  being empty are rejected.
- Container image names, Docker networks, tmpfs declarations, and added
  capabilities are validated. Only `NET_RAW` can currently be added (used by
  `naabu` for SYN scans and `nmap`'s raw-packet checks; prefer TCP CONNECT
  modes when raw-packet scanning isn't required for the engagement).
- Containers run with all capabilities dropped by default, a non-root user, a
  read-only root filesystem, `no-new-privileges`, and configured CPU, memory,
  PID, and time limits.
- The runner canonicalizes the manifest and artifact paths and checks that they
  are beneath their configured roots.
- The plugin installer rejects package contents that are not regular files or
  directories, normalizes installed ownership and permissions, serializes
  changes with a lock, stops the service while switching packages, installs by
  directory rename, and rolls back when service validation fails.
- Database configuration is never hard-coded in manifests or source: the
  installer provisions a dedicated non-superuser PostgreSQL role and
  database, and writes non-secret connection settings to
  `/etc/slickchook/postgres.env`, owned by `root:slickchook` with mode
  `0640`. Passwords, if later required for remote connections, must be
  supplied out-of-band through a secret manager or protected deployment
  mechanism.
- Plugins that need a persistent credential for an external service (for
  example, a Shodan API key) declare it in their manifest as `[[secrets]]`
  — an environment variable name and description, never a per-call
  `option`/`argument` a client can supply. Values are configured once,
  server-wide, in `/etc/slickchook/plugin-secrets.env`, owned by
  `root:slickchook` with mode `0640` and loaded via the same
  `EnvironmentFile=` systemd mechanism as `postgres.env`. A configured
  value is only ever forwarded into a container as `docker run --env NAME`
  (the bare variable name, no `=value`), so it flows entirely through
  inherited process environment at every hop and never appears in any
  argv, log line, or `plugin_runs` record — unlike the client-supplied
  credentials described in SC-005. `list_plugins` reports whether each
  declared secret is currently configured as a boolean only, never the
  value itself, and a run fails immediately, before any container is
  created, if a required secret is missing.
- Analyzer plugins (e.g. `osvscanner`, `screensanitize`) receive `/input` as a
  read-only mount by default; only a purpose-built fetcher may set
  `docker.input_read_only = false`, and that mount is still constrained to
  the current run's own artifact directory. This separates networked
  collection from offline analysis and prevents an analyzer from modifying
  collected content or reaching arbitrary host paths.

## Open concerns

### SC-001: Plugin source validation and copy TOCTOU

Status: Open  
Severity: High when plugin source is writable by an untrusted contributor

The root plugin installer validates a source directory and subsequently copies
it. A process able to modify that source can replace files or introduce links
between validation and copying. The copied staging tree is normalized after the
copy, but the source-to-staging operation is not yet an immutable snapshot with
no-follow guarantees.

Recommended hardening:

- Open package entries without following symlinks and copy from verified file
  descriptors.
- Reject hard links, unexpected owners, special files, and changes detected
  during the copy.
- Validate the complete root-owned staging snapshot after copying and build
  only from that snapshot.
- Keep the final installation as an atomic rename of the validated snapshot.

### SC-002: Manifest validation/use mismatch

Status: Open  
Severity: High if an installed manifest can be replaced or modified

The MCP server caches a parsed and validated manifest. At execution time it
passes the manifest pathname to the privileged runner, which opens and parses
the file again. A replacement between those operations can cause the runner to
use content different from the content approved and exposed by the MCP server.
Root-owned, non-group-writable installed permissions reduce this risk but do
not remove the validation/use gap.

Recommended hardening:

- Record a cryptographic digest of the exact manifest bytes loaded by the MCP
  server and pass the expected digest to the runner.
- Have the runner open the manifest once with no-follow semantics, validate its
  type, owner, mode, and location, read it from that descriptor, verify the
  digest, then parse and use only those in-memory bytes.
- Apply equivalent descriptor-based checks where mutable filesystem paths cross
  into privileged runner behavior.

### SC-003: Dockerfile plugin installation is highly privileged

Status: Open/documentation requirement  
Severity: Critical if arbitrary plugin authors are treated as trusted

The plugin installer runs `docker build` as root against plugin-provided build
instructions. Docker daemon access is effectively host-root-equivalent in the
normal deployment model. Container runtime restrictions do not make an
untrusted image build safe.

Recommended hardening:

- Require explicit operator approval for local image builds.
- Prefer reviewed, digest-pinned images built by a separate CI system.
- Consider disabling Dockerfile builds by default and enabling them only via an
  explicit installer option.
- Verify image provenance or signatures where practical.

### SC-004: Mutable image tags

Status: Open  
Severity: Medium to high depending on registry trust

Some manifests may reference tags rather than immutable image digests. A later
pull can retrieve different code without a corresponding manifest change.

Recommended hardening:

- Pin production plugin images by digest.
- Record the resolved image digest at installation and execution time.
- Define an explicit, auditable update workflow for changing image digests.

### SC-005: Sensitive plugin inputs are retained in plaintext

Status: Open  
Severity: High when a plugin accepts credentials, tokens, or other secrets

The MCP server records the complete plugin input JSON in the `plugin_runs`
PostgreSQL table. Plugins such as Medusa accept a password, so even a test run
persists that value in plaintext. Artifact output may independently disclose
the same inputs or other sensitive target information.

Recommended hardening:

- Add manifest metadata that marks individual inputs as sensitive.
- Redact sensitive values before writing run history, logs, errors, or MCP
  responses; retain only an explicit marker that a value was supplied.
- Document database and artifact retention, ownership, backup, and deletion
  requirements.
- Do not submit production credentials until input redaction is implemented and
  verified.

### SC-006: Artifact filename interference

Status: Mitigated in source; deployment pending  
Severity: Medium

Plugin containers share the per-run artifact directory. A container could
create or replace the reserved `stdout.txt` or `stderr.txt` paths before the
server records process output. Artifact recording now rejects symlinks and
non-regular files and writes through an explicit truncating file handle.

The updated Slickchook server binary must be deployed before this mitigation is
active on a running instance.

### SC-007: Offline analyzer database supply chain

Status: Mitigated with maintenance requirements  
Severity: Medium

OSV-Scanner bundles an OSV database into its image so runtime analysis does not
require network access. The scanner and database snapshot must be updated and
reviewed together. Rebuild with `--no-cache`, record the source revision and
image digest, and keep analyzer networking disabled.

### SC-008: Sensitive report artifacts

Status: Accepted risk  
Severity: High impact if artifact access is misconfigured

The artifacts directory is intentionally treated as report data. Screenshots,
fetched pages, tool output, and plugin inputs may contain credentials, tokens,
personal data, or internal details. This exposure is accepted for the intended
reporting workflow. The accepted risk does not authorize broad permissions:
retain root/service-account ownership, restrict directory traversal, protect
backups, and define retention/deletion procedures appropriate to the reports.

### SC-009: Backup archives in repository history

Status: Mitigated in working tree; historical exposure requires review  
Severity: High if archives contain credentials or sensitive artifacts

Backup archives were previously tracked by Git. They are now ignored and
removed from the current index, but may remain in existing commits or remote
clones. Review archive contents, rotate any exposed credentials, and rewrite
remote history when necessary. Do not add runtime databases, artifacts, or
backups to source control.

### SC-010: PostgreSQL adapter is the sole backend

Status: Mitigated  
Severity: Low

`PostgresStore` is the only run-store implementation. It uses parameterized
queries and canonical artifact-root containment checks equivalent to the
SQLite adapter it replaced. Credentials are supplied only via `DATABASE_URL`
(never hard-coded) — see the database configuration control above.

### SC-011: WebSocket probe scope

Status: Controlled  
Severity: Medium

The `websocat` plugin is limited to a short handshake/connectivity probe and
does not send application messages or credentials. Only explicitly authorized
`ws://` or `wss://` endpoints should be supplied. Treat handshake output as
untrusted artifact data and do not infer authorization or message-level safety
from a successful connection alone.

### SC-012: Artifact-read tools share `run_plugin`'s trust boundary

Status: Accepted, pending MCP authorization  
Severity: High once the server is reachable by more than one trusted client

`list_runs_by_target`, `list_run_artifacts`, and `read_artifact` let any MCP
caller enumerate every recorded run across every target and read any of their
artifact files, subject only to canonical-root containment and a bounded read
size — there is no per-caller authentication or authorization anywhere in
Slickchook yet, so this is not a new exposure relative to `run_plugin`, but it
does make it far easier to browse everything at once rather than only what a
single `run_plugin` call itself returned. `read_artifact` labels returned
content as untrusted report data and logs each read via `tracing`, but that
audit trail has no caller/subject identity to attach, only what was read and
when. Do not expose this server to untrusted or multi-tenant callers until the
OAuth 2.1 work in `ROADMAP.md` lands and these tools are gated by it.

### SC-013: PostgreSQL connection TLS is opt-in, not default

Status: Mitigated, configuration required for non-local deployments  
Severity: High if `DATABASE_URL` ever points at a non-loopback host

`database_tls` (in `slickchook.toml`) controls whether the connection to
PostgreSQL is encrypted; it defaults to `"disable"`, matching the documented
loopback-only deployment model in `RUNNER.md`. Set it to `"require"` before
pointing `DATABASE_URL` at any host that isn't a trusted loopback or local
socket — otherwise the DB password, all run metadata, and any plaintext
sensitive plugin inputs (see SC-005) cross the network unencrypted. There is
no unverified/opportunistic TLS tier: `"require"` fails the connection
outright rather than tolerating a certificate it can't verify against the
host's native trust store, and it's authoritative over whatever `sslmode`
(if any) is present in `DATABASE_URL` itself, rather than merely
supplementing it.

### SC-014: Caddy front-end certificate must be pinned per instance, not shared

Status: Mitigated, provisioning discipline required  
Severity: High if the certificate is trusted broadly rather than pinned

The Ubuntu installer and `.deb` package front `slickchook` with Caddy for
TLS (see `RUNNER.md`), generating a self-signed certificate once per
instance rather than obtaining one from a shared/public CA — there is no
meaningful public hostname for an internal, per-deployment endpoint.
Connecting clients must be configured to trust *only* that specific
certificate (delivered by whatever already-authoritative process
provisions both sides, e.g. at VM-pair creation time), not a shared
organization-wide CA and not the system trust store. Trusting a shared CA
across a fleet means every client trusts every instance, which is broader
than any single client-to-server pairing needs; trust-on-first-connection
is a weaker fallback (whatever intercepts the first connection is trusted
permanently) that should only be used if there is no shared provisioning
process able to deliver the real certificate directly. The certificate is
deliberately never regenerated on reinstall/upgrade, since doing so would
silently invalidate every client's existing pin.

If the installer's fallback path (adding Caddy's official third-party apt
repository) is used because `caddy` isn't in the target's default repos,
that is a new supply-chain trust addition equivalent in kind to the
container-image provenance concerns in SC-003/SC-004 — review it the same
way before relying on it in production.

### SC-015: Post-run artifact permission fixup runs as an unprivileged, same-UID container

Status: Mitigated by construction  
Severity: Low

`slickchook-runner` runs one additional short-lived container after every
plugin run (success, failure, or timeout) to normalize permissions on
whatever the plugin container wrote into `/artifacts`, since some tools
restrict permissions on their own output as they write it, leaving those
files unreadable by the Slickchook service afterward. This fixup container
is not a privilege escalation: it uses the exact same restrictive policy as
plugin containers themselves (`--user 65534:65534`, `--network none`,
`--read-only`, `--cap-drop ALL`, `--security-opt no-new-privileges`), the
same UID the plugin container ran as — not root — and its only mounted
volume is the same run's own `/artifacts` directory. Only a file's owner
(or root) may relax that file's own permissions, which is why this must
run as that same UID rather than being done by the runner or main service
process directly. A failure here is logged and does not fail the run.

### SC-016: `rmcp` DNS rebinding vulnerability (CVE-2026-42559) — pinned version predates the fix, and the fix requires a transport migration

Status: Open, mitigated in the current deployment model  
Severity: High per upstream advisory; lower in practice for this deployment, see below

Slickchook is pinned to `rmcp 0.5.0`. Versions prior to `1.4.0` do not
validate the incoming HTTP `Host` header on their network-facing server
transport, allowing a malicious public website to reach a
loopback-bound MCP server via a DNS rebinding attack and invoke any
tool it exposes, using the victim's browser as a confused deputy
([GHSA-89vp-x53w-74fx](https://github.com/modelcontextprotocol/rust-sdk/security/advisories/GHSA-89vp-x53w-74fx),
CVSS 8.8). The advisory names the newer "Streamable HTTP" transport
module specifically, but the same underlying gap exists in the older
SSE transport Slickchook actually uses: `sse_server.rs` at our pinned
`rmcp-v0.5.0` contains no Host, Origin, or allowlist logic at all
(confirmed by inspecting that file directly).

Two things currently limit the practical exposure of this gap in
Slickchook's actual deployment, matching the advisory's own recommended
workarounds:

- The server only ever binds to `bind_addr = "127.0.0.1:8080"` (loopback),
  never `0.0.0.0`.
- `install-ubuntu.sh` fronts it with Caddy, configured with a TLS site
  block scoped to one specific `CADDY_HOSTNAME:CADDY_LISTEN_PORT` and no
  catch-all block; Caddy's default virtual-host matching means a
  request whose `Host` header doesn't match that hostname — exactly
  what a DNS-rebound request would carry — doesn't match the site block
  and is rejected before reaching Slickchook.

The residual exposure is a browser running *on the same host* as a live
Slickchook instance, which could still reach `127.0.0.1:8080` directly,
bypassing Caddy entirely. This seems unlikely given Slickchook is
deployed as a headless backend service that nothing is expected to
browse to locally, but it is not impossible (for example, an
administrator doing local, interactive debugging on the box itself).

This is not a same-day fix: the patched version (`rmcp 1.4.0`) ships six
minor versions after `SseServer` — the exact transport type Slickchook's
`main.rs` depends on — was deleted from the crate entirely
([PR #562](https://github.com/modelcontextprotocol/rust-sdk/pull/562),
landed at `rmcp 0.11.0`), in favor of a different type,
`StreamableHttpService`, aligned with the current MCP spec (SSE was
removed from the spec itself). Reaching any version with the fix
requires migrating `main.rs`'s server setup to that new transport, not
just bumping the `Cargo.toml` version — including re-verifying the
whole MCP handshake flow (Streamable HTTP's session/endpoint model
differs from SSE's separate `/sse` + `/message?sessionId=...` pattern),
coordinating with whatever connects to Slickchook as an MCP client
today (also presumably SSE-based), and auditing the other
`[**breaking**]`-tagged changes across the intervening releases.

Recommended hardening:

- Track this as a planned migration, not a quick patch: upgrade `rmcp`
  to `>= 1.4.0` and migrate the server transport to
  `StreamableHttpService`, with its own dedicated testing pass.
- Until then, do not change `bind_addr` away from a loopback address,
  and do not remove or bypass the Caddy front-end's hostname-scoped site
  block — both are load-bearing for the mitigation described above.
- Re-review this entry once the migration lands and mark it resolved.

### SC-017: Screenshot/OCR content is an accepted prompt-injection vector

Status: Mitigated, isolation must be preserved  
Severity: High if screenshot content ever reaches tool-calling context directly

Web pages rendered by `gowitness` may contain instructions aimed at an AI
viewer — fake system messages, hidden or low-contrast text, encoded payloads
— intended to manipulate whatever reads the screenshot next. `screensanitize`
is the required mitigation: it strips metadata, re-encodes the image, runs
OCR, detects hidden/low-contrast text, and flags encoded forms (Base64,
Morse, Braille, QR codes, suspicious Unicode control characters). Its output
remains untrusted structured data with no tool-calling authority of its
own — vision/OCR processing must stay isolated from tool execution, return
structured observations only, and require independent confirmation before
any action is taken on what it finds. Never route raw `gowitness`
screenshots directly into a context that can invoke tools; always go
through `screensanitize` first.

Until 2026-08-06, this mitigation never actually ran against a real
screenshot: `gowitness` wrote its output to `/artifacts/screenshots`, while
`screensanitize`'s `input_run_id` chaining only ever exposes the prior
run's own `/input` staging directory — the convention `webfetch` uses to
hand off fetched content, which `gowitness` was never wired into. A chained
`screensanitize` run silently produced `"results": []` — zero images
found, not an error — with nothing to indicate anything was wrong; verified
live against a real `gowitness` capture before the fix. Fixed by having
`gowitness` write its screenshot into `/input` instead (mirroring
`webfetch`'s pattern), and by fixing `screensanitize`'s own re-encoded-image
write, which was separately failing (it tried to write back into the
correctly-read-only `/input` mount) and silently skipping the OCR/
flag-detection code that ran after it in the same `try` block. OCR is
currently a single pass at the image's native resolution, not multi-scale;
treat detection of small or low-contrast injected text as best-effort until
that's revisited.

Separately, until this same fix, an MCP client had no real way to retrieve
a screenshot's actual bytes at all: `list_run_artifacts` never recursed
into subdirectories (so a screenshot nested under `screenshots/` was
invisible to it), and `read_artifact` returned image bytes as a base64
string embedded in a JSON text block rather than a proper MCP image
content type. Both are now fixed: artifact listing recurses, and
`read_artifact` returns a real `image` content block for image mime types.

### SC-018: Plugin-configured secrets are fully exposed to their own container

Status: Accepted risk  
Severity: High impact if a plugin package or image is malicious or compromised

Manifest-declared `[[secrets]]` (see "Existing controls") keep a persistent
credential like a Shodan API key out of per-call inputs, run history, and
argv — but once a value is forwarded into a container as an environment
variable, it is exposed to that container exactly as fully as the plugin's
own code: any process running inside can read it, exfiltrate it over the
network (for any plugin with `docker.network != "none"`), or write it into
an artifact that later gets treated as report data rather than credential
material. This is not a gap in the mechanism itself; it is the same "plugin
containers are untrusted even after their manifests are approved" boundary
from the top of this document, now carrying a longer-lived, more valuable
credential than a one-off test password.

Recommended hardening:

- Review a manifest that declares `[[secrets]]` with at least the same
  scrutiny as one that installs a `Dockerfile` (SC-003): confirm the image
  only sends the credential to the service it's actually for, and does not
  log request/response bodies that might include it.
- Where the underlying tool supports it, prefer `docker.network = "none"`
  plus a purpose-built fetch step, or the narrowest possible egress, over
  general internet access for a plugin holding a real credential.
- Treat any stdout/stderr a secret-holding plugin produces as at least as
  sensitive as its own report artifacts (SC-008), since a buggy or verbose
  tool can echo the credential it was just handed.
- Rotate a key immediately if a plugin holding it is later found to be
  malicious, compromised, or simply buggy in a way that could have exposed
  it.

## Reporting and maintenance

For every new concern, assign an `SC-NNN` identifier, record its status and
severity, state the affected trust boundary, and distinguish current controls
from proposed work. When fixed, retain the entry, mark it resolved with the
date, and link to the relevant changelog entry or commit. Keep `### SC-NNN`
entries under "Open concerns" in ascending numeric order — new entries always
get the next unused number, appended at the end of that ordering.
