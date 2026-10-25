# Security considerations

This is a living record of Slickchook security assumptions, controls, and open
concerns. Update it whenever a security-relevant behavior is introduced,
discovered, fixed, or deliberately accepted.

Last reviewed: 2026-10-25

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
  are beneath their configured roots. Reviewed directly against CVE-2026-66384
  (a JFrog Artifactory path traversal where a Docker cache subsystem derived
  destination file paths from remote-repository-supplied metadata without
  canonicalizing/validating them against the cache root) on 2026-08-25:
  confirmed Slickchook's own equivalent surfaces don't share this flaw. A
  client-supplied `run_id` only ever drives a database lookup, never a direct
  path join; the resulting `artifact_dir` (and `read_artifact`'s genuinely
  client-controlled `path` parameter) are canonicalized and checked with
  `canonical_contained()`/`starts_with()` against the configured root before
  any filesystem access, which also resolves and rejects symlink escapes;
  container mount destinations (`/artifacts`, `/input`) are always fixed
  literals, never built from plugin- or target-supplied data. Both a `../`
  traversal attempt and a planted symlink escape are asserted-rejected by
  existing, currently-passing tests
  (`plugin_manager::tests::target_scoped_artifact_retrieval_end_to_end`,
  `storage::tests::postgres_store_round_trip_when_configured`), so this isn't
  a one-time check.
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
  rather than a per-call `option`/`argument` a client can supply, keeping
  it out of the plaintext exposure described in SC-005. The value never
  appears in any argv or log line — see RUNNER.md's "Configuring plugin
  API keys and secrets" for the deployment mechanism, PLUGINS.md's
  "Declaring required secrets" for the manifest schema, and SC-018 below
  for the residual trust boundary once a value reaches a container.
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

Status: Resolved, 2026-10-25 (see SC-022 for what actually closed this and
why the original mitigation described below didn't)
Severity: Medium

Plugin containers share the per-run artifact directory. A container could
create or replace the reserved `stdout.txt` or `stderr.txt` paths before the
server records process output.

This entry originally claimed artifact recording "rejects symlinks and
non-regular files" via a `path.exists()` pre-check before an explicit
truncating open. An independent security review (2026-10-20, see SC-022)
found that claim was stronger than the code actually supported: `exists()`
returns `false` for a *dangling* symlink, so that specific pre-check never
ran for one, and the subsequent `create(true).truncate(true)` open followed
it and created the destination as the service user - demonstrated directly
against the real `write_artifact` function body, not just reasoned about in
the abstract. SC-022 replaced the check-then-open sequence with an atomic
`create_new` open, which has no such gap. Kept as a reminder that a
"mitigated" status needs the same verification rigor the mitigation itself
got, not just a plausible-sounding description.

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

**Addendum (2026-10-15, issue #133):** the certificate's
`basicConstraints` extension was `CA:FALSE` until this date, which is
wrong for this specific use — a certificate pinned as a client's trust
anchor is acting as that client's root of trust, and a strict
chain-building verifier (OpenSSL's, which Node's TLS stack uses) refuses
to accept a `CA:FALSE` certificate in that role at all, even as a
self-signed leaf. `curl --cacert` is permissive enough not to catch this,
which is why it went unnoticed: the endpoint looked healthy under manual
`curl` checks while every Node-based MCP client (Claude Code included)
failed to connect with no further diagnostic than "Failed to connect."
Fixed by issuing new certificates with `CA:TRUE` and `keyUsage` extended
to include `keyCertSign`, in both `scripts/build-deb.sh` and
`scripts/install-ubuntu.sh`. This only affects *newly generated*
certificates — per the "deliberately never regenerated on
reinstall/upgrade" behavior above, an already-deployed instance keeps
its existing `CA:FALSE` certificate (and the connection failure it
causes) until `/etc/caddy/tls/server.{crt,key}` is deleted and
regenerated.

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

Verified live and working end-to-end as of 2026-08-07 (real `gowitness`
capture → `screensanitize` → populated OCR text and flags, plus a real
`image` content block from `read_artifact`). Before that date, five
compounding bugs across the capture → sanitize → retrieve chain meant
this mitigation had never actually run against a real screenshot in
production — see `CHANGELOG_2026-08-06.md` and `CHANGELOG_2026-08-07.md`
for the incident narrative if this resurfaces. One forward-looking caveat
survives the fix: OCR is currently a single pass at the image's native
resolution, not multi-scale; treat detection of small or low-contrast
injected text as best-effort until that's revisited.

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

### SC-019: `rmcp` Dependabot alerts reviewed and confirmed not applicable — vulnerable code isn't compiled into this binary

Status: Reviewed, not applicable  
Severity: High/High/Medium/Medium per upstream advisories; none apply to Slickchook's actual build

Four new Dependabot alerts landed against `rmcp` between September and
October 2026, on top of the already-tracked SC-016:

- [GHSA-9pj6-vhgr-3mwh](https://github.com/modelcontextprotocol/rust-sdk/security/advisories/GHSA-9pj6-vhgr-3mwh)
  (CVE-2026-63128, High) — an unauthenticated remote attacker can leak
  one permanent session-table entry per malformed POST against
  `StreamableHttpService::handle_post`, exhausting server memory.
- [GHSA-33f5-2c5q-wgwj](https://github.com/modelcontextprotocol/rust-sdk/security/advisories/GHSA-33f5-2c5q-wgwj)
  (CVE-2026-63127, High) — `rmcp`'s OAuth client doesn't validate the
  `resource` field in Protected Resource Metadata (RFC 9728), letting a
  malicious MCP server redirect an OAuth flow and steal the resulting
  access token from a victim client.
- [GHSA-9g45-5xwm-f3wc](https://github.com/modelcontextprotocol/rust-sdk/security/advisories/GHSA-9g45-5xwm-f3wc)
  (CVE-2026-64684, Medium) — `StreamableHttpClientTransport`'s default
  HTTP client follows redirects without stripping caller-supplied custom
  headers (e.g. an API key), leaking them to a cross-origin redirect
  target.
- [GHSA-c9xm-49cp-xcr9](https://github.com/modelcontextprotocol/rust-sdk/security/advisories/GHSA-c9xm-49cp-xcr9)
  (no CVE assigned yet, Medium) — `rmcp`'s OAuth client fetches a
  `resource_metadata` URL taken directly from a server-controlled
  `WWW-Authenticate` header with no same-origin or private-network
  check, letting a malicious MCP server make the client issue an SSRF
  request (e.g. to a cloud metadata endpoint or an internal service).

Unlike SC-016 (where the same *underlying* gap turned out to also exist
in the SSE transport code Slickchook actually uses, just not the exact
module the advisory named), all four of these were checked directly
against `rmcp 0.5.0`'s own `Cargo.toml` feature list
(`~/.cargo/registry/src/.../rmcp-0.5.0/Cargo.toml`) and don't apply, for
two independent reasons:

- **The vulnerable modules require Cargo features Slickchook doesn't
  enable at all.** Slickchook's own `Cargo.toml` only turns on `server`
  and `transport-sse-server`. The session leak lives entirely in
  `transport-streamable-http-server` (its own separate feature); both
  OAuth flaws (the resource-field validation gap and the
  `resource_metadata` SSRF) live in `auth` — confirmed directly in
  `rmcp 0.5.0`'s own `src/transport.rs`, where `pub mod auth;` sits
  behind a literal `#[cfg(feature = "auth")]`; the header-leak flaw
  lives in `transport-streamable-http-client` and `client`. None of
  those features are enabled, so none of that code is even compiled
  into Slickchook's binary — not just unreached at runtime, genuinely
  absent from the built artifact.
- **The session-leak's specific bug pattern doesn't independently exist
  in the code Slickchook does compile**, checked directly (the same
  rigor SC-016 applied, rather than trusting feature separation alone):
  `sse_server.rs`'s own `sse_handler` inserts a session and always
  spawns a cleanup task keyed to the connection actually closing: the
  one early-return path that skips explicit cleanup only fires when the
  server's own internal transport channel has already closed (i.e.
  during shutdown), not on attacker-controlled per-request input the
  way the advisory's `handle_post` bug does. There is no "validate the
  request body, leak the session state if it's invalid" pattern here
  at all — `sse_handler` never validates incoming message content in
  the first place, that happens in a separate handler that looks up an
  already-established session rather than creating one.
- Both OAuth flaws and the header-leak flaw are also conceptually
  inapplicable regardless of feature-gating: all three are client-side
  transport behavior (an `rmcp`-based *client* authenticating to, or
  sending custom-header requests to, another MCP server). Slickchook is
  exclusively an MCP *server* — it never acts as an OAuth client or
  sends outbound custom-header requests of this kind to anything.

Recommended hardening:

- No action needed against the current codebase for these four; they
  do not represent a real gap in the deployed binary.
- If `rmcp` is ever upgraded to enable `client`, `auth`, or
  `transport-streamable-http-client`/`-server` (e.g. as part of the
  SC-016 migration to `StreamableHttpService`), re-review this entry
  and SC-016 together, since crossing that migration reintroduces
  exactly the modules these four alerts describe — confirm the
  version landed on actually includes each fix (`>= 2.0.0` for the
  session leak and both OAuth flaws, `>= 2.1.0` for the header leak).
- Re-verify this reasoning (feature list + `sse_server.rs`'s own
  session logic) after any `rmcp` version bump, rather than assuming
  it still holds — a point release could restructure either.

### SC-020: `rustls` TLS 1.3 handshake encryption-level boundary bug (GHSA-2mjx-qc3c-rqvc) — fixed by upgrading, unlike SC-019's four alerts

Status: Resolved, 2026-10-15  
Severity: Medium per upstream advisory; on-path-attacker precondition limits practical impact here

Dependabot flagged
[GHSA-2mjx-qc3c-rqvc](https://github.com/rustls/rustls/security/advisories/GHSA-2mjx-qc3c-rqvc)
(no CVE assigned yet, CVSS 5.3) against `rustls`: versions `0.23.13`
through `0.23.44` accept a TLS 1.3 handshake message (e.g.
`EncryptedExtensions`) packed into the same record as the preceding
key-changing message, instead of rejecting the connection per RFC 8446
§5.1. The handshake transcript remains authenticated, so this can't be
used to alter or complete a forged handshake, but it means a peer could
send a message that should have been encrypted in plaintext without
`rustls` rejecting the connection.

Unlike the four alerts reviewed in SC-019, this one is directly
applicable: `Cargo.lock` had resolved `rustls` to `0.23.42`, inside the
vulnerable range, and `rustls` is a real, compiled dependency here —
`tokio-postgres-rustls` uses it as a TLS *client* for the PostgreSQL
connection when `database_tls = "require"` (see SC-013), which is
exactly the handshake-parsing role this advisory describes. There is no
feature-gating argument available this time.

Fixed by running `cargo update -p rustls`, which resolved to `0.23.46`
(`rustls-webpki` moved to `0.103.15` alongside it) — both already
satisfy the existing `Cargo.toml` constraint (`rustls = "0.23.42"`,
i.e. `^0.23.42`), so no manifest edit was needed. `cargo build --locked
--release --bins` and `cargo test --lib` (12/12) pass against the
updated lockfile.

Exploiting this still requires an attacker positioned on-path between
Slickchook and whatever TLS peer it's talking to as a client (here,
PostgreSQL) — see the "Intended network environment" note in
`README.md`'s security-model section for why that precondition is
expected not to hold in Slickchook's intended deployment, same as the
loopback/Caddy-scoping assumptions SC-016 and SC-014 already rely on.

Recommended hardening:

- Keep `rustls` current via routine `cargo update` passes, not just in
  response to a Dependabot alert; this crate is maintained actively
  enough that version drift accumulates real fixes.
- Re-run this same resolved-version check (`grep -A2 'name = "rustls"'
  Cargo.lock`) whenever reviewing a future `rustls` or
  `tokio-postgres-rustls` advisory, since `Cargo.toml`'s own
  version constraint only sets a floor — what's actually compiled in is
  whatever `Cargo.lock` resolved to.

### SC-021: Target-influenced output/storage had no host-side bound (R7 of the 2026-10-20 independent review)

Status: Resolved (first increment), 2026-10-25
Severity: High availability risk per the review; host memory/disk exhaustion, not a containment bypass

An independent security review of this codebase (not generated by or
committed to this repository) identified that a plugin's `docker.memory`
limit bounds only the *container's own* resident memory, not what the
slickchook service or runner hold in host memory/disk while a run executes.
Two concrete gaps:

- `slickchook-runner`'s `docker run` invocation used `Command::output()`,
  which buffers the full stdout/stderr of the plugin container with no
  limit. A tool that echoes target-controlled content back through its own
  output (a crawler's JSONL, a verbose scan result) could grow that buffer
  without bound — streaming through a pipe doesn't count against the
  container's own `--memory` limit the way holding the same data in its
  heap would.
- Named concrete routes from a target's response straight to an unbounded
  host-backed file: `httprequest` wrote a response body to
  `/artifacts/response_body` with no size cap, and `webfetch` wrote a
  fetched page to `/input/index.html` the same way. Neither curl nor wget
  was told to stop at any size.

There was also no global concurrency limit across `run_plugin`/`start_plugin`
(parallel runs multiply this exposure), no bound on recursive artifact
listing, and `start_plugin`'s async job map retained every finished job's
result indefinitely if a caller never called `plugin_status`.

Fixed, all with generous defaults an operator can override rather than a
hardcoded wall (see `RUNNER.md`'s "Host-side resource bounds" for the full
table):

- `slickchook-runner` now reads the container's stdout/stderr through a
  bounded, fully-draining reader (`SLICKCHOOK_MAX_PLUGIN_OUTPUT_BYTES`,
  default 256 MiB per stream) instead of `Command::output()`'s unbounded
  buffering; truncated output gets a visible marker rather than silently
  looking complete.
- `httprequest` gained curl's `--max-filesize 104857600` (100 MiB) and
  `webfetch` gained wget's `--quota=100m` — tool-level caps on the two
  concrete vectors named above.
- `PluginManager` gained a global `tokio::sync::Semaphore`
  (`SLICKCHOOK_MAX_CONCURRENT_RUNS`, default 8) acquired for the full
  duration of every run, shared by both execution APIs.
- `list_artifacts` now refuses to walk past 50,000 entries in one run's
  artifact directory (a hard safety backstop, not a tuning knob — no
  legitimate plugin output comes close).
- Each run's total artifact bytes are measured after it completes and
  flagged (`artifact_bytes`/`artifact_oversized` in the result,
  `SLICKCHOOK_MAX_ARTIFACT_BYTES`, default 1 GiB) if oversized — flag-only,
  never deleted, so a legitimately large run's output is never silently
  destroyed. A real preventive quota needs filesystem-level support (e.g.
  an XFS project quota on the artifacts volume), which is a deployment
  decision, not something this change can enforce from inside the
  application.
- `start_plugin`'s job map now evicts finished jobs after
  `SLICKCHOOK_JOB_RETENTION_SECONDS` (default 3600s) if never polled.

While restructuring `start_plugin`'s job bookkeeping for the eviction sweep,
also fixed a related but distinct correctness bug the same review flagged
(its R9): the task was spawned before its map entry existed, so a very fast
completion (e.g. an unknown-plugin failure) could finish and look itself up
before being inserted, silently discard its own result, and leave a
permanently-"running" phantom entry with an already-completed abort handle.
Each job's state now lives behind its own `Arc<Mutex<JobState>>`, created
and handed to its task before that task is spawned, so the task always has
somewhere to record its result regardless of timing.

This is the first of several increments addressing the 2026-10-20 review's
findings, worked through in its own stated priority order (R7, this entry,
first). A true host-level disk quota (the "artifact byte/inode quotas" half
of the review's own fix suggestion) remains open — it requires an
operator-established filesystem quota the application can't substitute for,
and the review itself asks that actual host disk/memory budgets be
established before closing this out completely.

Validation: `cargo build --locked --release --bins` and `cargo test --lib`
(12/12) pass; `cargo test --bin slickchook-runner` (5/5, new) exercises the
bounded-reader's truncation/non-truncation/exact-boundary behavior directly;
`cargo test --bin slickchook` (2/2) unaffected by the job-bookkeeping
change. Not independently load-tested against a real excessive-output
target in this sandbox (no Docker socket access) — needs confirming on a
real deployment that a deliberately oversized response actually gets capped
end to end.

### SC-022: `write_artifact` followed a dangling symlink (R3 of the 2026-10-20 independent review)

Status: Resolved, 2026-10-25
Severity: High-impact containment weakness, conditional on a writer able to
plant a filesystem entry in the artifact directory

The same independent review (see SC-021/R7) found that `write_artifact` -
used to persist a run's `stdout.txt`/`stderr.txt` - only called
`fs::symlink_metadata` inside `if path.exists()`. `exists()` follows
symlinks and returns `false` for a *dangling* one, so a plugin container
(the only other writer to the run's `/artifacts` bind mount) could plant
`stdout.txt` as a symlink to a nonexistent path outside the run directory
before exiting; the pre-check never ran, and the subsequent
`OpenOptions::create(true).truncate(true).open(path)` followed the link and
created its destination as the slickchook service user. Even when the
target path already existed as a regular file, the check and the open were
two separate operations, leaving a window for a concurrent writer to swap
it for a symlink in between. This directly contradicted SC-006's
"mitigated" claim (now corrected to point here).

Validated directly against the real function body before and after the
fix - a planted dangling symlink genuinely caused a file to be created
outside the run directory under the old code, and the three new unit tests
below both reproduce that and confirm it no longer happens.

Fixed by replacing the check-then-open sequence with a single atomic
`OpenOptions::new().create_new(true)` open (POSIX `O_CREAT|O_EXCL`):
`stdout.txt`/`stderr.txt` always land in a brand-new, just-created artifact
directory, so there is never a legitimate reason for anything to already
exist at that path. `create_new` fails on *anything* already there - a
symlink (dangling or not), a regular file, a FIFO - in one indivisible
syscall, which closes the TOCTOU window entirely rather than narrowing it;
there is no longer a separate check step for a concurrent writer to win a
race against.

Validation: `cargo build --locked --release --bins` and `cargo test --lib`
(15/15 - three new: `write_artifact_creates_a_brand_new_file`,
`write_artifact_refuses_a_dangling_symlink`,
`write_artifact_refuses_an_already_existing_regular_file`). The
dangling-symlink test is a direct regression test for this exact bug - it
fails under the old `path.exists()`-based code (confirmed by reasoning
through the old logic against it, since the old code is no longer present
to re-run) and passes under the fix.

## Reporting and maintenance

For every new concern, assign an `SC-NNN` identifier, record its status and
severity, state the affected trust boundary, and distinguish current controls
from proposed work. When fixed, retain the entry, mark it resolved with the
date, and link to the relevant changelog entry or commit. Keep `### SC-NNN`
entries under "Open concerns" in ascending numeric order — new entries always
get the next unused number, appended at the end of that ordering.
