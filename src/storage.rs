use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio_postgres::{Client, NoTls, config::SslMode};

/// Whether to require TLS on the connection to PostgreSQL. This is an explicit,
/// authoritative setting rather than something inferred from `DATABASE_URL`'s
/// own `sslmode` query parameter (if any): the parsed `Config`'s `ssl_mode` is
/// always overridden to match, so a misconfigured or missing `sslmode` in the
/// connection string can't silently downgrade an operator's intent either way.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseTls {
    /// No TLS. Suitable only when PostgreSQL is reachable exclusively over a
    /// trusted loopback or local socket, which is the default deployment model
    /// documented in RUNNER.md.
    #[default]
    Disable,
    /// Require TLS, verifying the server certificate against the host's native
    /// trust store. The connection fails outright if the server doesn't (or
    /// can't) negotiate TLS -- there is no "opportunistic"/unverified middle
    /// tier, since that would silently tolerate a stripped-TLS
    /// machine-in-the-middle without ever surfacing an error.
    Require,
}

/// Installs rustls's `ring` crypto provider as the process default, required
/// before constructing any `rustls::ClientConfig`. Safe to call more than
/// once; only the first call has any effect.
fn ensure_crypto_provider_installed() {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

fn build_rustls_connector() -> Result<tokio_postgres_rustls::MakeRustlsConnect> {
    ensure_crypto_provider_installed();
    let loaded = rustls_native_certs::load_native_certs();
    for error in &loaded.errors {
        tracing::warn!(%error, "error loading a native root certificate for PostgreSQL TLS");
    }
    let mut roots = rustls::RootCertStore::empty();
    let (added, ignored) = roots.add_parsable_certificates(loaded.certs);
    tracing::debug!(
        added,
        ignored,
        "loaded native root certificates for PostgreSQL TLS"
    );
    if added == 0 {
        bail!("no usable root certificates found; cannot verify a TLS PostgreSQL connection");
    }
    let config = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    Ok(tokio_postgres_rustls::MakeRustlsConnect::new(config))
}

fn parse_config(database_url: &str, tls: DatabaseTls) -> Result<tokio_postgres::config::Config> {
    let mut config: tokio_postgres::config::Config = database_url
        .parse()
        .context("failed to parse DATABASE_URL")?;
    config.ssl_mode(match tls {
        DatabaseTls::Disable => SslMode::Disable,
        DatabaseTls::Require => SslMode::Require,
    });
    Ok(config)
}

async fn log_connection_errors<S, T>(connection: tokio_postgres::Connection<S, T>)
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
    T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    if let Err(error) = connection.await {
        tracing::error!(%error, "postgresql connection closed");
    }
}

pub struct StoredRun<'a> {
    pub run_id: &'a str,
    pub plugin: &'a str,
    pub container: &'a str,
    pub target: Option<&'a str>,
    pub status: &'a str,
    pub exit_code: Option<i32>,
    pub artifact_dir: &'a Path,
    pub stdout_path: &'a Path,
    pub stderr_path: &'a Path,
    pub input_json: &'a str,
    pub started_at: &'a str,
    pub finished_at: &'a str,
}

#[derive(Debug, Serialize)]
pub struct RunSummary {
    pub run_id: String,
    pub plugin: String,
    pub container: String,
    pub target: Option<String>,
    pub status: String,
    pub exit_code: Option<i32>,
    pub artifact_dir: String,
    pub started_at: String,
    pub finished_at: String,
}

impl RunSummary {
    fn from_row(row: &tokio_postgres::Row) -> Self {
        Self {
            run_id: row.get(0),
            plugin: row.get(1),
            container: row.get(2),
            target: row.get(3),
            status: row.get(4),
            exit_code: row.get(5),
            artifact_dir: row.get(6),
            started_at: row.get(7),
            finished_at: row.get(8),
        }
    }
}

const RUN_SUMMARY_COLUMNS: &str =
    "run_id, plugin, container, target, status, exit_code, artifact_dir, started_at, finished_at";

/// Canonicalizes `candidate` and confirms it lies beneath the already-canonical
/// `root`, resolving symlinks along the way so a symlink planted inside a run's
/// artifact directory can't point an otherwise-valid-looking path outside it.
fn canonical_contained(root: &Path, candidate: impl AsRef<Path>) -> Result<PathBuf> {
    let resolved = std::fs::canonicalize(candidate)?;
    if !resolved.starts_with(root) {
        bail!("path is outside the expected root");
    }
    Ok(resolved)
}

/// PostgreSQL-backed run store. Construction is asynchronous because tokio-postgres
/// establishes the connection immediately; the connection driver is kept alive in a
/// background task.
pub struct PostgresStore {
    client: Client,
    pub artifacts_root: std::path::PathBuf,
}

impl PostgresStore {
    pub async fn connect(
        database_url: &str,
        artifacts_root: std::path::PathBuf,
        tls: DatabaseTls,
    ) -> Result<Self> {
        let config = parse_config(database_url, tls)?;
        let client = match tls {
            DatabaseTls::Disable => {
                let (client, connection) = config.connect(NoTls).await?;
                tokio::spawn(log_connection_errors(connection));
                client
            }
            DatabaseTls::Require => {
                let connector = build_rustls_connector()?;
                let (client, connection) = config.connect(connector).await?;
                tokio::spawn(log_connection_errors(connection));
                client
            }
        };
        Ok(Self {
            client,
            artifacts_root,
        })
    }

    /// Connects using `handle` for both the initial handshake and the
    /// long-lived connection-driving task, instead of owning a private
    /// runtime. A private runtime would outlive this call and later get
    /// dropped from inside `handle`'s own async context (e.g. on shutdown),
    /// which panics: Tokio forbids dropping a runtime from within another
    /// runtime's async task.
    pub fn connect_sync(
        handle: &tokio::runtime::Handle,
        database_url: &str,
        artifacts_root: std::path::PathBuf,
        tls: DatabaseTls,
    ) -> Result<Self> {
        let config = parse_config(database_url, tls)?;
        let client = match tls {
            DatabaseTls::Disable => {
                let (client, connection) = handle.block_on(config.connect(NoTls))?;
                handle.spawn(log_connection_errors(connection));
                client
            }
            DatabaseTls::Require => {
                let connector = build_rustls_connector()?;
                let (client, connection) = handle.block_on(config.connect(connector))?;
                handle.spawn(log_connection_errors(connection));
                client
            }
        };
        Ok(Self {
            client,
            artifacts_root,
        })
    }

    pub async fn init_schema(&self) -> Result<()> {
        self.client
            .batch_execute(include_str!("../migrations/postgres/001_initial.sql"))
            .await?;
        self.client
            .batch_execute(include_str!(
                "../migrations/postgres/002_add_run_target.sql"
            ))
            .await?;
        Ok(())
    }

    pub async fn resolve_completed_input(&self, run_id: &str) -> Result<std::path::PathBuf> {
        let row = self.client.query_opt(
            "SELECT artifact_dir FROM plugin_runs WHERE run_id = $1 AND status IN ('success','failed')",
            &[&run_id],
        ).await?.ok_or_else(|| anyhow::anyhow!("completed run not found: {run_id}"))?;
        let dir: String = row.get(0);
        let root = std::fs::canonicalize(&self.artifacts_root)?;
        let run = canonical_contained(&root, dir)?;
        let input = canonical_contained(&run, run.join("input"))?;
        Ok(input)
    }

    /// Resolves the artifact directory for any recorded run (regardless of
    /// status), confined to `artifacts_root`. Unlike `resolve_completed_input`,
    /// this doesn't filter by status: `record_run` only ever writes a row once a
    /// run has already finished (successfully, unsuccessfully, or with a runner
    /// error), so every row here already describes a completed attempt.
    pub async fn resolve_run_artifact_dir(&self, run_id: &str) -> Result<std::path::PathBuf> {
        let row = self
            .client
            .query_opt(
                "SELECT artifact_dir FROM plugin_runs WHERE run_id = $1",
                &[&run_id],
            )
            .await?
            .ok_or_else(|| anyhow::anyhow!("unknown run: {run_id}"))?;
        let dir: String = row.get(0);
        let root = std::fs::canonicalize(&self.artifacts_root)?;
        canonical_contained(&root, dir)
    }

    pub async fn list_runs_by_target(&self, target: &str, limit: i64) -> Result<Vec<RunSummary>> {
        let rows = self
            .client
            .query(
                &format!(
                    "SELECT {RUN_SUMMARY_COLUMNS} FROM plugin_runs WHERE target = $1 ORDER BY created_at DESC LIMIT $2"
                ),
                &[&target, &limit],
            )
            .await?;
        Ok(rows.iter().map(RunSummary::from_row).collect())
    }

    pub async fn record_run(&self, run: &StoredRun<'_>) -> Result<()> {
        let artifact_dir = run.artifact_dir.to_string_lossy().into_owned();
        let stdout_path = run.stdout_path.to_string_lossy().into_owned();
        let stderr_path = run.stderr_path.to_string_lossy().into_owned();
        let input_json: serde_json::Value = serde_json::from_str(run.input_json)?;
        self.client.execute(
            "INSERT INTO plugin_runs (run_id, plugin, container, target, input_json, status, exit_code, artifact_dir, stdout_artifact, stderr_artifact, started_at, finished_at) VALUES ($1,$2,$3,$4,$5::jsonb,$6,$7,$8,$9,$10,$11,$12) ON CONFLICT (run_id) DO UPDATE SET plugin=EXCLUDED.plugin, container=EXCLUDED.container, target=EXCLUDED.target, input_json=EXCLUDED.input_json, status=EXCLUDED.status, exit_code=EXCLUDED.exit_code, artifact_dir=EXCLUDED.artifact_dir, stdout_artifact=EXCLUDED.stdout_artifact, stderr_artifact=EXCLUDED.stderr_artifact, started_at=EXCLUDED.started_at, finished_at=EXCLUDED.finished_at",
            &[&run.run_id, &run.plugin, &run.container, &run.target, &input_json, &run.status, &run.exit_code, &artifact_dir, &stdout_path, &stderr_path, &run.started_at, &run.finished_at],
        ).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn postgres_store_round_trip_when_configured() {
        let Ok(url) = std::env::var("TEST_POSTGRES_URL") else {
            return;
        };
        let root = std::env::temp_dir().join(format!("slickchook-pg-{}", uuid::Uuid::new_v4()));
        let run = root.join("run");
        let outside =
            std::env::temp_dir().join(format!("slickchook-pg-outside-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(run.join("input")).unwrap();
        std::fs::create_dir_all(outside.join("input")).unwrap();
        let store = super::PostgresStore::connect(&url, root.clone(), super::DatabaseTls::Disable)
            .await
            .unwrap();
        store.init_schema().await.unwrap();
        let run_id = format!("test-{}", uuid::Uuid::new_v4());
        let target = format!("target-{}.example", uuid::Uuid::new_v4());
        store
            .record_run(&super::StoredRun {
                run_id: &run_id,
                plugin: "test",
                container: "test:latest",
                target: Some(&target),
                status: "success",
                exit_code: Some(0),
                artifact_dir: &run,
                stdout_path: &run.join("stdout"),
                stderr_path: &run.join("stderr"),
                input_json: "{}",
                started_at: "2026-01-01T00:00:00Z",
                finished_at: "2026-01-01T00:00:01Z",
            })
            .await
            .unwrap();
        assert_eq!(
            store.resolve_completed_input(&run_id).await.unwrap(),
            std::fs::canonicalize(run.join("input")).unwrap()
        );
        assert!(
            store
                .resolve_completed_input("does-not-exist")
                .await
                .is_err()
        );

        let by_target = store.list_runs_by_target(&target, 10).await.unwrap();
        assert_eq!(by_target.len(), 1);
        assert_eq!(by_target[0].run_id, run_id);
        assert_eq!(by_target[0].target.as_deref(), Some(target.as_str()));
        assert!(
            store
                .list_runs_by_target("no-such-target.example", 10)
                .await
                .unwrap()
                .is_empty()
        );

        assert_eq!(
            store.resolve_run_artifact_dir(&run_id).await.unwrap(),
            std::fs::canonicalize(&run).unwrap()
        );
        assert!(
            store
                .resolve_run_artifact_dir("does-not-exist")
                .await
                .is_err()
        );

        let outside_id = format!("outside-{}", uuid::Uuid::new_v4());
        store
            .record_run(&super::StoredRun {
                run_id: &outside_id,
                plugin: "test",
                container: "test:latest",
                target: None,
                status: "success",
                exit_code: Some(0),
                artifact_dir: &outside,
                stdout_path: &outside.join("stdout"),
                stderr_path: &outside.join("stderr"),
                input_json: "{}",
                started_at: "now",
                finished_at: "now",
            })
            .await
            .unwrap();
        assert!(store.resolve_completed_input(&outside_id).await.is_err());

        let reopened =
            super::PostgresStore::connect(&url, root.clone(), super::DatabaseTls::Disable)
                .await
                .unwrap();
        assert!(reopened.resolve_completed_input(&run_id).await.is_ok());

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let symlink_run = root.join("symlink-run");
            std::fs::create_dir_all(&symlink_run).unwrap();
            symlink(&outside, symlink_run.join("input")).unwrap();
            let symlink_id = format!("symlink-{}", uuid::Uuid::new_v4());
            store
                .record_run(&super::StoredRun {
                    run_id: &symlink_id,
                    plugin: "test",
                    container: "test:latest",
                    target: None,
                    status: "success",
                    exit_code: Some(0),
                    artifact_dir: &symlink_run,
                    stdout_path: &symlink_run.join("stdout"),
                    stderr_path: &symlink_run.join("stderr"),
                    input_json: "{}",
                    started_at: "now",
                    finished_at: "now",
                })
                .await
                .unwrap();
            assert!(store.resolve_completed_input(&symlink_id).await.is_err());
        }
        let _ = std::fs::remove_dir_all(root);
        let _ = std::fs::remove_dir_all(outside);
    }

    /// Requires a PostgreSQL instance with TLS enabled (`ssl = on`):
    /// `TEST_POSTGRES_TLS_URL` its connection string, `TEST_POSTGRES_TLS_CA`
    /// a PEM file containing (only) its server certificate, and
    /// `TEST_POSTGRES_TLS_UNRELATED_CA` a PEM cert unrelated to that server.
    ///
    /// Both directions are asserted sequentially in one test (rather than as
    /// two separate `#[tokio::test]`s) because both mutate the
    /// process-global `SSL_CERT_FILE` env var -- `rustls-native-certs` (used
    /// by the real `build_rustls_connector`) loads trust roots from it
    /// instead of the OS store whenever it's set. Two separate tests would
    /// race under the default parallel test runner, each capable of
    /// clobbering the other's `SSL_CERT_FILE` mid-connection.
    ///
    /// First connects with `DatabaseTls::Require` pointed at the unrelated
    /// cert and confirms it's rejected, proving TLS is actually attempted
    /// and verified, not silently skipped. Then points `SSL_CERT_FILE` at
    /// the real server certificate and confirms a fully verified connection
    /// succeeds and is actually usable (not just that the handshake
    /// completes) -- exercising the exact production TLS path end to end.
    #[tokio::test]
    async fn postgres_store_tls_require_verifies_certificate() {
        let (Ok(url), Ok(ca_path), Ok(unrelated_ca_path)) = (
            std::env::var("TEST_POSTGRES_TLS_URL"),
            std::env::var("TEST_POSTGRES_TLS_CA"),
            std::env::var("TEST_POSTGRES_TLS_UNRELATED_CA"),
        ) else {
            return;
        };

        // SAFETY: no other test in this crate reads or writes SSL_CERT_FILE.
        unsafe {
            std::env::set_var("SSL_CERT_FILE", &unrelated_ca_path);
        }
        let untrusted_root =
            std::env::temp_dir().join(format!("slickchook-tls-{}", uuid::Uuid::new_v4()));
        let error =
            super::PostgresStore::connect(&url, untrusted_root, super::DatabaseTls::Require)
                .await
                .err()
                .unwrap();
        let message = format!("{error:#}");
        assert!(
            message.to_lowercase().contains("certificate")
                || message.to_lowercase().contains("unknownissuer")
                || message.to_lowercase().contains("invalid peer certificate"),
            "expected a certificate-verification failure, got: {message}"
        );

        // SAFETY: no other test in this crate reads or writes SSL_CERT_FILE.
        unsafe {
            std::env::set_var("SSL_CERT_FILE", &ca_path);
        }
        let trusted_root =
            std::env::temp_dir().join(format!("slickchook-tls-{}", uuid::Uuid::new_v4()));
        let store = super::PostgresStore::connect(&url, trusted_root, super::DatabaseTls::Require)
            .await
            .unwrap();
        store.init_schema().await.unwrap();
        assert!(
            store
                .list_runs_by_target("no-such-target-tls.example", 10)
                .await
                .unwrap()
                .is_empty()
        );
    }
}
