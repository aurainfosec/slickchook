use anyhow::{Context, Result, anyhow, bail};
use serde_json::Value;
use slickchook::plugin_manager::{
    DockerPolicy, build_container_args, load_manifest, missing_required_secrets,
    validate_docker_policy,
};
use std::{
    env, fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};
use tokio::{io::AsyncReadExt, process::Command, time};

/// Upper bound, per stream, on how much of a plugin container's stdout/stderr
/// this runner holds in host memory. `docker run`'s own `.output()` buffers
/// both streams in full with no limit of its own - target-controlled content
/// echoed back through a tool's own output (a crawler's JSONL, a verbose
/// scanner, etc.) could otherwise grow that buffer without bound regardless
/// of the plugin container's own `--memory` limit, since streaming output
/// through a pipe doesn't count against the container's resident memory the
/// way holding it in the container's own heap would. Generous by design -
/// see SECURITY_REVIEW_2026-10-20.md's R7 for the full reasoning - so a
/// legitimate large crawl or scan result isn't silently clipped; override
/// with `SLICKCHOOK_MAX_PLUGIN_OUTPUT_BYTES` for an unusually output-heavy
/// deployment.
const DEFAULT_MAX_PLUGIN_OUTPUT_BYTES: usize = 256 * 1024 * 1024;

fn max_plugin_output_bytes() -> usize {
    env::var("SLICKCHOOK_MAX_PLUGIN_OUTPUT_BYTES")
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|&value| value > 0)
        .unwrap_or(DEFAULT_MAX_PLUGIN_OUTPUT_BYTES)
}

/// Output of a bounded-capture run: like `std::process::Output`, but each
/// stream was capped at `max_bytes` while still being fully drained (so a
/// chatty child never blocks on a full pipe), with `_truncated` recording
/// whether the real stream was actually longer than what was kept.
struct BoundedOutput {
    status: std::process::ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    stdout_truncated: bool,
    stderr_truncated: bool,
}

/// Reads `reader` to completion, keeping at most `max_bytes` of it. Bytes
/// beyond the cap are read and discarded rather than left unread, so an
/// over-producing child's pipe never fills and stalls it - draining is what
/// bounds *this process's* memory; it does not stop the child from running
/// or from continuing to produce output.
async fn read_capped<R>(mut reader: R, max_bytes: usize) -> io::Result<(Vec<u8>, bool)>
where
    R: tokio::io::AsyncRead + Unpin,
{
    let mut buf = Vec::with_capacity(max_bytes.min(64 * 1024));
    let mut chunk = [0u8; 64 * 1024];
    let mut truncated = false;
    loop {
        let read = reader.read(&mut chunk).await?;
        if read == 0 {
            break;
        }
        if buf.len() < max_bytes {
            let keep = (max_bytes - buf.len()).min(read);
            buf.extend_from_slice(&chunk[..keep]);
            if keep < read {
                truncated = true;
            }
        } else {
            truncated = true;
        }
    }
    Ok((buf, truncated))
}

/// Runs `command`, capturing stdout/stderr the same way `Command::output()`
/// does but with `read_capped`'s per-stream byte bound instead of unbounded
/// buffering. Both streams are read concurrently with the child's own
/// execution (required, not an optimization: reading them sequentially could
/// deadlock if the child fills one pipe while blocked writing to the other).
async fn run_with_bounded_output(mut command: Command, max_bytes: usize) -> io::Result<BoundedOutput> {
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command.spawn()?;
    let stdout = child
        .stdout
        .take()
        .expect("stdout was configured as piped");
    let stderr = child
        .stderr
        .take()
        .expect("stderr was configured as piped");

    let ((stdout_bytes, stdout_truncated), (stderr_bytes, stderr_truncated), status) = tokio::try_join!(
        read_capped(stdout, max_bytes),
        read_capped(stderr, max_bytes),
        child.wait(),
    )?;

    Ok(BoundedOutput {
        status,
        stdout: stdout_bytes,
        stderr: stderr_bytes,
        stdout_truncated,
        stderr_truncated,
    })
}

/// Appends a clear, human-visible marker to truncated output so a truncated
/// artifact never looks like the tool's own complete, unremarkable output.
fn mark_truncated(mut bytes: Vec<u8>, max_bytes: usize) -> Vec<u8> {
    bytes.extend_from_slice(
        format!(
            "\n[slickchook runner: output truncated at {max_bytes} bytes; the plugin's actual \
             output was longer]"
        )
        .as_bytes(),
    );
    bytes
}

/// UID:GID a plugin's container (and its permission-fixup follow-up) runs
/// as. `0:0` only for a plugin whose manifest opts into
/// `DockerPolicy::run_as_root` - see that field's doc comment for why.
fn container_user(docker: &DockerPolicy) -> &'static str {
    if docker.run_as_root {
        "0:0"
    } else {
        "65534:65534"
    }
}

struct RunnerArgs {
    manifest: PathBuf,
    plugins_root: PathBuf,
    artifact_dir: PathBuf,
    artifacts_root: PathBuf,
    input_dir: Option<PathBuf>,
    input_json: String,
}

#[tokio::main]
async fn main() {
    // A separate, minimal mode from the normal `--manifest ...` plugin-run
    // path: `PluginManager::cancel_run` needs to kill a running container by
    // name, but the *main* slickchook service process is deliberately never
    // added to the `docker` group (see RUNNER.md) - only this setgid binary
    // can reach the Docker socket at all. Routing the kill through here,
    // rather than the service process invoking `docker` directly, is the
    // actual fix; invoking `docker` from the unprivileged service process
    // fails with a permission error that a narrow `.map_err`/`warn!` can
    // easily look like a harmless "container already exited" case instead
    // of the real cause (confirmed live: this exact gap shipped and the
    // kill silently did nothing before this was added).
    let mut args = env::args().skip(1);
    if args.next().as_deref() == Some("--kill-container") {
        let Some(name) = args.next() else {
            eprintln!("--kill-container requires a container name");
            std::process::exit(2);
        };
        std::process::exit(kill_container(&name).await);
    }

    if let Err(error) = run().await {
        eprintln!("{error:#}");
        std::process::exit(1);
    }
}

/// Validates `name` matches the exact shape `PluginManager::cancel_run`
/// constructs (`slickchook-<run_id>`, `run_id` always a server-generated
/// UUID) before running `docker kill` - this binary's setgid bit grants
/// Docker socket access specifically so the unprivileged main service
/// process never needs it directly; accepting an arbitrary name here would
/// hand that privilege to whatever argv a caller supplies instead, the same
/// class of concern `validate_runner_paths` already guards for
/// `--manifest`/`--artifact-dir`. Returns the process exit code to use.
/// `true` for exactly the shape `PluginManager::cancel_run` constructs
/// (`slickchook-<run_id>`, `run_id` always a server-generated UUID) -
/// factored out from `kill_container` so this check is directly testable
/// without invoking `docker`.
fn is_valid_container_name_to_kill(name: &str) -> bool {
    match name.strip_prefix("slickchook-") {
        Some(suffix) if !suffix.is_empty() => {
            suffix.chars().all(|ch| ch.is_ascii_hexdigit() || ch == '-')
        }
        _ => false,
    }
}

async fn kill_container(name: &str) -> i32 {
    if !is_valid_container_name_to_kill(name) {
        eprintln!("refusing to kill container with unexpected name `{name}`");
        return 2;
    }

    match time::timeout(
        Duration::from_secs(10),
        Command::new("docker")
            .arg("kill")
            .arg(name)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output(),
    )
    .await
    {
        Ok(Ok(output)) if output.status.success() => 0,
        Ok(Ok(output)) => {
            // A container that already exited naturally (the run finished
            // right as cancellation landed) is the most likely reason this
            // fails in practice - cancel_run logs this at debug level, not
            // as a scary warning, for exactly that reason. The real exit
            // code (not forced to 0) and stderr both still pass through so
            // a genuine failure isn't silently indistinguishable from that
            // harmless case.
            eprint!("{}", String::from_utf8_lossy(&output.stderr));
            output.status.code().unwrap_or(1)
        }
        Ok(Err(error)) => {
            eprintln!("failed to run docker kill: {error}");
            1
        }
        Err(_) => {
            eprintln!("docker kill timed out");
            1
        }
    }
}

async fn run() -> Result<()> {
    let args = parse_args()?;
    validate_runner_paths(&args)?;
    let artifact_dir = canonicalize_dir(&args.artifact_dir)?;
    let input_dir = args
        .input_dir
        .as_ref()
        .map(|path| canonicalize_dir(path))
        .transpose()?;
    if let Some(input_dir) = &input_dir {
        let artifacts_root = canonicalize_dir(&args.artifacts_root)?;
        if !input_dir.starts_with(&artifacts_root) {
            bail!("input dir is outside artifacts root");
        }
    }
    let manifest = load_manifest(&args.manifest)?;
    validate_docker_policy(&manifest.docker)?;

    // Independent re-validation of what `PluginManager::run_plugin_with_input`
    // already rejects server-side, matching the existing pattern of the
    // runner never trusting the server passed a valid call (see
    // `missing_required_secrets`'s doc comment). The run's own fresh input
    // directory (`<artifact_dir>/input`, always created before the runner is
    // invoked, chained or not) is the only input directory a writable
    // fetcher may legitimately be mounted against read-write; anything else
    // can only be a previous run's directory, which this plugin's own
    // output would then mutate - see SECURITY_REVIEW_2026-10-20.md's R5.
    if let Some(input_dir) = &input_dir {
        let own_fresh_input_dir = canonicalize_dir(&artifact_dir.join("input"))?;
        if *input_dir != own_fresh_input_dir && !manifest.docker.input_read_only {
            bail!(
                "plugin `{}` has writable input (`docker.input_read_only = false`) and was \
                 given an input directory other than its own run's - refusing to mount a \
                 chained prior run's input read-write",
                manifest.name
            );
        }
    }

    let missing_secrets = missing_required_secrets(&manifest);
    if !missing_secrets.is_empty() {
        bail!(
            "plugin `{}` requires the following environment variable(s) to be configured on the slickchook service before it can run: {}",
            manifest.name,
            missing_secrets.join(", ")
        );
    }

    let input: Value =
        serde_json::from_str(&args.input_json).context("failed to parse plugin input JSON")?;
    let container_args = build_container_args(&manifest, &input)?;
    eprintln!(
        "slickchook runner container={} args={:?}",
        manifest.container, container_args
    );
    tracing::info!(container = %manifest.container, args = ?container_args, "launching plugin container");

    let container_name = format!(
        "slickchook-{}",
        artifact_dir
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| anyhow!("artifact dir has no usable name"))?
    );

    let mut command = Command::new("docker");
    command
        .arg("run")
        .arg("--rm")
        .arg("--name")
        .arg(&container_name)
        .arg("--network")
        .arg(&manifest.docker.network)
        .arg("--read-only")
        .arg("--cap-drop")
        .arg("ALL");

    for cap in &manifest.docker.cap_add {
        command.arg("--cap-add").arg(cap);
    }

    command
        .arg("--security-opt")
        .arg("no-new-privileges")
        .arg("--pids-limit")
        .arg(manifest.docker.pids_limit.to_string())
        .arg("--memory")
        .arg(&manifest.docker.memory)
        .arg("--cpus")
        .arg(&manifest.docker.cpus)
        .arg("--user")
        .arg(container_user(&manifest.docker));

    for tmpfs in &manifest.docker.tmpfs {
        command.arg("--tmpfs").arg(tmpfs);
    }

    // Pass only the bare variable name, never `NAME=value`: Docker then reads
    // the actual value from this process's own (inherited) environment
    // rather than from argv, so a configured secret never appears in `ps`
    // output, this process's own argv, or anywhere it would get logged.
    // Required secrets are already confirmed present above; an optional one
    // is forwarded only if actually configured.
    for secret in &manifest.secrets {
        if env::var_os(&secret.env_var).is_some() {
            command.arg("--env").arg(&secret.env_var);
        }
    }

    command
        .arg("--volume")
        .arg(format!("{}:/artifacts:rw", artifact_dir.display()));
    if let Some(input_dir) = &input_dir {
        let mode = if manifest.docker.input_read_only {
            "ro"
        } else {
            "rw"
        };
        command
            .arg("--volume")
            .arg(format!("{}:/input:{mode}", input_dir.display()));
    }
    command
        .arg(&manifest.container)
        .args(container_args)
        .stdin(Stdio::null());

    let max_output_bytes = max_plugin_output_bytes();
    let output = match time::timeout(
        Duration::from_secs(manifest.docker.timeout_seconds),
        run_with_bounded_output(command, max_output_bytes),
    )
    .await
    {
        Ok(result) => result.context("failed to run docker")?,
        Err(_) => {
            // The `docker run` client is still attached and the container is still
            // running under dockerd; dropping our awaited future only abandons the
            // client, it does not stop the container. Stop it by its deterministic
            // name directly so a slow/hung tool doesn't leak a running container.
            // Bounded and its result actually checked - an unbounded kill would
            // hang this whole process indefinitely if dockerd itself were stuck,
            // defeating the point of having a timeout at all; a silently ignored
            // failure would leave a container running with nothing in the log to
            // explain why (see SECURITY_REVIEW_2026-10-20.md's R6).
            match time::timeout(
                Duration::from_secs(10),
                Command::new("docker")
                    .arg("kill")
                    .arg(&container_name)
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::piped())
                    .output(),
            )
            .await
            {
                Ok(Ok(output)) if !output.status.success() => {
                    eprintln!(
                        "slickchook runner: docker kill for timed-out container {container_name} \
                         did not succeed: {}",
                        String::from_utf8_lossy(&output.stderr)
                    );
                }
                Ok(Err(error)) => {
                    eprintln!(
                        "slickchook runner: failed to run docker kill for timed-out container \
                         {container_name}: {error}"
                    );
                }
                Err(_) => {
                    eprintln!(
                        "slickchook runner: docker kill for timed-out container \
                         {container_name} itself timed out"
                    );
                }
                Ok(Ok(_)) => {}
            }
            fixup_artifact_permissions(&artifact_dir, &manifest.docker).await;
            bail!(
                "plugin `{}` exceeded timeout of {} seconds",
                manifest.name,
                manifest.docker.timeout_seconds
            );
        }
    };

    // Plugin containers run as UID 65534 (or UID 0, for the few plugins
    // whose real functionality requires root - see DockerPolicy::run_as_root),
    // and some tools (e.g. testssl.sh) restrict permissions on their own
    // output files as they write them, leaving them unreadable by the
    // slickchook service's own user afterward. Fix up permissions as the
    // same UID that owns the files (only their owner, or root, may relax
    // them), immediately after the container exits, before anything tries
    // to read them back.
    fixup_artifact_permissions(&artifact_dir, &manifest.docker).await;

    let stdout = if output.stdout_truncated {
        mark_truncated(output.stdout, max_output_bytes)
    } else {
        output.stdout
    };
    let stderr = if output.stderr_truncated {
        mark_truncated(output.stderr, max_output_bytes)
    } else {
        output.stderr
    };
    io::stdout()
        .write_all(&stdout)
        .context("failed to write runner stdout")?;
    io::stderr()
        .write_all(&stderr)
        .context("failed to write runner stderr")?;

    if output.status.success() {
        Ok(())
    } else {
        std::process::exit(output.status.code().unwrap_or(1));
    }
}

/// Some plugin tools (e.g. testssl.sh) restrict permissions on their own
/// output files as they write them, leaving them unreadable by the
/// slickchook service's own user afterward. Only a file's owner (or root)
/// may relax its permissions, so this runs a trivial follow-up container as
/// the same UID the plugin container ran as, making everything it wrote
/// readable. Best-effort: failures are logged, not propagated, since the
/// plugin's own result matters more than this cleanup step.
async fn fixup_artifact_permissions(artifact_dir: &Path, docker: &DockerPolicy) {
    let mut command = Command::new("docker");
    command
        .arg("run")
        .arg("--rm")
        .arg("--network")
        .arg("none")
        .arg("--read-only")
        .arg("--cap-drop")
        .arg("ALL")
        .arg("--security-opt")
        .arg("no-new-privileges")
        .arg("--user")
        .arg(container_user(docker))
        .arg("--volume")
        .arg(format!("{}:/artifacts:rw", artifact_dir.display()))
        .arg("alpine:3.21")
        .arg("find")
        .arg("/artifacts")
        .arg("-mindepth")
        .arg("1")
        // `/artifacts/input` itself is always created host-side (by the
        // slickchook service, not the plugin container) before the container
        // even starts, whether or not this run actually uses input chaining
        // - the slickchook service already owns that directory entry and can
        // read it fine on its own, so only the bare directory path is
        // excluded here rather than producing a permission error on every
        // single run trying to chmod a directory this container was never
        // going to own. Its *contents* are NOT excluded: a fetcher plugin
        // (docker.input_read_only = false, e.g. webfetch, gowitness) writes
        // real files there as UID 65534, and those still need the same
        // host-readability fixup as any other plugin output, or a later
        // read_artifact call on them fails outright.
        .arg("!")
        .arg("-path")
        .arg("/artifacts/input")
        .arg("-exec")
        .arg("chmod")
        .arg("a+rX")
        .arg("{}")
        .arg("+")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());

    match time::timeout(Duration::from_secs(30), command.output()).await {
        Ok(Ok(output)) if !output.status.success() => {
            eprintln!(
                "slickchook runner: artifact permission fixup failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Ok(Err(error)) => {
            eprintln!("slickchook runner: failed to run artifact permission fixup: {error}");
        }
        Err(_) => {
            eprintln!("slickchook runner: artifact permission fixup timed out");
        }
        Ok(Ok(_)) => {}
    }
}

fn parse_args() -> Result<RunnerArgs> {
    let mut manifest = None;
    let mut plugins_root = None;
    let mut artifact_dir = None;
    let mut artifacts_root = None;
    let mut input_json = None;
    let mut input_dir = None;

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--manifest" => manifest = Some(next_arg(&mut args, "--manifest")?.into()),
            "--plugins-root" => plugins_root = Some(next_arg(&mut args, "--plugins-root")?.into()),
            "--artifact-dir" => artifact_dir = Some(next_arg(&mut args, "--artifact-dir")?.into()),
            "--artifacts-root" => {
                artifacts_root = Some(next_arg(&mut args, "--artifacts-root")?.into())
            }
            "--input-json" => input_json = Some(next_arg(&mut args, "--input-json")?),
            "--input-dir" => input_dir = Some(next_arg(&mut args, "--input-dir")?.into()),
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            "--version" | "-V" => {
                println!("slickchook-runner {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            _ => bail!("unknown argument `{arg}`"),
        }
    }

    Ok(RunnerArgs {
        manifest: manifest.ok_or_else(|| anyhow!("missing --manifest"))?,
        plugins_root: plugins_root.ok_or_else(|| anyhow!("missing --plugins-root"))?,
        artifact_dir: artifact_dir.ok_or_else(|| anyhow!("missing --artifact-dir"))?,
        artifacts_root: artifacts_root.ok_or_else(|| anyhow!("missing --artifacts-root"))?,
        input_json: input_json.ok_or_else(|| anyhow!("missing --input-json"))?,
        input_dir,
    })
}

fn next_arg(args: &mut impl Iterator<Item = String>, name: &str) -> Result<String> {
    args.next()
        .ok_or_else(|| anyhow!("missing value for {name}"))
}

fn print_usage() {
    eprintln!(
        "Usage: slickchook-runner --manifest <path> --plugins-root <path> --artifact-dir <path> --artifacts-root <path> --input-json <json>"
    );
}

fn validate_runner_paths(args: &RunnerArgs) -> Result<()> {
    let manifest = canonicalize_file(&args.manifest)?;
    let plugins_root = canonicalize_dir(&args.plugins_root)?;
    if !manifest.starts_with(&plugins_root) {
        bail!(
            "manifest {} is outside plugin root {}",
            manifest.display(),
            plugins_root.display()
        );
    }

    let artifact_dir = canonicalize_dir(&args.artifact_dir)?;
    let artifacts_root = canonicalize_dir(&args.artifacts_root)?;
    if !artifact_dir.starts_with(&artifacts_root) {
        bail!(
            "artifact dir {} is outside artifact root {}",
            artifact_dir.display(),
            artifacts_root.display()
        );
    }

    Ok(())
}

fn canonicalize_file(path: &Path) -> Result<PathBuf> {
    let canonical = fs::canonicalize(path)
        .with_context(|| format!("failed to canonicalize {}", path.display()))?;
    if !canonical.is_file() {
        bail!("{} is not a file", canonical.display());
    }
    Ok(canonical)
}

fn canonicalize_dir(path: &Path) -> Result<PathBuf> {
    let canonical = fs::canonicalize(path)
        .with_context(|| format!("failed to canonicalize {}", path.display()))?;
    if !canonical.is_dir() {
        bail!("{} is not a directory", canonical.display());
    }
    Ok(canonical)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_valid_container_name_to_kill_matches_only_the_real_shape() {
        assert!(is_valid_container_name_to_kill(
            "slickchook-271c58a4-db19-4285-bbca-e2d8804c9904"
        ));
        assert!(!is_valid_container_name_to_kill("slickchook-"));
        assert!(!is_valid_container_name_to_kill("not-slickchook-abc123"));
        assert!(!is_valid_container_name_to_kill(
            "slickchook-abc; rm -rf /"
        ));
        assert!(!is_valid_container_name_to_kill("other-container"));
        assert!(!is_valid_container_name_to_kill(""));
    }

    #[tokio::test]
    async fn read_capped_keeps_everything_under_the_cap() {
        let data = b"hello world".to_vec();
        let (bytes, truncated) = read_capped(data.as_slice(), 1024).await.unwrap();
        assert_eq!(bytes, data);
        assert!(!truncated);
    }

    #[tokio::test]
    async fn read_capped_truncates_and_still_drains_the_stream() {
        let data = vec![b'x'; 10_000];
        let (bytes, truncated) = read_capped(data.as_slice(), 100).await.unwrap();
        assert_eq!(bytes.len(), 100);
        assert!(bytes.iter().all(|&b| b == b'x'));
        assert!(truncated);
    }

    #[tokio::test]
    async fn read_capped_exact_boundary_is_not_truncated() {
        let data = vec![b'y'; 256];
        let (bytes, truncated) = read_capped(data.as_slice(), 256).await.unwrap();
        assert_eq!(bytes.len(), 256);
        assert!(!truncated);
    }

    #[test]
    fn mark_truncated_appends_a_visible_marker_without_hiding_the_original_bytes() {
        let original = b"partial output".to_vec();
        let marked = mark_truncated(original.clone(), 42);
        assert!(marked.starts_with(&original));
        let marker = String::from_utf8(marked[original.len()..].to_vec()).unwrap();
        assert!(marker.contains("truncated"));
        assert!(marker.contains("42"));
    }

    #[test]
    fn max_plugin_output_bytes_falls_back_to_default_on_invalid_env() {
        // SAFETY: this test's own env var, not read by any other test
        // concurrently - `cargo test` runs each test in its own thread, but
        // env vars are process-global, so a name no other test touches
        // avoids cross-test interference regardless.
        unsafe {
            env::set_var("SLICKCHOOK_MAX_PLUGIN_OUTPUT_BYTES", "not-a-number");
        }
        assert_eq!(max_plugin_output_bytes(), DEFAULT_MAX_PLUGIN_OUTPUT_BYTES);
        unsafe {
            env::set_var("SLICKCHOOK_MAX_PLUGIN_OUTPUT_BYTES", "12345");
        }
        assert_eq!(max_plugin_output_bytes(), 12345);
        unsafe {
            env::remove_var("SLICKCHOOK_MAX_PLUGIN_OUTPUT_BYTES");
        }
    }
}
