use anyhow::Result;
use rmcp::{
    ServerHandler,
    handler::server::tool::{Parameters, ToolRouter},
    model::*,
    schemars, tool, tool_handler, tool_router,
    transport::sse_server::SseServer,
};
use serde::Deserialize;
use slickchook::plugin_manager::PluginManager;
use std::sync::Arc;
use std::{collections::HashMap, sync::Mutex};
use uuid::Uuid;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct RunPluginRequest {
    plugin: String,
    input: serde_json::Value,
    #[serde(default)]
    input_run_id: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct JobRequest {
    job_id: String,
}

#[derive(Clone)]
struct JobState {
    status: String,
    result: Option<String>,
}

#[derive(Clone)]
#[allow(dead_code)]
struct ReconServer {
    tool_router: ToolRouter<Self>,
    plugin_manager: Arc<PluginManager>,
    jobs: Arc<Mutex<HashMap<String, (JobState, tokio::task::AbortHandle)>>>,
}

impl ReconServer {
    fn new(plugin_manager: Arc<PluginManager>) -> Self {
        Self {
            tool_router: Self::tool_router(),
            plugin_manager,
            jobs: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[tool_router]
impl ReconServer {
    #[tool(description = "Run a recon plugin by name using JSON input")]
    async fn run_plugin(
        &self,
        Parameters(RunPluginRequest {
            plugin,
            input,
            input_run_id,
        }): Parameters<RunPluginRequest>,
    ) -> Result<String, rmcp::ErrorData> {
        let result = self
            .plugin_manager
            .run_plugin_with_input(&plugin, input, input_run_id.as_deref())
            .await
            .map_err(|error| rmcp::ErrorData::internal_error(error.to_string(), None))?;
        serde_json::to_string_pretty(&result)
            .map_err(|error| rmcp::ErrorData::internal_error(error.to_string(), None))
    }

    #[tool(description = "Start a recon plugin asynchronously and return a job ID")]
    async fn start_plugin(
        &self,
        Parameters(RunPluginRequest {
            plugin,
            input,
            input_run_id,
        }): Parameters<RunPluginRequest>,
    ) -> Result<String, rmcp::ErrorData> {
        let job_id = Uuid::new_v4().to_string();
        let manager = Arc::clone(&self.plugin_manager);
        let jobs = Arc::clone(&self.jobs);
        let id = job_id.clone();
        let handle = tokio::spawn(async move {
            let result = manager
                .run_plugin_with_input(&plugin, input, input_run_id.as_deref())
                .await
                .map(|value| serde_json::to_string_pretty(&value).unwrap_or_else(|e| e.to_string()))
                .unwrap_or_else(|e| format!("{{\"error\":{}}}", serde_json::json!(e.to_string())));
            if let Ok(mut guard) = jobs.lock() {
                if let Some((state, _)) = guard.get_mut(&id) {
                    state.status = "completed".into();
                    state.result = Some(result);
                }
            }
        });
        let abort = handle.abort_handle();
        self.jobs
            .lock()
            .map_err(|_| rmcp::ErrorData::internal_error("job lock poisoned", None))?
            .insert(
                job_id.clone(),
                (
                    JobState {
                        status: "running".into(),
                        result: None,
                    },
                    abort,
                ),
            );
        Ok(serde_json::json!({"job_id": job_id, "status": "running"}).to_string())
    }

    #[tool(description = "Get asynchronous plugin job status and result")]
    async fn plugin_status(
        &self,
        Parameters(JobRequest { job_id }): Parameters<JobRequest>,
    ) -> Result<String, rmcp::ErrorData> {
        let guard = self
            .jobs
            .lock()
            .map_err(|_| rmcp::ErrorData::internal_error("job lock poisoned", None))?;
        let (state, _) = guard
            .get(&job_id)
            .ok_or_else(|| rmcp::ErrorData::invalid_params("unknown job", None))?;
        Ok(
            serde_json::json!({"job_id": job_id, "status": state.status, "result": state.result})
                .to_string(),
        )
    }

    #[tool(description = "Cancel a running asynchronous plugin job")]
    async fn plugin_cancel(
        &self,
        Parameters(JobRequest { job_id }): Parameters<JobRequest>,
    ) -> Result<String, rmcp::ErrorData> {
        let mut guard = self
            .jobs
            .lock()
            .map_err(|_| rmcp::ErrorData::internal_error("job lock poisoned", None))?;
        let (state, abort) = guard
            .get_mut(&job_id)
            .ok_or_else(|| rmcp::ErrorData::invalid_params("unknown job", None))?;
        abort.abort();
        state.status = "cancelled".into();
        Ok(serde_json::json!({"job_id": job_id, "status": state.status}).to_string())
    }

    #[tool(description = "List available recon plugins and their declared inputs")]
    async fn list_plugins(&self) -> Result<String, rmcp::ErrorData> {
        let plugins = self
            .plugin_manager
            .list_plugins()
            .map_err(|error| rmcp::ErrorData::internal_error(error.to_string(), None))?;
        serde_json::to_string_pretty(&plugins)
            .map_err(|error| rmcp::ErrorData::internal_error(error.to_string(), None))
    }

    #[tool(description = "Basic health check")]
    async fn health(&self) -> Result<String, rmcp::ErrorData> {
        Ok("recon-mcp is running".to_string())
    }
}

#[tool_handler]
impl ServerHandler for ReconServer {
    fn get_info(&self) -> ServerInfo {
        server_info()
    }
}

fn server_info() -> ServerInfo {
    ServerInfo {
        capabilities: ServerCapabilities::builder().enable_tools().build(),
        instructions: Some(
            "HTTP MCP server for running recon plugins via JSON input/output.".into(),
        ),
        ..Default::default()
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let bind_addr = "127.0.0.1:8080";
    let plugin_manager = Arc::new(PluginManager::new(
        "plugins",
        "artifacts",
        "artifacts/plugin_runs.sqlite3",
    )?);

    let service_plugin_manager = Arc::clone(&plugin_manager);
    let _ct = SseServer::serve(bind_addr.parse()?)
        .await?
        .with_service(move || ReconServer::new(Arc::clone(&service_plugin_manager)));

    println!("MCP SSE server running at http://{}/sse", bind_addr);
    println!("Press Ctrl+C to stop.");

    tokio::signal::ctrl_c().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advertises_tool_capability() {
        assert!(server_info().capabilities.tools.is_some());
    }

    #[test]
    fn registers_all_mcp_tools() {
        let router = ReconServer::tool_router();

        assert!(router.has_route("run_plugin"));
        assert!(router.has_route("list_plugins"));
        assert!(router.has_route("health"));
        assert!(router.has_route("start_plugin"));
        assert!(router.has_route("plugin_status"));
        assert!(router.has_route("plugin_cancel"));
    }
}
