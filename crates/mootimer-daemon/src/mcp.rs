use anyhow::Result;
use mootimer_client::MooTimerClient;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::io::{self, BufRead, Write};
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    params: Option<Value>,
    id: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
    id: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

struct McpServer {
    client: Arc<MooTimerClient>,
}

impl McpServer {
    fn new(client: Arc<MooTimerClient>) -> Self {
        Self { client }
    }

    async fn handle_request(&self, request: JsonRpcRequest) -> Option<JsonRpcResponse> {
        let id = request.id.clone();

        if id.is_none() {
            if request.method == "notifications/initialized" {
                tracing::info!("MCP Client initialized");
            }
            return None;
        }

        let result = match request.method.as_str() {
            "initialize" => self.handle_initialize().await,
            "tools/list" => self.handle_tools_list().await,
            "tools/call" => self.handle_tools_call(request.params).await,
            _ => Err(JsonRpcError {
                code: -32601,
                message: "Method not found".to_string(),
                data: None,
            }),
        };

        let (result_val, error_val) = match result {
            Ok(v) => (Some(v), None),
            Err(e) => (None, Some(e)),
        };

        Some(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: result_val,
            error: error_val,
            id,
        })
    }

    async fn handle_initialize(&self) -> Result<Value, JsonRpcError> {
        Ok(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "mootimer-daemon-mcp", "version": "0.1.0" }
        }))
    }

    async fn handle_tools_list(&self) -> Result<Value, JsonRpcError> {
        Ok(json!({
            "tools": [
                {
                    "name": "list_profiles",
                    "description": "List all available user profiles in Mootimer.",
                    "inputSchema": { "type": "object", "properties": {} }
                },
                {
                    "name": "list_tasks",
                    "description": "List all non-archived tasks for a specific profile.",
                    "inputSchema": {
                        "type": "object",
                        "properties": { "profile_id": { "type": "string", "description": "The profile ID to list tasks from." } },
                        "required": ["profile_id"]
                    }
                },
                {
                    "name": "create_task",
                    "description": "Create a new task.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "profile_id": { "type": "string", "description": "The profile to add the task to." },
                            "title": { "type": "string", "description": "The title of the task." },
                            "description": { "type": "string", "description": "Optional description for the task." }
                        },
                        "required": ["profile_id", "title"]
                    }
                },
                {
                    "name": "update_task_status",
                    "description": "Update the status of a task (e.g., 'todo', 'in_progress', 'done', 'archived').",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "profile_id": { "type": "string", "description": "The profile containing the task." },
                            "task_id": { "type": "string", "description": "The ID of the task to update." },
                            "status": { "type": "string", "description": "The new status.", "enum": ["todo", "in_progress", "done", "archived"] }
                        },
                        "required": ["profile_id", "task_id", "status"]
                    }
                },
                {
                    "name": "delete_task",
                    "description": "Permanently delete a task.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "profile_id": { "type": "string", "description": "The profile containing the task." },
                            "task_id": { "type": "string", "description": "The ID of the task to delete." }
                        },
                        "required": ["profile_id", "task_id"]
                    }
                },
                {
                    "name": "list_entries",
                    "description": "List time entries for a profile. Optionally filter by date range, task, or tags.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "profile_id": { "type": "string", "description": "The profile to list entries from." },
                            "start_date": { "type": "string", "description": "Optional start date (ISO 8601)." },
                            "end_date": { "type": "string", "description": "Optional end date (ISO 8601)." },
                            "task_id": { "type": "string", "description": "Optional task ID to filter by." },
                            "tags": { "type": "array", "items": { "type": "string" }, "description": "Optional list of tags to filter by." }
                        },
                        "required": ["profile_id"]
                    }
                },
                {
                    "name": "update_entry",
                    "description": "Update a time entry. Requires the full entry object (usually obtained from list_entries).",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "profile_id": { "type": "string", "description": "The profile containing the entry." },
                            "entry": { "type": "object", "description": "The full entry object to update." }
                        },
                        "required": ["profile_id", "entry"]
                    }
                },
                {
                    "name": "delete_entry",
                    "description": "Delete a time entry.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "profile_id": { "type": "string", "description": "The profile containing the entry." },
                            "entry_id": { "type": "string", "description": "The ID of the entry to delete." }
                        },
                        "required": ["profile_id", "entry_id"]
                    }
                },
                { "name": "get_timer_status", "description": "Get the status of the timer for a specific profile.", "inputSchema": { "type": "object", "properties": { "profile_id": { "type": "string" } }, "required": ["profile_id"] } },
                { "name": "start_timer", "description": "Start a simple manual (stopwatch) timer.", "inputSchema": { "type": "object", "properties": { "profile_id": { "type": "string" }, "task_id": { "type": "string" } }, "required": ["profile_id"] } },
                { "name": "stop_timer", "description": "Stops the currently running timer, saving the time entry.", "inputSchema": { "type": "object", "properties": { "profile_id": { "type": "string" } }, "required": ["profile_id"] } }
            ]
        }))
    }

    async fn handle_tools_call(&self, params: Option<Value>) -> Result<Value, JsonRpcError> {
        let params = params.ok_or_else(|| JsonRpcError {
            code: -32602,
            message: "Invalid params".to_string(),
            data: None,
        })?;
        let name = get_string_arg(&params, "name")?;
        let args = params.get("arguments").cloned().unwrap_or(json!({}));

        let daemon_result = match name {
            "list_profiles" => self.client.profile_list().await,
            "list_tasks" => {
                let profile_id = get_string_arg(&args, "profile_id")?;
                self.client.task_list(profile_id).await
            }
            "create_task" => {
                let profile_id = get_string_arg(&args, "profile_id")?;
                let title = get_string_arg(&args, "title")?;
                let description = args.get("description").and_then(|v| v.as_str());
                self.client
                    .task_create(profile_id, title, description)
                    .await
            }
            "update_task_status" => {
                let profile_id = get_string_arg(&args, "profile_id")?;
                let task_id = get_string_arg(&args, "task_id")?;
                let status = get_string_arg(&args, "status")?;

                let mut task = self
                    .client
                    .task_get(profile_id, task_id)
                    .await
                    .map_err(|e| JsonRpcError {
                        code: -32000,
                        message: format!("Failed to get task: {}", e),
                        data: None,
                    })?;
                if let Some(obj) = task.as_object_mut() {
                    obj.insert("status".to_string(), json!(status));
                }

                self.client.task_update(profile_id, task).await
            }
            "delete_task" => {
                let profile_id = get_string_arg(&args, "profile_id")?;
                let task_id = get_string_arg(&args, "task_id")?;
                self.client.task_delete(profile_id, task_id).await
            }
            "list_entries" => {
                let profile_id = get_string_arg(&args, "profile_id")?;
                let start_date = args
                    .get("start_date")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let end_date = args
                    .get("end_date")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let task_id = args.get("task_id").and_then(|v| v.as_str());
                let tags = args.get("tags").and_then(|v| v.as_array()).map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                });

                if start_date.is_some() || end_date.is_some() || task_id.is_some() || tags.is_some()
                {
                    self.client
                        .entry_filter(profile_id, start_date, end_date, task_id, tags)
                        .await
                } else {
                    self.client.entry_list(profile_id).await
                }
            }
            "update_entry" => {
                let profile_id = get_string_arg(&args, "profile_id")?;
                let entry = args
                    .get("entry")
                    .ok_or_else(|| JsonRpcError {
                        code: -32602,
                        message: "Missing entry object".to_string(),
                        data: None,
                    })?
                    .clone();
                self.client.entry_update(profile_id, entry).await
            }
            "delete_entry" => {
                let profile_id = get_string_arg(&args, "profile_id")?;
                let entry_id = get_string_arg(&args, "entry_id")?;
                self.client.entry_delete(profile_id, entry_id).await
            }
            "get_timer_status" => {
                let profile_id = get_string_arg(&args, "profile_id")?;
                self.client.timer_get(profile_id).await
            }
            "start_timer" => {
                let profile_id = get_string_arg(&args, "profile_id")?;
                let task_id = args.get("task_id").and_then(|v| v.as_str());
                self.client.timer_start_manual(profile_id, task_id).await
            }
            "start_pomodoro_timer" => {
                let profile_id = get_string_arg(&args, "profile_id")?;
                let task_id = args.get("task_id").and_then(|v| v.as_str());
                self.client
                    .timer_start_pomodoro(profile_id, task_id, None)
                    .await
            }
            "start_countdown_timer" => {
                let profile_id = get_string_arg(&args, "profile_id")?;
                let task_id = args.get("task_id").and_then(|v| v.as_str());
                let duration = args
                    .get("duration_minutes")
                    .and_then(|v| v.as_u64())
                    .ok_or_else(|| JsonRpcError {
                        code: -32602,
                        message: "Missing or invalid duration_minutes".to_string(),
                        data: None,
                    })?;
                self.client
                    .timer_start_countdown(profile_id, task_id, duration)
                    .await
            }
            "stop_timer" => {
                let profile_id = get_string_arg(&args, "profile_id")?;
                self.client.timer_stop(profile_id).await
            }
            _ => {
                return Err(JsonRpcError {
                    code: -32601,
                    message: format!("Tool not found: {}", name),
                    data: None,
                });
            }
        };

        let result_value = daemon_result.map_err(|e| JsonRpcError {
            code: -32000,
            message: format!("Daemon error: {}", e),
            data: None,
        })?;
        let result_text = serde_json::to_string_pretty(&result_value)
            .unwrap_or_else(|_| "Failed to serialize result".to_string());

        Ok(json!({
            "content": [
                {
                    "type": "text",
                    "text": result_text
                }
            ]
        }))
    }
}

fn get_string_arg<'a>(args: &'a Value, key: &'a str) -> Result<&'a str, JsonRpcError> {
    args.get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| JsonRpcError {
            code: -32602,
            message: format!("Missing or invalid argument: {}", key),
            data: None,
        })
}

pub async fn run_mcp_server(socket_path: String) -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(io::stderr)
        .with_env_filter("info")
        .init();

    let client = Arc::new(MooTimerClient::new(socket_path));

    if let Err(e) = client.profile_list().await {
        anyhow::bail!(
            "Failed to connect to daemon: {}. Make sure the daemon is running with: mootimerd",
            e
        );
    }

    tracing::info!("Connected to daemon successfully");

    let server = Arc::new(McpServer::new(client));
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    tracing::info!("MooTimer MCP server mode started. Listening on stdin.");

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        tracing::debug!(%line, "Received MCP request");

        match serde_json::from_str::<JsonRpcRequest>(&line) {
            Ok(request) => {
                let server = server.clone();
                if let Some(response) = server.handle_request(request).await {
                    let response_str = serde_json::to_string(&response)?;
                    tracing::debug!(%response_str, "Sending MCP response");
                    writeln!(stdout, "{}", response_str)?;
                    stdout.flush()?;
                }
            }
            Err(e) => {
                tracing::error!("Failed to parse request: {}", e);
                let error_response = json!({
                    "jsonrpc": "2.0",
                    "id": null,
                    "error": { "code": -32700, "message": "Parse error" }
                });
                writeln!(stdout, "{}", serde_json::to_string(&error_response)?)?;
                stdout.flush()?;
            }
        }
    }
    Ok(())
}
