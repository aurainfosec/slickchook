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
- PostgreSQL packages are installed and the local service is enabled by the
  installer for the upcoming durable backend; SQLite remains the active
  Slickchook backend until migration validation is complete;
- the repository is the current directory; and
- Rust is installed on the build machine.

## Automated installation

For a standard Ubuntu Server, build the release binaries and run the included
installer from the repository root:

```bash
cargo build --locked --release --bins
sudo ./scripts/install-ubuntu.sh
```

The script performs the steps below, installs `deploy/slickchook.service`, and
stops with an error if the runner ownership, setgid mode, ACL, Docker socket, or
mount options are unsafe. Keep the manual procedure below for auditing and
troubleshooting.

The same command is used for new installations and updates. On an update it:

- stops an existing `slickchook.service` before replacing binaries;
- repairs legacy service-account settings and removes unsafe direct membership
  in the `docker` group;
- preserves the artifact database and all existing run artifacts;
- leaves independently installed plugin packages in place;
- backs up changed core binaries, helper commands, and the systemd unit below
  `/var/backups/slickchook` before replacing them; and
- refuses to proceed if an unmanaged process is listening on port 8080.

Plugin packages have their own installation lifecycle and are not copied or
updated by this core installer. See `PLUGINS.md`.

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

Repeat for each plugin you want to enable. The installed copy is then visible
under `/var/lib/slickchook/plugins`; do not edit that live tree directly.

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

Current capability policy:

- default: `cap_add = []`
- raw sockets: `cap_add = ["NET_RAW"]`

Containers run with `--cap-drop ALL`, `--security-opt no-new-privileges`, a
read-only root filesystem, a non-root user, resource limits, no stdin, and only
the individual run artifact directory mounted writable at `/artifacts`.
# Static-analysis input mounts

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
