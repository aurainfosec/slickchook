# Deploying the runner on Ubuntu Server

`slickchook` is the long-running, unprivileged MCP server. It starts
`slickchook-runner` only when a plugin is run. The runner is the only Slickchook
process that receives access to the Docker socket.

The setup below uses the set-group-ID (`setgid`) bit on the runner executable.
When the `slickchook` user executes that file, that process and its descendants
receive the `docker` group as their effective group. Do **not** add the
`slickchook` user to the `docker` group: Docker socket access is effectively
root-equivalent.

These instructions assume:

- Ubuntu Server with Docker Engine already installed and running;
- PostgreSQL is installed and running; Slickchook requires it, connecting via
  `DATABASE_URL`. The automated installer provisions a local role, database,
  and connection file for you (see below);
- the repository is the current directory; and
- Rust is installed on the build machine.

## Recommended: install from the `.deb` package

For a standard Ubuntu Server, build a native package and install it with
`apt` — this is the recommended way to deploy Slickchook, since `apt`
resolves and installs Slickchook's own package dependencies
(`docker.io`/`docker-ce`, `postgresql`, `postgresql-client`, `adduser`,
`acl`) for you, on top of everything the installer script below already
does:

```bash
cargo build --locked --release --bins
./scripts/build-deb.sh
sudo apt install ./dist/slickchook_*_$(dpkg --print-architecture).deb
```

(Use `apt install ./file.deb`, not `dpkg -i`, precisely so dependency
resolution happens — `dpkg -i` alone will fail or leave dependencies
unresolved.)

The package's `postinst` performs the same setup steps as
`install-ubuntu.sh` below (service account, PostgreSQL role/database and
schema, the systemd unit, and the Caddy TLS front-end) and is equally
idempotent: running `apt install` again over an existing installation (a
newer `.deb`) updates it in place — same env-var override mechanism for
the Caddy hostname/port/cert lifetime as described in that section below,
since `sudo` resets most environment variables by default:

`apt` decides "newer" purely by comparing the package's `Version:`
field — which `build-deb.sh` reads straight from `Cargo.toml`'s own
`version` — never by comparing file contents. **Any change to anything
the package actually ships must bump that version first**, not just a
change to the compiled binary: `build-deb.sh` also copies the whole
`plugins/` tree into the package (`/usr/share/slickchook/plugins`,
source for `slickchook-plugin install`) and bakes the installer's
`postinst`/`prerm` scripts into it, so a plugin-manifest/Dockerfile fix
or an installer-script fix needs the same version bump as a
dependency-only `Cargo.lock` update does. Rebuilding and reinstalling
without bumping the version produces a `.deb` `apt` considers identical
to what's already installed, and `apt install` silently reports
`<pkg> is already the newest version` and does nothing — whatever was
actually fixed keeps not taking effect, with no error to signal it. If
that already happened, recover with
`sudo apt install --reinstall ./dist/slickchook_*.deb`
rather than re-running the normal install command.

```bash
sudo --preserve-env=CADDY_HOSTNAME,CADDY_LISTEN_PORT,CADDY_CERT_DAYS \
  CADDY_HOSTNAME=my-vm CADDY_LISTEN_PORT=8443 CADDY_CERT_DAYS=730 \
  apt install ./dist/slickchook_*_$(dpkg --print-architecture).deb
```

Plugin packages have their own installation lifecycle and are not copied or
updated by either installation method. See `PLUGINS.md`.

## Development use: `scripts/install-ubuntu.sh`

**For a new installation, use the `.deb` package above — it's the
recommended path, not this script.** This script exists for
development iteration on a host you already control: rebuilding and
reinstalling core binaries quickly while working on Slickchook itself
(or on core changes needed alongside plugin work — a new manifest
field, a runner behavior change) without repackaging a `.deb` for
every change. It is not a recommended alternative production
deployment method — reach for it when you're iterating on this
repository's own code on a dev/test host, not when standing up a new
instance to actually run. (Plugin-only development doesn't need this
script at all: `sudo slickchook-plugin install plugins/<name>`
already installs/updates one plugin directly from a checkout with no
core rebuild involved — see `PLUGINS.md`.)

Run it directly from the repository root:

```bash
cargo build --locked --release --bins
sudo ./scripts/install-ubuntu.sh
```

The script performs the steps below, installs `deploy/slickchook.service`, and
stops with an error if the runner ownership, setgid mode, ACL, Docker socket, or
mount options are unsafe. Keep the manual procedure below for auditing and
troubleshooting.

The same command works for both a first run and a later re-run against an
already-installed host (the update case you'll actually hit in practice
while iterating). On a re-run it:

- stops an existing `slickchook.service` before replacing binaries;
- repairs legacy service-account settings and removes unsafe direct membership
  in the `docker` group;
- preserves the artifact database and all existing run artifacts;
- leaves independently installed plugin packages in place;
- backs up changed core binaries, helper commands, and the systemd unit below
  `/var/backups/slickchook` before replacing them; and
- refuses to proceed if an unmanaged process is listening on port 8080.

It installs `acl`, `postgresql`, `postgresql-client`, and Caddy itself, the
same as the `.deb`'s package dependencies do — but unlike the `.deb`, it
cannot pull in Docker automatically (there's no single correct
`docker.io`/`docker-ce` choice to default to via a plain `apt-get install`
the way the package's `Depends:` line expresses it); Docker Engine must
already be installed and running before you run this script (see step 1
below).

## 1. Check Docker and install deployment utilities

The Docker package should have created a `docker` group and a group-writable
socket. Install `acl` for the runner and artifact-directory ACLs:

```bash
sudo apt-get update
sudo apt-get install --yes acl
sudo systemctl enable --now docker
getent group docker
sudo stat -c '%A %a %U:%G %n' /var/run/docker.sock
```

The socket will normally be mode `0660` and owned by `root:docker`. Resolve any
Docker installation problem before continuing.

## 2. Create the service account explicitly

Create both the primary group and user so this procedure does not depend on the
host's `useradd` defaults:

```bash
getent group slickchook >/dev/null || sudo groupadd --system slickchook
id slickchook >/dev/null 2>&1 || sudo useradd --system \
  --gid slickchook \
  --home-dir /var/lib/slickchook \
  --shell /usr/sbin/nologin \
  slickchook
sudo install -d -o slickchook -g slickchook -m 0750 /var/lib/slickchook
```

Confirm that `docker` is not among the user's supplementary groups:

```bash
id slickchook
test "$(id -gn slickchook)" = slickchook
! id -nG slickchook | tr ' ' '\n' | grep -qx docker
```

## 3. Build and install the binaries

Build on the Ubuntu server, or copy binaries built for the same architecture and
Ubuntu/glibc environment into `target/release` first:

```bash
cargo build --locked --release --bins
sudo install -d -o root -g root -m 0755 /opt/slickchook
sudo install -o root -g root -m 0755 \
  target/release/slickchook /opt/slickchook/slickchook
sudo install -o root -g docker -m 0750 \
  target/release/slickchook-runner /opt/slickchook/slickchook-runner
sudo setfacl -m u:slickchook:rx,m::rx /opt/slickchook/slickchook-runner
sudo chmod 0751 /opt/slickchook/slickchook-runner
sudo chmod g+s /opt/slickchook/slickchook-runner
```

The final `chmod g+s` is intentional: it ensures the setgid bit is present after all
ownership and ACL changes. Verify the result:

```bash
sudo stat -c '%A %a %U:%G %n' /opt/slickchook/slickchook-runner
sudo getfacl /opt/slickchook/slickchook-runner
sudo -u slickchook /opt/slickchook/slickchook-runner --help
```

The `stat` output must show mode `2751`, owner `root`, group `docker`, and an `s`
in the group execute position, similar to:

```text
-rwxr-s--x 2751 root:docker /opt/slickchook/slickchook-runner
```

The ACL must contain `user:slickchook:r-x`. The help command should print usage;
it verifies that the service account is allowed to execute the runner, but does
not launch Docker.

## 4. Install plugins and create writable artifact storage

The server currently uses paths relative to its working directory. Install
plugin manifests under `/var/lib/slickchook/plugins` and keep them root-owned so
the service cannot rewrite runner policy:

```bash
sudo install -d -o root -g slickchook -m 0750 /var/lib/slickchook/plugins
# Install each plugin through the policy-validating installer; pass the source
# directory, not just the plugin name. This preserves root ownership and
# validates the manifest before it becomes live policy.
sudo /usr/local/sbin/slickchook-plugin install /path/to/slickchook/plugins/nmap
```

Repeat for each plugin you want to enable. The installed copy is then visible
under `/var/lib/slickchook/plugins`; do not edit that live tree directly.

```bash
sudo install -d -o slickchook -g slickchook -m 0750 \
  /var/lib/slickchook/artifacts
```

Containers run as UID/GID `65534:65534` and need to write tool-generated files
inside each run directory. Apply both an access ACL and a default ACL so new run
directories inherit that permission:

```bash
sudo setfacl -m u:65534:rwx,m::rwx /var/lib/slickchook/artifacts
sudo setfacl -m d:u:65534:rwx,d:m::rwx /var/lib/slickchook/artifacts
sudo getfacl /var/lib/slickchook/artifacts
```

On Ubuntu, UID 65534 is normally `nobody`. This ACL means every plugin container
uses the same host identity for artifact access; do not store unrelated data in
the artifacts tree.

## 5. Install the systemd service

Create `/etc/systemd/system/slickchook.service`:

The installer also creates `/etc/slickchook/slickchook.toml` (root:slickchook,
0640) and the unit loads it through `SLICKCHOOK_CONFIG`. Set the listener port there:

```toml
bind_addr = "127.0.0.1:18080"
```

```ini
[Unit]
Description=Slickchook MCP server
Wants=docker.service
After=network.target docker.service

[Service]
Type=simple
User=slickchook
Group=slickchook
WorkingDirectory=/var/lib/slickchook
Environment=SLICKCHOOK_RUNNER=/opt/slickchook/slickchook-runner
Environment=PATH=/usr/sbin:/usr/bin:/sbin:/bin
ExecStart=/opt/slickchook/slickchook
Restart=on-failure
RestartSec=5s
UMask=0027

# Do not set NoNewPrivileges=true: the runner relies on its setgid bit.
NoNewPrivileges=false

[Install]
WantedBy=multi-user.target
```

Then load and start it:

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now slickchook.service
sudo systemctl status slickchook.service
sudo journalctl -u slickchook.service -n 100 --no-pager
```

The server listens on `127.0.0.1:8080` and exposes its SSE endpoint at `/sse`.

## Front the MCP endpoint with Caddy (TLS)

The installer also installs and configures [Caddy](https://caddyserver.com/) as
a TLS-terminating reverse proxy in front of the plaintext `127.0.0.1:8080`
listener, since `slickchook` itself has no built-in transport encryption.
Caddy listens on `CADDY_HOSTNAME:CADDY_LISTEN_PORT` (defaults: `slickchook`,
`8443`) using a self-signed certificate generated once at
`/etc/caddy/tls/server.crt`, valid for `CADDY_CERT_DAYS` days (default 365).
All three are environment variables read by the installer:

```bash
sudo --preserve-env=CADDY_HOSTNAME,CADDY_LISTEN_PORT,CADDY_CERT_DAYS \
  CADDY_HOSTNAME=my-vm CADDY_LISTEN_PORT=8443 CADDY_CERT_DAYS=730 \
  ./scripts/install-ubuntu.sh
```

(`sudo` resets most environment variables by default, hence
`--preserve-env`; running the installer from an already-root shell avoids the
issue entirely.)

**Connecting clients must resolve and connect via `CADDY_HOSTNAME`, not a bare
IP.** TLS servers select a certificate by the ClientHello's SNI, and clients
don't send an SNI extension for literal IP addresses — connecting to the
VM's IP directly fails the handshake outright with no usable certificate to
present, regardless of trust. Give the connecting client (e.g. a paired
AI-agent instance) an `/etc/hosts` entry or DNS record mapping
`CADDY_HOSTNAME` to this VM's address.

**Establish trust at provisioning time, not on first connection.** Whatever
is provisioning both the client and this server already has an authoritative
channel to deliver the real certificate — use it. Hand the client the exact
contents of `/etc/caddy/tls/server.crt` (or at minimum its SHA-256
fingerprint, printed at the end of installation) directly, and configure
the client to trust *only* that specific certificate for this connection —
not a shared CA, not the system trust store. A shared CA means every client
trusts every instance in the fleet, which is more than any single pairing
needs; trust-on-first-connection (like SSH host keys) is possible but weaker
still, since whatever intercepts that first connection is trusted forever
after. Provisioning-time pinning has neither weakness, since there's no
window where trust is decided by anything other than the process that's
already authoritative for both sides.

The certificate is generated once and left in place on reinstall/update:
regenerating it on every run would silently invalidate whatever pin a
connected client already has. Delete `/etc/caddy/tls/server.{crt,key}`
manually to force reissuance (and re-deliver the new certificate to
connecting clients).

If a Node-based MCP client (Claude Code included) fails to connect with
no more diagnostic than "Failed to connect," while `curl --cacert
server.crt` against the same endpoint succeeds, the certificate was
likely generated before the `CA:TRUE` fix in SC-014 — delete and
regenerate it as above.

## 6. Verify runner and container execution

First check the endpoint and confirm the service account still has no direct
Docker access:

```bash
curl -N --max-time 2 http://127.0.0.1:8080/sse
sudo -u slickchook docker version
```

The first command should return an SSE `endpoint` event. The second command is
expected to fail with Docker socket permission denied; this confirms that Docker
access is restricted to the setgid runner.

Finally, call the MCP `run_plugin` tool through your MCP client using a real
installed plugin. For the bundled `testssl` plugin, an example tool request is:

```json
{
  "plugin": "testssl",
  "input": {
    "target": "example.com:443",
    "fast": true
  }
}
```

If the MCP client reports `tools/call` as an unknown method, the server build is
not advertising MCP tool capabilities. That is an MCP server configuration
problem, not a runner or Docker permission problem; fix it before using this
step to validate the runner.

Watch the service log while it runs:

```bash
sudo journalctl -u slickchook.service -f
```

A successful response includes a run ID and paths below
`/var/lib/slickchook/artifacts`. Check the resulting permissions and files with:

```bash
sudo find /var/lib/slickchook/artifacts -maxdepth 2 -type f -ls
```

If the runner reports Docker socket permission denied, repeat the `stat` and
`getfacl` checks from step 3 and check whether `/opt` is mounted with `nosuid`:

```bash
findmnt -T /opt -no TARGET,OPTIONS
```

The setgid design cannot work on a filesystem mounted `nosuid`. In that case,
place the binary on a suitable filesystem or replace this design with a
dedicated privileged systemd runner service and a narrowly controlled IPC
interface; do not add the MCP service account to the `docker` group as a
workaround.

## Runner enforcement

The runner reloads the plugin TOML and independently validates Docker policy. It
rejects manifests outside the configured plugin root and artifact directories
outside the configured artifact root.

A manifest's `docker.network` must be `none` or `bridge` — nothing else
validates. `bridge` grants outbound network access; use `none` for any
plugin that doesn't genuinely need it.

Current capability policy:

- default: `cap_add = []`
- raw sockets: `cap_add = ["NET_RAW"]`

Containers run with `--cap-drop ALL`, `--security-opt no-new-privileges`, a
read-only root filesystem, a non-root user, resource limits, no stdin, and only
the individual run artifact directory mounted writable at `/artifacts`.

## Configuring plugin API keys and secrets

Some plugins need a persistent credential for an external service they call
into — a Shodan API key, for example — rather than something an MCP client
supplies per call. A plugin declares this in its manifest as `[[secrets]]`
(see PLUGINS.md), naming the environment variable its container expects and
whether it's required. Configure the actual value once, server-wide, in
`/etc/slickchook/plugin-secrets.env`:

```bash
sudo tee -a /etc/slickchook/plugin-secrets.env >/dev/null <<'EOF'
SHODAN_API_KEY=your-real-key-here
EOF
sudo systemctl restart slickchook.service
```

The installer (and the `.deb` package) creates this file empty, owned
`root:slickchook` with mode `0640`, and the unit loads it via
`EnvironmentFile=-/etc/slickchook/plugin-secrets.env` — the leading `-` makes
it optional, so a fresh install with nothing configured still starts
cleanly. A key's name in this file must exactly match the `env_var` a
plugin's manifest declares.

The value flows entirely through inherited process environment — the
`slickchook` service, then the setgid runner it spawns (which inherits its
parent's environment by default), then `docker run --env NAME` (the bare
variable name, no `=value`, so Docker reads it from the runner's own
environment rather than argv) — and never appears in any argv, log line, or
the `plugin_runs` history table, unlike a client-supplied input (see
SECURITY_CONSIDERATIONS.md's SC-005). `list_plugins` reports
`configured: true/false` per declared secret so an MCP client can see
upfront whether a plugin is usable, without the value itself ever being
returned. A plugin with a missing *required* secret fails the run
immediately, before any Docker container is created.

Once a secret is inside a plugin's container, it is exposed to that
container exactly as fully as the plugin's own code — see SC-018 in
SECURITY_CONSIDERATIONS.md before configuring a real credential for a
plugin you haven't reviewed.

## Importing exported URLs into Burp Suite

A `url-export`-style plugin (see PLUGINS.md) produces a plain-text list of
URLs, one per line — matching exactly what Burp Suite's own native bulk
import expects, confirmed directly against PortSwigger's documentation
rather than assumed:

- **Target tab → Scope sub-tab → Advanced scope control → Load** imports a
  file of URLs/hostnames, one per line, directly into Burp's scope
  (no edition restriction noted — this isn't a Professional-only Live
  Tasks feature). A **Paste URL** button next to it takes a single URL
  from the clipboard the same way. Regex isn't applied to a loaded
  file's entries — each line becomes a literal match.
- This populates **scope** (the allow-list filter controlling what Burp
  will act on), not the **Site Map** itself — the Site Map only fills in
  as Burp actually observes real HTTP traffic (via its proxy or active
  scanning), so loading a scope file doesn't retroactively create Site
  Map entries for URLs Burp has never seen.
- To seed the **Site Map** directly from the same list, install the BApp
  Store extension **Add to SiteMap+**, which reads a plaintext URL file
  and adds its entries into the Site Map for selected hosts directly.
- A universal fallback that works with any intercepting proxy, not just
  Burp: replay each URL through the proxy so it's observed as real
  traffic, e.g. `xargs -I{} curl -s -k -x http://127.0.0.1:8080 {} -o
  /dev/null < urls.txt` (`-k`/`--insecure` skips Burp's self-signed CA
  cert — installing Burp's real CA certificate is the more correct choice
  for anything beyond a quick one-off). Burp's Proxy "Intercept" toggle
  only affects manual interception of in-flight requests, not passive
  Site Map recording, so it doesn't need to be off for this to work.

## Static-analysis input mounts

Every plugin run receives a per-run staging directory at `artifacts/<run-id>/input`.
It is mounted inside the container as `/input`.

Analyzer plugins use the default:

```toml
[docker]
input_read_only = true
```

This mounts `/input` read-only. A fetcher may explicitly opt into writing its own
staging output with `input_read_only = false`; fetchers must remain tightly scoped,
rate-limited, and restricted to the per-run staging directory. The runner rejects
input paths outside the artifact directory and canonicalizes them before mounting.
