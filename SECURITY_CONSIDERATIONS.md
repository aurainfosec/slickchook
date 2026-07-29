# Security considerations

This is a living record of Slickchook security assumptions, controls, and open
concerns. Update it whenever a security-relevant behavior is introduced,
discovered, fixed, or deliberately accepted.

Last reviewed: 2026-07-15

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
  capabilities are validated. Only `NET_RAW` can currently be added.
- Containers run with all capabilities dropped by default, a non-root user, a
  read-only root filesystem, `no-new-privileges`, and configured CPU, memory,
  PID, and time limits.
- The runner canonicalizes the manifest and artifact paths and checks that they
  are beneath their configured roots.
- The plugin installer rejects package contents that are not regular files or
  directories, normalizes installed ownership and permissions, serializes
  changes with a lock, stops the service while switching packages, installs by
  directory rename, and rolls back when service validation fails.

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
SQLite table. Plugins such as Medusa accept a password, so even a test run
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

## Reporting and maintenance

For every new concern, assign an `SC-NNN` identifier, record its status and
severity, state the affected trust boundary, and distinguish current controls
from proposed work. When fixed, retain the entry, mark it resolved with the
date, and link to the relevant changelog entry or commit.
## Static-analysis staging mounts

Analyzer containers receive `/input` as a read-only mount by default. Only a
purpose-built fetcher may set `docker.input_read_only = false`, and that mount is
still constrained to the current run's artifact directory. This separates networked
collection from offline analysis and prevents analyzers from modifying collected
content or accessing arbitrary host paths.

Screenshot prompt injection is an accepted-content risk. Web pages may render
instructions aimed at an AI viewer, including fake system messages or hidden
text. Vision/OCR processing must be isolated from tool execution, return
structured observations, and require independent confirmation before action.

PostgreSQL is the only supported run-store backend; Slickchook requires
`DATABASE_URL` to start and has no embedded fallback.

Database configuration must not be hard-coded in manifests or source. The
installer provisions a dedicated non-superuser role and database, and writes
non-secret connection settings to `/etc/slickchook/postgres.env`, owned by
`root:slickchook` with mode `0640`. Passwords, if later required for remote
connections, must be supplied out-of-band through a secret manager or protected
deployment mechanism.

### SC-009: Backup archives in repository history

Status: Mitigated in working tree; historical exposure requires review  
Severity: High if archives contain credentials or sensitive artifacts

Backup archives were previously tracked by Git. They are now ignored and
removed from the current index, but may remain in existing commits or remote
clones. Review archive contents, rotate any exposed credentials, and rewrite
remote history when necessary. Do not add runtime databases, artifacts, or
backups to source control.

The `screensanitize` plugin strips metadata through re-encoding,
re-encodes images, performs multi-scale OCR, detects hidden/low-contrast text,
and flags encoded forms such as Base64, Morse, Braille, QR codes, and suspicious
Unicode controls. Sanitizer output remains untrusted structured data and has no
tool-calling authority.

Naabu may use the narrowly scoped `NET_RAW` capability for SYN scans when the
updated manifest is installed. This increases packet-level network access; use
CONNECT mode when raw-packet scanning is unnecessary and retain existing rate,
process, memory, and timeout limits.

### SC-008: Sensitive report artifacts

Status: Accepted risk  
Severity: High impact if artifact access is misconfigured

The artifacts directory is intentionally treated as report data. Screenshots,
fetched pages, tool output, and plugin inputs may contain credentials, tokens,
personal data, or internal details. This exposure is accepted for the intended
reporting workflow. The accepted risk does not authorize broad permissions:
retain root/service-account ownership, restrict directory traversal, protect
backups, and define retention/deletion procedures appropriate to the reports.

Kiterunner route discovery is active and can generate substantial request
volume. The default plugin uses the small corpus and low concurrency; any large
corpus must be separately reviewed and rate-limited before deployment.

### SC-007: Offline analyzer database supply chain

Status: Mitigated with maintenance requirements  
Severity: Medium

OSV-Scanner bundles an OSV database into its image so runtime analysis does not
require network access. The scanner and database snapshot must be updated and
reviewed together. Rebuild with `--no-cache`, record the source revision and
image digest, and keep analyzer networking disabled.
### SC-010: PostgreSQL adapter is the sole backend

`PostgresStore` is the only run-store implementation. It uses parameterized queries and
canonical artifact-root containment checks equivalent to the SQLite adapter it replaced.
Credentials are supplied only via `DATABASE_URL` (never hard-coded); see the database
configuration guidance above.

### SC-011: WebSocket probe scope

Status: Controlled
Severity: Medium

The websocat plugin is limited to a short handshake/connectivity probe and does
not send application messages or credentials. Only explicitly authorized
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
