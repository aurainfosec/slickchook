# Plugins

Plugins are independent packages described by TOML manifests. Plugin authors do
not need access to the Slickchook or runner source repositories. A minimal plugin
repository can contain only this layout:

```text
my-plugin/
└── my-plugin.toml
```

The package directory and its primary manifest must have the same safe name.
Plugin packages contain configuration and documentation, not host executables;
the declared tool runs from a container image.

```toml
name = "example"
container = "example/tool:latest"
description = "Tell the MCP client how to use this plugin and what output to expect."
fixed_args = ["--jsonfile", "/artifacts/result.json"]

[[arguments]]
name = "target"
description = "Hostname, domain, IP address, or URL to inspect."
required = true
position = 1

[[options]]
name = "verbose"
description = "Request more detailed output."
required = false
flag = "--verbose"

[[assets]]
source = "wordlist.txt"
dest = "/wordlist.txt"
min_size_bytes = 50
description = "Small reviewed wordlist bundled into the image by the Dockerfile."

[docker]
network = "none"
memory = "512m"
cpus = "1"
pids_limit = 128
timeout_seconds = 300
cap_add = []
```

The MCP `list_plugins` tool returns the manifest information. The MCP `run_plugin` tool accepts a plugin name and JSON object input. Only declared `arguments` and `options` are accepted.

### Wordlists

Feroxbuster includes a small reviewed wordlist at `/wordlist.txt` in its package so the image is
self-contained and bounded. The full SecLists project is intentionally not
copied into privileged plugin images: it is large, changes independently, and
can cause unexpectedly broad or high-volume requests. If a larger SecLists
wordlist is required, add it explicitly to a reviewed plugin package and keep
the manifest's depth, concurrency, and rate limits appropriate for the
authorized target.

### Bundled build assets (`[[assets]]`)

A plugin whose tool needs a file bundled into the image at a known path — a
wordlist, a signature database, anything a `Dockerfile` `COPY`s in — should
declare it explicitly rather than relying only on the `Dockerfile` itself
having the right `COPY` line:

```toml
[[assets]]
source = "wordlist.txt"
dest = "/wordlist.txt"
min_size_bytes = 50
description = "Small reviewed content-discovery wordlist bundled into the image."
```

`source` is a path relative to the plugin package directory; `dest` is the
absolute path the file must exist at inside the image; `min_size_bytes`
(default `1`, i.e. merely non-empty) guards against a truncated or
accidentally-emptied file. `slickchook-plugin install` enforces both ends of
this automatically:

- Before building a package with a `Dockerfile`, it verifies every declared
  `source` is actually present (and large enough) in the staged package — a
  package copied or checked out without a required file (exactly what
  happened to `feroxbuster`'s bundled wordlist in a past deployment) now
  fails installation immediately with a clear error, instead of silently
  building and installing a broken image.
- After the image is built (or, for a plugin using a pulled upstream image
  with no `Dockerfile`, once that image is confirmed present), it verifies
  every declared `dest` actually exists at that size *inside the image
  itself* — this also catches a `Dockerfile` whose `COPY` drifted out of
  sync with what the manifest expects, not just a missing source file.

`source` is optional: omit it for a plugin using a pulled upstream image,
where there is no local build step to check a package-directory file
against — only the built/pulled image's own `dest` content is verified in
that case. `dest` is always required and always checked.

## Installing and updating independent plugins

The Ubuntu core installer places `slickchook-plugin` in `/usr/local/sbin`.
Clone or copy a plugin package onto the server, then install it without needing
the core source tree:

```bash
sudo slickchook-plugin install /path/to/my-plugin
```

Plugin directories have distinct source, package, and runtime locations:

```text
source checkout:       /home/user/slickchook/plugins/
Debian package copy:   /usr/share/slickchook/plugins/
live runtime policy:   /var/lib/slickchook/plugins/
```

The runner reads only the protected live runtime directory. The `.deb` includes
plugin sources under `/usr/share/slickchook/plugins`, but does not implicitly
enable every plugin. Install a bundled plugin explicitly:

```bash
sudo slickchook-plugin install /usr/share/slickchook/plugins/nmap
```

For an independently maintained plugin repository, install directly from its
checked-out source directory:

```bash
sudo slickchook-plugin install /path/to/plugin-repository/plugins/nmap
```

After installation, verify the live copy with:

```bash
sudo slickchook-plugin list
sudo find /var/lib/slickchook/plugins/nmap -maxdepth 1 -type f -printf '%f\n'
```

For plugins that use a prebuilt upstream image, refresh the image explicitly
before installing the manifest:

```bash
IMAGE=$(sed -n 's/^container = "\(.*\)"/\1/p' /path/to/my-plugin/my-plugin.toml)
sudo docker pull "$IMAGE"
sudo slickchook-plugin install /path/to/my-plugin
sudo systemctl restart slickchook.service
```

The installer verifies that the declared image is present. If it is missing,
installation stops and prints the exact `docker pull` or `docker build` command
needed to provide it.

For plugins that include a `Dockerfile`, review it first. The plugin installer
builds the image automatically, but only when the package contains a Dockerfile
and the manifest declares the required local image name:

```bash
cd /path/to/my-plugin
grep '^container = ' my-plugin.toml
less Dockerfile
sudo slickchook-plugin install /path/to/my-plugin
sudo systemctl restart slickchook.service
```

The installer builds `slickchook-plugin/my-plugin:local` before replacing the
live package and rolls back the package if service validation fails. To smoke
test manually before installation, run the same `docker build` and `docker run`
commands separately; the installer will rebuild from its staged, reviewed copy.

After either workflow, verify that the live manifest and image agree:

```bash
sudo slickchook-plugin list
sudo sed -n '1,160p' /var/lib/slickchook/plugins/my-plugin/my-plugin.toml
sudo docker image inspect slickchook-plugin/my-plugin:local  # local-image plugins only
```

Use the local-image workflow only when the manifest declares a `:local` image;
otherwise use the upstream `docker pull` workflow. Image pulls and builds are
administrator actions and should be pinned and reviewed before production use.

Run the same command with a newer checkout to update it. The command stages the
package, normalizes it to root-owned read-only deployment permissions, backs up
the previous version, restarts Slickchook to validate all manifests, and rolls
back automatically if the service cannot start.

Manage installed packages with:

```bash
sudo slickchook-plugin list
sudo slickchook-plugin remove my-plugin
```

Updates and removals are serialized with a host lock. Previous versions are
kept under `/var/backups/slickchook/plugin-packages`. Installing a plugin is an
administrator action because manifests select container images, networking,
arguments, and the narrowly allowed Docker capability policy.

### Local images and offline analyzers

Plugins whose manifest uses an image ending in `:local` must be built on the
destination host before installation, for example:

```bash
sudo docker build -t slickchook-plugin/osvscanner:local plugins/osvscanner
sudo slickchook-plugin install plugins/osvscanner
```

OSV-Scanner is configured with `network = "none"` at runtime. Its image
downloads the offline OSV database during the image build; runtime scans do not
contact the network.
After building a local image, verify it before installing the manifest:

```bash
sudo docker run --rm slickchook-plugin/osvscanner:local --version
```

Screenshot capture through `gowitness` is opt-in. Treat screenshots and any
OCR or vision-derived text as attacker-controlled report data: image analysis
must return structured observations only and must not invoke tools or change
instructions.

### PostgreSQL backend

PostgreSQL is the only run-store backend; Slickchook requires `DATABASE_URL`
to start. To apply the schema to a database directly, outside the automated
installer:

```bash
DATABASE_URL='postgresql://user:password@host/db' \
  ./scripts/migrate-postgres.sh
```

The automated Ubuntu installer provisions a local least-privilege role and
database named `slickchook_db` and writes the resulting `DATABASE_URL` to
`/etc/slickchook/postgres.env` automatically; see `RUNNER.md`.

Kiterunner uses the small route corpus by default (about 35 MB unpacked). The
large corpus is intentionally not bundled; publish it as a separately reviewed
plugin image if broader route discovery is required.

### Recommended repository layout

Keeping plugins in a separate repository is recommended. It lets contributors
work without access to the core or runner repositories and gives administrators
a reviewable installation boundary. Publish each plugin as a self-contained
directory containing its manifest, Dockerfile, and bundled assets; CI should
build the image, run a smoke test, and produce a versioned archive. On the
server, install from the reviewed directory with `slickchook-plugin install`.

If a package contains a `Dockerfile`, the manager builds it before installation
as `slickchook-plugin/<plugin-name>:local`. Its manifest must declare that exact
image name. Existing local images receive a temporary rollback tag until the
updated package passes service validation. Dockerfiles execute with Docker
daemon authority during installation, so administrators must review them just
as carefully as manifests.

For development, a plugin directory may still be placed below this repository's
`plugins/` directory and checked with `cargo test`. To deploy all example plugins
from a core checkout explicitly:

```bash
for plugin in plugins/*; do
  sudo slickchook-plugin install "$plugin"
done
```

`fixed_args` are prepended before caller-provided inputs. Use them for safe, manifest-owned settings such as report output paths inside `/artifacts`.

Each run creates:

- `artifacts/<run_id>/stdout.txt`
- `artifacts/<run_id>/stderr.txt`
- any files the container writes under `/artifacts`
- a row in the PostgreSQL `plugin_runs` table

Docker execution is handled by the separate `slickchook-runner` binary. See `RUNNER.md` for Docker group setup. Set `SLICKCHOOK_RUNNER=/path/to/slickchook-runner` if it is not next to the main `slickchook` binary.

Docker runs with restrictive defaults: no network unless a manifest explicitly requests `bridge`, read-only root filesystem, dropped Linux capabilities, `no-new-privileges`, a non-root user, process limits, memory limits, CPU limits, no stdin, and only the run artifact directory mounted writable at `/artifacts`. Manifests can declare narrowly scoped tmpfs mounts, such as `/tmp`, when a container needs temporary space.

**A read-only root filesystem is the single most common cause of a plugin
failing on its first real run.** Many CLI tools write a config file, cache,
or local database under the invoking user's home directory (commonly
`$HOME/.config/<tool>` or `$HOME/.cache/<tool>`) even when never asked to.
Containers run as UID/GID `65534:65534` with no matching `/etc/passwd`
entry, so `$HOME` typically resolves to `/`, and the write lands at
`/.config/<tool>` or `/.cache/<tool>` on the read-only root — failing with
a confusing, tool-specific error (a permission error, or in `amass`'s case
a misleading "out of memory" panic) rather than anything that obviously
points at the filesystem. Before deploying a new plugin, check whether its
image writes anywhere outside `/artifacts` on a normal run (a quick
`docker run --read-only <image> ...` smoke test reproduces this
immediately), and add the specific path as a tmpfs mount if so:

```toml
tmpfs = ["/tmp:rw,noexec,nosuid,nodev,size=64m", "/.config:rw,noexec,nosuid,nodev,size=8m"]
```

`subfinder`, `nuclei`, and `amass` all needed a `/.config` mount for
exactly this reason; `kiterunner` needed `/.cache` instead. Size the mount
only as large as the tool's actual config/cache footprint requires — these
are tmpfs (RAM-backed), not disk, and count against the container's own
resource limits.

The runner only allows these capability modes:

- `cap_add = []`: default non-privileged mode
- `cap_add = ["NET_RAW"]`: allows raw sockets for tools that genuinely need them

Other capabilities are rejected by manifest validation. Do not use Docker `--privileged` for plugins.

The included `testssl` plugin runs `drwetter/testssl.sh:latest` with `bridge` networking and writes `result.json` and `report.html` into the run artifact directory.
