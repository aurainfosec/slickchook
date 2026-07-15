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

## Installing and updating independent plugins

The Ubuntu core installer places `slickchook-plugin` in `/usr/local/sbin`.
Clone or copy a plugin package onto the server, then install it without needing
the core source tree:

```bash
sudo slickchook-plugin install /path/to/my-plugin
```

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

### PostgreSQL migration status

The PostgreSQL schema is staged but SQLite remains the active runtime backend.
To apply the reviewed schema to a separate PostgreSQL database:

```bash
DATABASE_URL='postgresql://user:password@host/db' \
  ./scripts/migrate-postgres.sh
```

Applying this schema does not migrate existing SQLite rows; do not switch a
production service until the repository backend and data migration tooling are
implemented and tested.

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
- a row in `artifacts/plugin_runs.sqlite3`

Docker execution is handled by the separate `slickchook-runner` binary. See `RUNNER.md` for Docker group setup. Set `SLICKCHOOK_RUNNER=/path/to/slickchook-runner` if it is not next to the main `slickchook` binary.

Docker runs with restrictive defaults: no network unless a manifest explicitly requests `bridge`, read-only root filesystem, dropped Linux capabilities, `no-new-privileges`, a non-root user, process limits, memory limits, CPU limits, no stdin, and only the run artifact directory mounted writable at `/artifacts`. Manifests can declare narrowly scoped tmpfs mounts, such as `/tmp`, when a container needs temporary space.

The runner only allows these capability modes:

- `cap_add = []`: default non-privileged mode
- `cap_add = ["NET_RAW"]`: allows raw sockets for tools that genuinely need them

Other capabilities are rejected by manifest validation. Do not use Docker `--privileged` for plugins.

The included `testssl` plugin runs `drwetter/testssl.sh:latest` with `bridge` networking and writes `result.json` and `report.html` into the run artifact directory.
