use anyhow::{Context, Result, anyhow, bail};
use serde_json::Value;
use slickchook::plugin_manager::{build_container_args, load_manifest, validate_docker_policy};
use std::{
    env, fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};
use tokio::{process::Command, time};

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
    if let Err(error) = run().await {
        eprintln!("{error:#}");
        std::process::exit(1);
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

    let input: Value =
        serde_json::from_str(&args.input_json).context("failed to parse plugin input JSON")?;
    let container_args = build_container_args(&manifest, &input)?;
    eprintln!(
        "slickchook runner container={} args={:?}",
        manifest.container, container_args
    );
    tracing::info!(container = %manifest.container, args = ?container_args, "launching plugin container");

    let mut command = Command::new("docker");
    command
        .arg("run")
        .arg("--rm")
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
        .arg("65534:65534");

    for tmpfs in &manifest.docker.tmpfs {
        command.arg("--tmpfs").arg(tmpfs);
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
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let output = time::timeout(
        Duration::from_secs(manifest.docker.timeout_seconds),
        command.output(),
    )
    .await
    .with_context(|| {
        format!(
            "plugin `{}` exceeded timeout of {} seconds",
            manifest.name, manifest.docker.timeout_seconds
        )
    })?
    .context("failed to run docker")?;

    io::stdout()
        .write_all(&output.stdout)
        .context("failed to write runner stdout")?;
    io::stderr()
        .write_all(&output.stderr)
        .context("failed to write runner stderr")?;

    if output.status.success() {
        Ok(())
    } else {
        std::process::exit(output.status.code().unwrap_or(1));
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
