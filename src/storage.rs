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

/// Establishes one client connection. Used for both the initial connect and
/// later reconnects; `connect_sync` drives this synchronously itself via
/// `handle.block_on(make_client(..))` rather than this function blocking.
async fn make_client(database_url: &str, tls: DatabaseTls) -> Result<Client> {
    let config = parse_config(database_url, tls)?;
    match tls {
        DatabaseTls::Disable => {
            let (client, connection) = config.connect(NoTls).await?;
            tokio::spawn(log_connection_errors(connection));
            Ok(client)
        }
        DatabaseTls::Require => {
            let connector = build_rustls_connector()?;
            let (client, connection) = config.connect(connector).await?;
            tokio::spawn(log_connection_errors(connection));
            Ok(client)
        }
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
    /// The exact JSON input a run was called with. Lets a caller checking
    /// `list_runs_by_target` tell whether a past run actually used the
    /// options it's about to use (worth reusing) or different ones (worth
    /// re-running), without an extra `read_artifact` call to parse the
    /// stderr's `args=[...]` log line just to find out.
    pub input_json: serde_json::Value,
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
            input_json: row.get(9),
        }
    }
}

const RUN_SUMMARY_COLUMNS: &str = "run_id, plugin, container, target, status, exit_code, artifact_dir, started_at, finished_at, input_json";

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

/// Outcome of `PostgresStore::delete_runs_by_target`: which runs were fully
/// removed (database row and on-disk artifact directory both gone) and which
/// hit an error partway through (row still deleted - see below - but the
/// directory removal failed for some other reason, e.g. a permissions issue).
#[derive(Debug, Serialize)]
pub struct PurgeSummary {
    pub target: String,
    pub deleted_run_ids: Vec<String>,
    pub errors: Vec<String>,
}

/// PostgreSQL-backed run store. Construction is asynchronous because tokio-postgres
/// establishes the connection immediately; the connection driver is kept alive in a
/// background task.
///
/// A `tokio_postgres::Client` never recovers on its own once its connection
/// dies (network blip, PostgreSQL restart, idle timeout) -- every subsequent
/// query fails with `Error::is_closed() == true` forever after, even though
/// the rest of the process keeps running fine. `client` is therefore held
/// behind a lock so it can be replaced in place: each query method retries
/// once after calling `reconnect()`, but only when the failure is
/// specifically a closed connection, not any other query error.
pub struct PostgresStore {
    client: tokio::sync::RwLock<Client>,
    database_url: String,
    tls: DatabaseTls,
    pub artifacts_root: std::path::PathBuf,
}

impl PostgresStore {
    pub async fn connect(
        database_url: &str,
        artifacts_root: std::path::PathBuf,
        tls: DatabaseTls,
    ) -> Result<Self> {
        let client = make_client(database_url, tls).await?;
        Ok(Self {
            client: tokio::sync::RwLock::new(client),
            database_url: database_url.to_string(),
            tls,
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
        let client = handle.block_on(make_client(database_url, tls))?;
        Ok(Self {
            client: tokio::sync::RwLock::new(client),
            database_url: database_url.to_string(),
            tls,
            artifacts_root,
        })
    }

    /// Replaces the held client with a freshly connected one, using the same
    /// `database_url`/`tls` this store was constructed with. Called only
    /// after a query fails with `Error::is_closed()`, and serialized by
    /// `client`'s write lock: concurrent callers that all observe the same
    /// dead connection queue up here rather than each reconnecting
    /// independently.
    async fn reconnect(&self) -> Result<()> {
        tracing::warn!("postgresql connection was closed; reconnecting");
        let fresh = make_client(&self.database_url, self.tls).await?;
        *self.client.write().await = fresh;
        Ok(())
    }

    pub async fn init_schema(&self) -> Result<()> {
        for migration in [
            include_str!("../migrations/postgres/001_initial.sql"),
            include_str!("../migrations/postgres/002_add_run_target.sql"),
        ] {
            let result = self.client.read().await.batch_execute(migration).await;
            match result {
                Ok(()) => {}
                Err(error) if error.is_closed() => {
                    self.reconnect().await?;
                    self.client.read().await.batch_execute(migration).await?;
                }
                Err(error) => return Err(error.into()),
            }
        }
        Ok(())
    }

    pub async fn resolve_completed_input(&self, run_id: &str) -> Result<std::path::PathBuf> {
        const SQL: &str = "SELECT artifact_dir FROM plugin_runs WHERE run_id = $1 AND status IN ('success','failed')";
        let params: &[&(dyn tokio_postgres::types::ToSql + Sync)] = &[&run_id];
        let result = self.client.read().await.query_opt(SQL, params).await;
        let row = match result {
            Ok(row) => row,
            Err(error) if error.is_closed() => {
                self.reconnect().await?;
                self.client.read().await.query_opt(SQL, params).await?
            }
            Err(error) => return Err(error.into()),
        }
        .ok_or_else(|| anyhow::anyhow!("completed run not found: {run_id}"))?;
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
        const SQL: &str = "SELECT artifact_dir FROM plugin_runs WHERE run_id = $1";
        let params: &[&(dyn tokio_postgres::types::ToSql + Sync)] = &[&run_id];
        let result = self.client.read().await.query_opt(SQL, params).await;
        let row = match result {
            Ok(row) => row,
            Err(error) if error.is_closed() => {
                self.reconnect().await?;
                self.client.read().await.query_opt(SQL, params).await?
            }
            Err(error) => return Err(error.into()),
        }
        .ok_or_else(|| anyhow::anyhow!("unknown run: {run_id}"))?;
        let dir: String = row.get(0);
        let root = std::fs::canonicalize(&self.artifacts_root)?;
        canonical_contained(&root, dir)
    }

    /// Permanently deletes every recorded run whose target matches exactly,
    /// removing both the database row and its on-disk artifact directory.
    /// Irreversible - callers must only invoke this after explicit human
    /// confirmation (see the `purge_runs_by_target` MCP tool's description),
    /// never automatically or silently.
    ///
    /// Deletes the database row first, then removes the directory: a row
    /// whose directory removal fails still counts as deleted (it's gone from
    /// `list_runs_by_target` either way), but is reported in `errors` so the
    /// caller can flag the leftover directory rather than silently losing
    /// track of it.
    pub async fn delete_runs_by_target(&self, target: &str) -> Result<PurgeSummary> {
        const SQL: &str = "DELETE FROM plugin_runs WHERE target = $1 RETURNING run_id, artifact_dir";
        let params: &[&(dyn tokio_postgres::types::ToSql + Sync)] = &[&target];
        let result = self.client.read().await.query(SQL, params).await;
        let rows = match result {
            Ok(rows) => rows,
            Err(error) if error.is_closed() => {
                self.reconnect().await?;
                self.client.read().await.query(SQL, params).await?
            }
            Err(error) => return Err(error.into()),
        };
        let root = std::fs::canonicalize(&self.artifacts_root)?;
        let mut deleted_run_ids = Vec::new();
        let mut errors = Vec::new();
        for row in rows {
            let run_id: String = row.get(0);
            let dir: String = row.get(1);
            match canonical_contained(&root, &dir) {
                Ok(path) => match std::fs::remove_dir_all(&path) {
                    Ok(()) => deleted_run_ids.push(run_id),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                        deleted_run_ids.push(run_id);
                    }
                    Err(error) => errors.push(format!(
                        "{run_id}: failed to remove {}: {error}",
                        path.display()
                    )),
                },
                Err(error) => {
                    deleted_run_ids.push(run_id.clone());
                    errors.push(format!("{run_id}: {error}"));
                }
            }
        }
        Ok(PurgeSummary {
            target: target.to_string(),
            deleted_run_ids,
            errors,
        })
    }

    pub async fn list_runs_by_target(&self, target: &str, limit: i64) -> Result<Vec<RunSummary>> {
        let sql = format!(
            "SELECT {RUN_SUMMARY_COLUMNS} FROM plugin_runs WHERE target = $1 ORDER BY created_at DESC LIMIT $2"
        );
        let params: &[&(dyn tokio_postgres::types::ToSql + Sync)] = &[&target, &limit];
        let result = self.client.read().await.query(&sql, params).await;
        let rows = match result {
            Ok(rows) => rows,
            Err(error) if error.is_closed() => {
                self.reconnect().await?;
                self.client.read().await.query(&sql, params).await?
            }
            Err(error) => return Err(error.into()),
        };
        Ok(rows.iter().map(RunSummary::from_row).collect())
    }

    pub async fn record_run(&self, run: &StoredRun<'_>) -> Result<()> {
        const SQL: &str = "INSERT INTO plugin_runs (run_id, plugin, container, target, input_json, status, exit_code, artifact_dir, stdout_artifact, stderr_artifact, started_at, finished_at) VALUES ($1,$2,$3,$4,$5::jsonb,$6,$7,$8,$9,$10,$11,$12) ON CONFLICT (run_id) DO UPDATE SET plugin=EXCLUDED.plugin, container=EXCLUDED.container, target=EXCLUDED.target, input_json=EXCLUDED.input_json, status=EXCLUDED.status, exit_code=EXCLUDED.exit_code, artifact_dir=EXCLUDED.artifact_dir, stdout_artifact=EXCLUDED.stdout_artifact, stderr_artifact=EXCLUDED.stderr_artifact, started_at=EXCLUDED.started_at, finished_at=EXCLUDED.finished_at";
        let artifact_dir = run.artifact_dir.to_string_lossy().into_owned();
        let stdout_path = run.stdout_path.to_string_lossy().into_owned();
        let stderr_path = run.stderr_path.to_string_lossy().into_owned();
        let input_json: serde_json::Value = serde_json::from_str(run.input_json)?;
        let params: &[&(dyn tokio_postgres::types::ToSql + Sync)] = &[
            &run.run_id,
            &run.plugin,
            &run.container,
            &run.target,
            &input_json,
            &run.status,
            &run.exit_code,
            &artifact_dir,
            &stdout_path,
            &stderr_path,
            &run.started_at,
            &run.finished_at,
        ];
        let result = self.client.read().await.execute(SQL, params).await;
        match result {
            Ok(_) => {}
            Err(error) if error.is_closed() => {
                self.reconnect().await?;
                self.client.read().await.execute(SQL, params).await?;
            }
            Err(error) => return Err(error.into()),
        }
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

    /// Simulates the exact production incident this reconnect logic fixes:
    /// PostgreSQL closes the store's session out from under it (here, via
    /// `pg_terminate_backend`; in production this was a restart racing with
    /// slickchook's own startup) while the process itself keeps running.
    /// Before this fix, every subsequent query failed forever with
    /// `Error::is_closed()` ("connection closed") until the whole process
    /// was restarted. This asserts a query issued after the kill succeeds
    /// anyway, and that a write (`record_run`) does too.
    ///
    /// The kill/verify cycle is repeated twice (not just once), specifically
    /// because a live report claimed the drop "self-recovered once but
    /// happened twice" -- code inspection shows no one-shot state that would
    /// make recovery work only the first time (every query independently
    /// checks `is_closed()` and reconnects), but this asserts it directly
    /// rather than relying on that reasoning alone.
    #[tokio::test]
    async fn postgres_store_reconnects_after_connection_closed() {
        let Ok(url) = std::env::var("TEST_POSTGRES_URL") else {
            return;
        };
        let root =
            std::env::temp_dir().join(format!("slickchook-reconnect-{}", uuid::Uuid::new_v4()));
        let store = super::PostgresStore::connect(&url, root.clone(), super::DatabaseTls::Disable)
            .await
            .unwrap();
        store.init_schema().await.unwrap();

        let target = format!("reconnect-{}.example", uuid::Uuid::new_v4());
        assert!(
            store
                .list_runs_by_target(&target, 10)
                .await
                .unwrap()
                .is_empty(),
            "sanity check before killing the connection"
        );

        async fn kill_current_backend(store: &super::PostgresStore, url: &str) {
            let backend_pid: i32 = store
                .client
                .read()
                .await
                .query_one("SELECT pg_backend_pid()", &[])
                .await
                .unwrap()
                .get(0);
            let (admin, admin_connection) = tokio_postgres::connect(url, tokio_postgres::NoTls)
                .await
                .unwrap();
            tokio::spawn(async move {
                let _ = admin_connection.await;
            });
            admin
                .execute("SELECT pg_terminate_backend($1)", &[&backend_pid])
                .await
                .unwrap();
            // Give the connection-driving background task a moment to
            // observe the closed socket before the next query is issued.
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }

        // First kill/recover cycle.
        kill_current_backend(&store, &url).await;
        let runs = store
            .list_runs_by_target(&target, 10)
            .await
            .expect("query should transparently reconnect and succeed (1st drop)");
        assert!(runs.is_empty());

        let run_id = format!("reconnect-run-{}", uuid::Uuid::new_v4());
        let run_dir = root.join("run");
        std::fs::create_dir_all(run_dir.join("input")).unwrap();
        store
            .record_run(&super::StoredRun {
                run_id: &run_id,
                plugin: "test",
                container: "test:latest",
                target: Some(&target),
                status: "success",
                exit_code: Some(0),
                artifact_dir: &run_dir,
                stdout_path: &run_dir.join("stdout"),
                stderr_path: &run_dir.join("stderr"),
                input_json: "{}",
                started_at: "now",
                finished_at: "now",
            })
            .await
            .expect("record_run should also work after reconnecting (1st drop)");
        let runs_after = store.list_runs_by_target(&target, 10).await.unwrap();
        assert_eq!(runs_after.len(), 1);
        assert_eq!(runs_after[0].run_id, run_id);

        // Second kill/recover cycle, against the connection the first
        // reconnect established -- this is the part a one-shot "reconnect
        // once, then give up" bug would fail.
        kill_current_backend(&store, &url).await;
        let runs_second = store
            .list_runs_by_target(&target, 10)
            .await
            .expect("query should transparently reconnect and succeed (2nd drop)");
        assert_eq!(runs_second.len(), 1);

        let run_id_2 = format!("reconnect-run-{}", uuid::Uuid::new_v4());
        store
            .record_run(&super::StoredRun {
                run_id: &run_id_2,
                plugin: "test",
                container: "test:latest",
                target: Some(&target),
                status: "success",
                exit_code: Some(0),
                artifact_dir: &run_dir,
                stdout_path: &run_dir.join("stdout"),
                stderr_path: &run_dir.join("stderr"),
                input_json: "{}",
                started_at: "now",
                finished_at: "now",
            })
            .await
            .expect("record_run should also work after reconnecting (2nd drop)");
        let runs_final = store.list_runs_by_target(&target, 10).await.unwrap();
        assert_eq!(runs_final.len(), 2);

        let _ = std::fs::remove_dir_all(root);
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
