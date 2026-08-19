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

[[secrets]]
env_var = "EXAMPLE_API_KEY"
description = "API key for the upstream service this tool queries."
required = true

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

### Declaring required secrets (`[[secrets]]`)

A plugin whose tool needs a persistent credential for an external service —
an API key for Shodan, for example — should declare it explicitly rather
than accepting it as a client-supplied `option`:

```toml
[[secrets]]
env_var = "SHODAN_API_KEY"
description = "Shodan API key, used to query host and service data."
required = true
```

`env_var` names the environment variable the plugin's container expects to
find already set; `required` (default `true`) controls whether a missing
value fails the run immediately or is simply omitted. The actual value is
never part of the manifest, never supplied per call by an MCP client, and
never ends up in `plugin_runs` history or any log line — an administrator
configures it once, server-wide, in `/etc/slickchook/plugin-secrets.env`.
See RUNNER.md's "Configuring plugin API keys and secrets" for the full
deployment-side mechanism, and SECURITY_CONSIDERATIONS.md's SC-018 for the
residual trust boundary: once inside the container, the value is exposed to
that container exactly as fully as the plugin's own code.

### Wrapping an upstream tool that wants the secret as a CLI flag

`[[secrets]]` only gets a value as far as an environment variable inside
the container — it doesn't know how a specific tool expects to receive a
credential. Some tools read an env var directly; many don't, and expect
`--api-token <value>` (or similar) on the command line, or a one-time
`init` step. For those, wrap the real binary with a tiny shell entrypoint
that reads the env var and does the translation itself, so the value still
never passes through slickchook's own argv construction — only ever
inside the container, at the one point it's actually needed:

```dockerfile
FROM realtool/realtool:latest
COPY entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod 0555 /usr/local/bin/entrypoint.sh
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
```

```sh
#!/bin/sh
set -eu
: "${REALTOOL_API_TOKEN:?REALTOOL_API_TOKEN must be configured}"
exec realtool --api-token "$REALTOOL_API_TOKEN" "$@"
```

`wpscan` and `shodan-host`/`shodan-search` (below) both use this pattern —
`wpscan`'s Dockerfile builds `FROM wpscanteam/wpscan:latest` and only adds
the wrapper; the `shodan-*` plugins build a small image from scratch
(`shodan`'s own CLI has no official image) and use `shodan init` as the
one-time credential step instead of a per-call flag.

## Custom (first-party) plugins

Most bundled plugins are thin manifests around an existing, independently
published security tool (`nmap`, `testssl`, `amass`, ProjectDiscovery's
tools, and so on) — the manifest just declares how to invoke a real
upstream image. A few are not: they run bespoke code written for this
repository, with no equivalent standalone published tool behind them.
Know which is which before relying on one for engagement coverage:

- **`screensanitize`** — a small Python script (`plugins/screensanitize/sanitize.py`)
  that re-encodes staged screenshots, runs OCR (`tesseract`) over them, and
  flags hidden/low-contrast text and encoded content (Base64, Morse,
  Braille, QR codes, suspicious Unicode). Written specifically to make
  `gowitness` screenshots safe to hand to an AI reader — see
  `SECURITY_CONSIDERATIONS.md` SC-017. There is no third-party tool
  equivalent; its detection coverage is only as good as this script.
- **`graphqlcheck`** — a small Python script (`plugins/graphqlcheck/check.py`)
  that sends a single bounded GraphQL introspection query to one URL and
  reports whether introspection is enabled. It is not a full GraphQL
  security scanner (no batch queries, no known-vulnerability checks) — it
  answers exactly one question.
- **`sublert`** — despite the name, this is **not** the real upstream
  [`sublert`](https://github.com/yassineaboukir/sublert) project (which
  continuously monitors and maintains its own database). It's a small
  bespoke script (`plugins/sublert/collect_ct.py`) that makes one query to
  the public [crt.sh](https://crt.sh/) certificate-transparency search and
  prints the distinct subdomains found for that run — a single-shot
  lookup, not ongoing monitoring. Don't assume upstream `sublert`'s feature
  set or behavior applies here.
- **`webfetch`** — a thin, purpose-built restriction around plain `wget`
  (fetch exactly one authorized URL into the run's input staging
  directory, nothing else) rather than a general-purpose fetch tool. See
  `ROADMAP.md`'s static-web-content-analysis section for the fetch→analyze
  pipeline this exists to support.
- **`rawping`** — a bounded three-packet ICMP echo using plain `alpine`'s
  built-in `ping`, not a dedicated network-tools image. It exists as a
  minimal, focused verification of the runner's `NET_RAW` capability
  policy, not as a general reachability/latency tool.

A related but distinct category: `wpscan` and `shodan-host`/`shodan-search`
run the real upstream tool (WPScan, the official `shodan` CLI) with no
reimplemented logic of their own — their capabilities and limitations are
whatever the real tool's are — but wrap it in a thin entrypoint script
purely to translate an injected `[[secrets]]` value into the CLI flag or
`init` step that tool expects (see "Wrapping an upstream tool that wants
the secret as a CLI flag" above). Don't confuse this wrapper with a
bespoke reimplementation; it adds nothing to what the real tool does.

If a plugin isn't in the list above, treat its manifest `description` and
underlying `container` image as accurately reflecting a real, independently
maintained tool — check that upstream project's own documentation for its
actual capabilities and limitations.

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

A `docker build` whose `Dockerfile` and build context are unchanged correctly
reuses Docker's cached layers — the resulting image is identical either way,
so this is expected, not a sign the install skipped anything. Pass
`--force-rebuild` (before or after the plugin directory) to bypass that cache
explicitly, for example when specifically diagnosing or ruling out a
suspected stale image:

```bash
sudo slickchook-plugin install --force-rebuild /path/to/my-plugin
```

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

Docker execution is handled by the separate `slickchook-runner` binary. See
`RUNNER.md`'s "Runner enforcement" section for the container hardening
defaults (network, capabilities, read-only filesystem, resource limits)
and Docker group setup; set `SLICKCHOOK_RUNNER=/path/to/slickchook-runner`
if it is not next to the main `slickchook` binary. Manifests can declare
narrowly scoped tmpfs mounts, such as `/tmp`, when a container needs
temporary space.

**A read-only root filesystem is the single most common cause of a plugin
failing on its first real run.** Many CLI tools write a config file, cache,
or local database under the invoking user's home directory (commonly
`$HOME/.config/<tool>` or `$HOME/.cache/<tool>`) even when never asked to.
Containers run as UID/GID `65534:65534` with no matching `/etc/passwd`
entry, so `$HOME` typically resolves to `/`, and the write lands at
`/.config/<tool>` or `/.cache/<tool>` on the read-only root — failing with
a confusing, tool-specific error (a permission error, an "out of memory"
panic in `amass`'s case, or `katana`'s flatly misleading "Could not read
flags: no such file or directory" — that one isn't a flag-parsing bug at
all, it's `$HOME` resolution failing while trying to read a *default
config file* before argument parsing even begins) rather than anything
that obviously points at the filesystem. Before deploying a new plugin,
check whether its image writes anywhere outside `/artifacts` on a normal
run (a quick `docker run --read-only <image> ...` smoke test reproduces
this immediately).

Two ways to fix it, depending on whether you control the Dockerfile:

- **If you do** (building from source, or wrapping an official image in a
  thin `FROM upstream/tool:tag` layer) — set `ENV HOME=/tmp` and add a
  plain `/tmp` tmpfs mount. This is the simpler, more robust fix: one
  setting handles *any* path the tool tries to use under `$HOME`, without
  having to discover the tool's exact subpath first. `wpscan`,
  `shodan-host`/`shodan-search`, `subfaster`, `virustotal-resolutions`, and
  `katana` all use this.
- **If you don't** (a plugin with no `Dockerfile` at all, pulling an
  upstream image directly) — you can't set `ENV HOME`, so mount the
  specific path the tool actually uses instead:

  ```toml
  tmpfs = ["/tmp:rw,noexec,nosuid,nodev,size=64m", "/.config:rw,noexec,nosuid,nodev,size=8m"]
  ```

  `subfinder`, `nuclei`, and `amass` all needed a `/.config` mount for
  exactly this reason; `kiterunner` needed `/.cache` instead. Size the
  mount only as large as the tool's actual config/cache footprint requires
  — these are tmpfs (RAM-backed), not disk, and count against the
  container's own resource limits.

Plugin containers run as UID 65534 by default. Some tools restrict permissions on their own output files as they write them (for example, `testssl.sh` writing its JSON/HTML reports); left alone, that leaves the file unreadable by the Slickchook service's own user afterward, so `read_artifact` and any other host-side read would fail even though the file exists. The runner fixes this automatically after every container exits — see SC-015 in `SECURITY_CONSIDERATIONS.md` for how. Plugin authors do not need to work around this themselves.

A handful of tools' real functionality needs actual root, not just a
Docker capability grant — `[docker].run_as_root = true` runs both the
plugin container and its permission-fixup follow-up as UID 0 instead.
`nmap` is the reason this field exists: `-sS`/`-sU`/every scan technique
besides plain TCP connect, `-O` OS detection, and `-f` fragmentation all
hard-fail with Nmap's own "requires root privileges" error under UID
65534 even with `NET_RAW` granted — confirmed live, not assumed, and it
turned out `[docker].cap_add = ["NET_RAW"]` alone had never been enough
for any of that. This is a real, deliberate exception to the fleet's
unprivileged-by-default policy — set it only when a tool's own
documented behavior (or a live "requires root" error) actually demands
it, not preemptively. Setting it also requires the host's own
`/var/lib/slickchook/artifacts` ACL to grant UID 0 alongside the usual
UID 65534 (see `install-ubuntu.sh`/`build-deb.sh`'s `setfacl` calls) —
without that, the permission-fixup step itself fails with a bare
`Permission denied`, since a `--cap-drop ALL` container's root has no
`CAP_DAC_OVERRIDE` and is still subject to normal file permission bits.
Not every tool needing raw sockets actually needs this, though: `naabu`
checks for the `NET_RAW` *capability* directly and falls back to a
CONNECT scan with a log line if it's unavailable, rather than hard
failing — test a tool's actual behavior before assuming it needs
`run_as_root` just because it does something privileged-sounding.

### Verify against the tool's real behavior, not its docs or `--help` text

Every plugin in this fleet has, at some point, had a flag or an assumed
behavior turn out wrong in a way only a real invocation caught — never a
hypothetical concern, always a live bug that shipped and then got fixed.
Treat each of these as a standing checklist before trusting a new
manifest, not just historical trivia:

- **Docs and generated help pages can be stale relative to the exact
  version you're pinning.** Fetching a repo's `doc/` files (or any file)
  without pinning the ref returns the *default branch*, which can already
  be ahead of the release/commit your Dockerfile actually builds.
  `virustotal-resolutions`' first attempt used docs describing
  dedicated `vt domain resolutions`-style subcommands that don't exist in
  the pinned build at all — reading the actual source at the exact pinned
  commit (`?ref=<sha>` on a GitHub API content fetch, or the equivalent
  for whatever host) resolved it, and should have been the first move,
  not the fallback after a live failure.
- **A tool can require a one-time setup step before its real
  functionality even exists as a command.** `vt-cli` only registers
  per-relationship subcommands from a local cache file that `vt init`
  writes; skip `init` and the subcommand silently doesn't exist, falling
  through to a different command entirely with a confusing error. `shodan
  init` is the same shape of requirement. If a tool ships an `init`
  or similar setup subcommand, assume it's load-bearing until proven
  otherwise, not just a convenience wrapper around writing a config file.
- **A flag that looks combinable in `--help` output might not be.**
  Naabu's `-ip-version` genuinely accepts `4,6` joined together; Katana's
  `-known-files` help text is formatted identically —
  `(all,robotstxt,sitemapxml)` — but is actually a strict single-choice
  enum, and errors outright if you join two values. The visual pattern in
  `--help` text is not a reliable signal either way; run the actual
  invocation with a real value before writing the option's description as
  fact.
- **A version tag in `--help`/release notes doesn't guarantee it's a
  valid Go module version.** `go install pkg@vX.Y.Z` requires the
  underlying git tag to actually be `vX.Y.Z` — `vt-cli`'s real tags are
  `1.3.1` etc., with no `v` prefix, so the "obvious" invocation failed
  with `unknown revision`. Pinning by the tag's exact commit SHA instead
  sidesteps the mismatch entirely and is equally reproducible.
- **The install step never runs the container, for any plugin.**
  `slickchook-plugin install` validates declared `[[assets]]`, that the
  manifest parses, and that the service restarts — nothing more. No
  runtime-only bug (a missing dependency, a wrong default path, an
  argument-parsing quirk) can ever be caught before a real `run_plugin`/
  `start_plugin` call actually happens. Treat a merged manifest change as
  unverified until it's been exercised live against a real target, not as
  done once it builds.
- **When a fix turns out wrong, say so plainly instead of quietly
  editing history.** A wrong fix shipped for `shodan-host`/`shodan-search`
  earlier this fleet's history (an unpinned `setuptools` "fix" that
  didn't actually work); the correction was a new changelog entry stating
  the prior one was wrong, not a silent edit to the original. The record
  of what was tried and why it didn't work is worth as much as the
  eventual right answer.

Manifest validation rejects any capability beyond the two RUNNER.md
documents (default none, or `NET_RAW` for tools that genuinely need raw
sockets). Do not use Docker `--privileged` for plugins.

The included `testssl` plugin runs `drwetter/testssl.sh:latest` with `bridge` networking and writes `result.json` and `report.html` into the run artifact directory.
