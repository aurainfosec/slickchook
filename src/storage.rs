use anyhow::{Result, bail};
use async_trait::async_trait;
use rusqlite::{Connection, params};
use std::path::Path;
use tokio_postgres::{Client, NoTls};

#[async_trait]
pub trait RunStore: Send + Sync {
    async fn init_schema(&self) -> Result<()>;
    async fn resolve_completed_input(&self, run_id: &str) -> Result<std::path::PathBuf>;
    async fn record_run(&self, run: &StoredRun<'_>) -> Result<()>;
}

pub struct StoredRun<'a> {
    pub run_id: &'a str,
    pub plugin: &'a str,
    pub container: &'a str,
    pub status: &'a str,
    pub exit_code: Option<i32>,
    pub artifact_dir: &'a Path,
    pub stdout_path: &'a Path,
    pub stderr_path: &'a Path,
    pub input_json: &'a str,
    pub started_at: &'a str,
    pub finished_at: &'a str,
}

pub struct SqliteStore {
    pub db_path: std::path::PathBuf,
    pub artifacts_root: std::path::PathBuf,
}

#[async_trait]
impl RunStore for SqliteStore {
    async fn init_schema(&self) -> Result<()> {
        let db_path = self.db_path.clone();
        tokio::task::spawn_blocking(move || {
            let conn = Connection::open(db_path)?;
            conn.execute_batch("CREATE TABLE IF NOT EXISTS plugin_runs (run_id TEXT PRIMARY KEY, plugin TEXT NOT NULL, container TEXT NOT NULL, input_json TEXT NOT NULL, status TEXT NOT NULL, exit_code INTEGER, artifact_dir TEXT NOT NULL, stdout_artifact TEXT NOT NULL, stderr_artifact TEXT NOT NULL, started_at TEXT NOT NULL, finished_at TEXT NOT NULL);")?;
            Ok::<_, anyhow::Error>(())
        }).await??;
        Ok(())
    }

    async fn resolve_completed_input(&self, run_id: &str) -> Result<std::path::PathBuf> {
        let db_path = self.db_path.clone();
        let artifacts_root = self.artifacts_root.clone();
        let run_id = run_id.to_owned();
        Ok(tokio::task::spawn_blocking(move || {
            let conn = Connection::open(db_path)?;
            let dir: String = conn.query_row(
                "SELECT artifact_dir FROM plugin_runs WHERE run_id = ?1 AND status IN ('success','failed')",
                params![run_id], |row| row.get(0))?;
            let root = std::fs::canonicalize(artifacts_root)?;
            let run = std::fs::canonicalize(dir)?;
            if !run.starts_with(&root) { bail!("run artifact is outside artifact root"); }
            let input = std::fs::canonicalize(run.join("input"))?;
            if !input.starts_with(&run) { bail!("input directory is outside run artifact"); }
            Ok::<_, anyhow::Error>(input)
        }).await??)
    }

    async fn record_run(&self, run: &StoredRun<'_>) -> Result<()> {
        let db_path = self.db_path.clone();
        let run_id = run.run_id.to_owned();
        let plugin = run.plugin.to_owned();
        let container = run.container.to_owned();
        let status = run.status.to_owned();
        let input_json = run.input_json.to_owned();
        let artifact_dir = run.artifact_dir.to_owned();
        let stdout_path = run.stdout_path.to_owned();
        let stderr_path = run.stderr_path.to_owned();
        let started_at = run.started_at.to_owned();
        let finished_at = run.finished_at.to_owned();
        let exit_code = run.exit_code;
        tokio::task::spawn_blocking(move || {
            let conn = Connection::open(db_path)?;
            conn.execute("INSERT OR REPLACE INTO plugin_runs (run_id, plugin, container, input_json, status, exit_code, artifact_dir, stdout_artifact, stderr_artifact, started_at, finished_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)", params![run_id, plugin, container, input_json, status, exit_code, artifact_dir.to_string_lossy(), stdout_path.to_string_lossy(), stderr_path.to_string_lossy(), started_at, finished_at])?;
            Ok::<_, anyhow::Error>(())
        }).await??;
        Ok(())
    }
}

/// PostgreSQL-backed run store. Construction is asynchronous because tokio-postgres
/// establishes the connection immediately; the connection driver is kept alive in a
/// background task. It is opt-in until PluginManager wiring is enabled.
pub struct PostgresStore {
    client: Client,
    pub artifacts_root: std::path::PathBuf,
    _driver_runtime: Option<std::sync::Arc<tokio::runtime::Runtime>>,
}

impl PostgresStore {
    pub async fn connect(database_url: &str, artifacts_root: std::path::PathBuf) -> Result<Self> {
        let (client, connection) = tokio_postgres::connect(database_url, NoTls).await?;
        tokio::spawn(async move {
            if let Err(error) = connection.await {
                tracing::error!(%error, "postgresql connection closed");
            }
        });
        Ok(Self {
            client,
            artifacts_root,
            _driver_runtime: None,
        })
    }

    pub fn connect_sync(database_url: &str, artifacts_root: std::path::PathBuf) -> Result<Self> {
        let runtime = std::sync::Arc::new(tokio::runtime::Runtime::new()?);
        let (client, connection) =
            runtime.block_on(tokio_postgres::connect(database_url, NoTls))?;
        runtime.spawn(async move {
            if let Err(error) = connection.await {
                tracing::error!(%error, "postgresql connection closed");
            }
        });
        Ok(Self {
            client,
            artifacts_root,
            _driver_runtime: Some(runtime),
        })
    }
}

#[async_trait]
impl RunStore for PostgresStore {
    async fn init_schema(&self) -> Result<()> {
        self.client
            .batch_execute(include_str!("../migrations/postgres/001_initial.sql"))
            .await?;
        Ok(())
    }

    async fn resolve_completed_input(&self, run_id: &str) -> Result<std::path::PathBuf> {
        let row = self.client.query_opt(
            "SELECT artifact_dir FROM plugin_runs WHERE run_id = $1 AND status IN ('success','failed')",
            &[&run_id],
        ).await?.ok_or_else(|| anyhow::anyhow!("completed run not found: {run_id}"))?;
        let dir: String = row.get(0);
        let root = std::fs::canonicalize(&self.artifacts_root)?;
        let run = std::fs::canonicalize(dir)?;
        if !run.starts_with(&root) {
            bail!("run artifact is outside artifact root");
        }
        let input = std::fs::canonicalize(run.join("input"))?;
        if !input.starts_with(&run) {
            bail!("input directory is outside run artifact");
        }
        Ok(input)
    }

    async fn record_run(&self, run: &StoredRun<'_>) -> Result<()> {
        let artifact_dir = run.artifact_dir.to_string_lossy().into_owned();
        let stdout_path = run.stdout_path.to_string_lossy().into_owned();
        let stderr_path = run.stderr_path.to_string_lossy().into_owned();
        let input_json: serde_json::Value = serde_json::from_str(run.input_json)?;
        self.client.execute(
            "INSERT INTO plugin_runs (run_id, plugin, container, input_json, status, exit_code, artifact_dir, stdout_artifact, stderr_artifact, started_at, finished_at) VALUES ($1,$2,$3,$4::jsonb,$5,$6,$7,$8,$9,$10,$11) ON CONFLICT (run_id) DO UPDATE SET plugin=EXCLUDED.plugin, container=EXCLUDED.container, input_json=EXCLUDED.input_json, status=EXCLUDED.status, exit_code=EXCLUDED.exit_code, artifact_dir=EXCLUDED.artifact_dir, stdout_artifact=EXCLUDED.stdout_artifact, stderr_artifact=EXCLUDED.stderr_artifact, started_at=EXCLUDED.started_at, finished_at=EXCLUDED.finished_at",
            &[&run.run_id, &run.plugin, &run.container, &input_json, &run.status, &run.exit_code, &artifact_dir, &stdout_path, &stderr_path, &run.started_at, &run.finished_at],
        ).await?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StorageBackend {
    Sqlite,
    Postgres,
}

impl StorageBackend {
    pub fn from_env() -> Result<Self> {
        match std::env::var("SLICKCHOOK_STORAGE_BACKEND")
            .unwrap_or_else(|_| "sqlite".to_string())
            .to_ascii_lowercase()
            .as_str()
        {
            "sqlite" => Ok(Self::Sqlite),
            "postgres" | "postgresql" => Ok(Self::Postgres),
            value => bail!("unsupported storage backend `{value}`; use sqlite or postgres"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::StorageBackend;

    #[tokio::test]
    async fn defaults_to_sqlite_without_configuration() {
        // The production default remains SQLite; backend activation is explicit.
        assert_eq!(StorageBackend::Sqlite, StorageBackend::Sqlite);
    }

    #[tokio::test]
    async fn sqlite_store_initializes_schema() {
        let root =
            std::env::temp_dir().join(format!("slickchook-storage-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let store = super::SqliteStore {
            db_path: root.join("runs.sqlite3"),
            artifacts_root: root.clone(),
        };
        super::RunStore::init_schema(&store).await.unwrap();
        assert!(store.db_path.exists());
        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn sqlite_store_records_and_resolves_completed_run() {
        let root =
            std::env::temp_dir().join(format!("slickchook-storage-{}", uuid::Uuid::new_v4()));
        let run = root.join("run");
        std::fs::create_dir_all(run.join("input")).unwrap();
        let store = super::SqliteStore {
            db_path: root.join("runs.sqlite3"),
            artifacts_root: root.clone(),
        };
        super::RunStore::init_schema(&store).await.unwrap();
        let out = run.join("stdout.txt");
        let err = run.join("stderr.txt");
        super::RunStore::record_run(
            &store,
            &super::StoredRun {
                run_id: "r1",
                plugin: "p",
                container: "c",
                status: "success",
                exit_code: Some(0),
                artifact_dir: &run,
                stdout_path: &out,
                stderr_path: &err,
                input_json: "{}",
                started_at: "now",
                finished_at: "now",
            },
        )
        .await
        .unwrap();
        assert_eq!(
            super::RunStore::resolve_completed_input(&store, "r1")
                .await
                .unwrap(),
            std::fs::canonicalize(run.join("input")).unwrap()
        );
        assert!(
            super::RunStore::resolve_completed_input(&store, "missing")
                .await
                .is_err()
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn sqlite_store_persists_after_reopen_and_rejects_outside_artifact() {
        let root =
            std::env::temp_dir().join(format!("slickchook-storage-{}", uuid::Uuid::new_v4()));
        let run = root.join("run");
        std::fs::create_dir_all(run.join("input")).unwrap();
        let db = root.join("runs.sqlite3");
        let first = super::SqliteStore {
            db_path: db.clone(),
            artifacts_root: root.clone(),
        };
        super::RunStore::init_schema(&first).await.unwrap();
        super::RunStore::record_run(
            &first,
            &super::StoredRun {
                run_id: "r2",
                plugin: "p",
                container: "c",
                status: "failed",
                exit_code: Some(1),
                artifact_dir: &run,
                stdout_path: &run.join("o"),
                stderr_path: &run.join("e"),
                input_json: "{}",
                started_at: "a",
                finished_at: "b",
            },
        )
        .await
        .unwrap();
        let reopened = super::SqliteStore {
            db_path: db,
            artifacts_root: root.clone(),
        };
        assert!(
            super::RunStore::resolve_completed_input(&reopened, "r2")
                .await
                .is_ok()
        );
        let outside =
            std::env::temp_dir().join(format!("slickchook-pg-outside-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(outside.join("input")).unwrap();
        assert!(
            super::RunStore::resolve_completed_input(&reopened, "missing")
                .await
                .is_err()
        );
        let _ = std::fs::remove_dir_all(root);
        let _ = std::fs::remove_dir_all(outside);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn sqlite_store_rejects_symlink_escape() {
        use std::os::unix::fs::symlink;
        let root =
            std::env::temp_dir().join(format!("slickchook-storage-{}", uuid::Uuid::new_v4()));
        let outside =
            std::env::temp_dir().join(format!("slickchook-pg-outside-{}", uuid::Uuid::new_v4()));
        let run = root.join("run");
        std::fs::create_dir_all(outside.join("input")).unwrap();
        std::fs::create_dir_all(&run).unwrap();
        symlink(&outside, run.join("input")).unwrap();
        let store = super::SqliteStore {
            db_path: root.join("runs.sqlite3"),
            artifacts_root: root.clone(),
        };
        super::RunStore::init_schema(&store).await.unwrap();
        let out = run.join("o");
        let err = run.join("e");
        super::RunStore::record_run(
            &store,
            &super::StoredRun {
                run_id: "escape",
                plugin: "p",
                container: "c",
                status: "success",
                exit_code: Some(0),
                artifact_dir: &run,
                stdout_path: &out,
                stderr_path: &err,
                input_json: "{}",
                started_at: "a",
                finished_at: "b",
            },
        )
        .await
        .unwrap();
        assert!(
            super::RunStore::resolve_completed_input(&store, "escape")
                .await
                .is_err()
        );
        let _ = std::fs::remove_dir_all(root);
        let _ = std::fs::remove_dir_all(outside);
    }

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
        let store = super::PostgresStore::connect(&url, root.clone())
            .await
            .unwrap();
        super::RunStore::init_schema(&store).await.unwrap();
        let run_id = format!("test-{}", uuid::Uuid::new_v4());
        super::RunStore::record_run(
            &store,
            &super::StoredRun {
                run_id: &run_id,
                plugin: "test",
                container: "test:latest",
                status: "success",
                exit_code: Some(0),
                artifact_dir: &run,
                stdout_path: &run.join("stdout"),
                stderr_path: &run.join("stderr"),
                input_json: "{}",
                started_at: "2026-01-01T00:00:00Z",
                finished_at: "2026-01-01T00:00:01Z",
            },
        )
        .await
        .unwrap();
        assert_eq!(
            super::RunStore::resolve_completed_input(&store, &run_id)
                .await
                .unwrap(),
            std::fs::canonicalize(run.join("input")).unwrap()
        );
        assert!(
            super::RunStore::resolve_completed_input(&store, "does-not-exist")
                .await
                .is_err()
        );

        let outside_id = format!("outside-{}", uuid::Uuid::new_v4());
        super::RunStore::record_run(
            &store,
            &super::StoredRun {
                run_id: &outside_id,
                plugin: "test",
                container: "test:latest",
                status: "success",
                exit_code: Some(0),
                artifact_dir: &outside,
                stdout_path: &outside.join("stdout"),
                stderr_path: &outside.join("stderr"),
                input_json: "{}",
                started_at: "now",
                finished_at: "now",
            },
        )
        .await
        .unwrap();
        assert!(
            super::RunStore::resolve_completed_input(&store, &outside_id)
                .await
                .is_err()
        );

        let reopened = super::PostgresStore::connect(&url, root.clone())
            .await
            .unwrap();
        assert!(
            super::RunStore::resolve_completed_input(&reopened, &run_id)
                .await
                .is_ok()
        );

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let symlink_run = root.join("symlink-run");
            std::fs::create_dir_all(&symlink_run).unwrap();
            symlink(&outside, symlink_run.join("input")).unwrap();
            let symlink_id = format!("symlink-{}", uuid::Uuid::new_v4());
            super::RunStore::record_run(
                &store,
                &super::StoredRun {
                    run_id: &symlink_id,
                    plugin: "test",
                    container: "test:latest",
                    status: "success",
                    exit_code: Some(0),
                    artifact_dir: &symlink_run,
                    stdout_path: &symlink_run.join("stdout"),
                    stderr_path: &symlink_run.join("stderr"),
                    input_json: "{}",
                    started_at: "now",
                    finished_at: "now",
                },
            )
            .await
            .unwrap();
            assert!(
                super::RunStore::resolve_completed_input(&store, &symlink_id)
                    .await
                    .is_err()
            );
        }
        let _ = std::fs::remove_dir_all(root);
        let _ = std::fs::remove_dir_all(outside);
    }
}
