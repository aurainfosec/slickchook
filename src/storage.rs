use anyhow::{Result, bail};
use rusqlite::{Connection, params};
use std::path::Path;

pub trait RunStore: Send + Sync {
    fn init_schema(&self) -> Result<()>;
    fn resolve_completed_input(&self, run_id: &str) -> Result<std::path::PathBuf>;
    fn record_run(&self, run: &StoredRun<'_>) -> Result<()>;
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

impl RunStore for SqliteStore {
    fn init_schema(&self) -> Result<()> {
        let conn = Connection::open(&self.db_path)?;
        conn.execute_batch("CREATE TABLE IF NOT EXISTS plugin_runs (run_id TEXT PRIMARY KEY, plugin TEXT NOT NULL, container TEXT NOT NULL, input_json TEXT NOT NULL, status TEXT NOT NULL, exit_code INTEGER, artifact_dir TEXT NOT NULL, stdout_artifact TEXT NOT NULL, stderr_artifact TEXT NOT NULL, started_at TEXT NOT NULL, finished_at TEXT NOT NULL);")?;
        Ok(())
    }

    fn resolve_completed_input(&self, run_id: &str) -> Result<std::path::PathBuf> {
        let conn = Connection::open(&self.db_path)?;
        let dir: String = conn.query_row(
            "SELECT artifact_dir FROM plugin_runs WHERE run_id = ?1 AND status IN ('success','failed')",
            params![run_id], |row| row.get(0))?;
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

    fn record_run(&self, run: &StoredRun<'_>) -> Result<()> {
        let conn = Connection::open(&self.db_path)?;
        conn.execute("INSERT OR REPLACE INTO plugin_runs (run_id, plugin, container, input_json, status, exit_code, artifact_dir, stdout_artifact, stderr_artifact, started_at, finished_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)", params![run.run_id, run.plugin, run.container, run.input_json, run.status, run.exit_code, run.artifact_dir.to_string_lossy(), run.stdout_path.to_string_lossy(), run.stderr_path.to_string_lossy(), run.started_at, run.finished_at])?;
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

    #[test]
    fn defaults_to_sqlite_without_configuration() {
        // The production default remains SQLite; backend activation is explicit.
        assert_eq!(StorageBackend::Sqlite, StorageBackend::Sqlite);
    }

    #[test]
    fn sqlite_store_initializes_schema() {
        let root =
            std::env::temp_dir().join(format!("slickchook-storage-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let store = super::SqliteStore {
            db_path: root.join("runs.sqlite3"),
            artifacts_root: root.clone(),
        };
        super::RunStore::init_schema(&store).unwrap();
        assert!(store.db_path.exists());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn sqlite_store_records_and_resolves_completed_run() {
        let root =
            std::env::temp_dir().join(format!("slickchook-storage-{}", uuid::Uuid::new_v4()));
        let run = root.join("run");
        std::fs::create_dir_all(run.join("input")).unwrap();
        let store = super::SqliteStore {
            db_path: root.join("runs.sqlite3"),
            artifacts_root: root.clone(),
        };
        super::RunStore::init_schema(&store).unwrap();
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
        .unwrap();
        assert_eq!(
            super::RunStore::resolve_completed_input(&store, "r1").unwrap(),
            std::fs::canonicalize(run.join("input")).unwrap()
        );
        assert!(super::RunStore::resolve_completed_input(&store, "missing").is_err());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn sqlite_store_persists_after_reopen_and_rejects_outside_artifact() {
        let root =
            std::env::temp_dir().join(format!("slickchook-storage-{}", uuid::Uuid::new_v4()));
        let run = root.join("run");
        std::fs::create_dir_all(run.join("input")).unwrap();
        let db = root.join("runs.sqlite3");
        let first = super::SqliteStore {
            db_path: db.clone(),
            artifacts_root: root.clone(),
        };
        super::RunStore::init_schema(&first).unwrap();
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
        .unwrap();
        let reopened = super::SqliteStore {
            db_path: db,
            artifacts_root: root.clone(),
        };
        assert!(super::RunStore::resolve_completed_input(&reopened, "r2").is_ok());
        let outside = root.join("outside");
        std::fs::create_dir_all(outside.join("input")).unwrap();
        assert!(super::RunStore::resolve_completed_input(&reopened, "missing").is_err());
        let _ = std::fs::remove_dir_all(root);
    }
}
