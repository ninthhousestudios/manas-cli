use std::sync::Arc;

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};

use super::wake_up;

#[derive(Clone)]
pub struct HubState {
    pub chitta_url: String,
    pub yojana_url: String,
}

#[derive(Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    #[serde(default)]
    id: Option<serde_json::Value>,
    method: String,
    #[serde(default)]
    params: serde_json::Value,
}

#[derive(Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Serialize)]
struct JsonRpcError {
    code: i64,
    message: String,
}

impl JsonRpcResponse {
    fn success(id: Option<serde_json::Value>, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: Some(result),
            error: None,
        }
    }

    fn error(id: Option<serde_json::Value>, code: i64, message: String) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: None,
            error: Some(JsonRpcError { code, message }),
        }
    }
}

static TOOL_DEFS: &[(&str, &str, &str)] = &[
    (
        "manas_wake_up",
        "Session-start context injection. Fans out to chitta + yojana and returns a merged preamble.",
        r#"{
            "type": "object",
            "properties": {
                "project": { "type": "string", "description": "Project name or slug" },
                "profile": { "type": "string", "description": "Chitta profile for get_profile (default: josh)", "default": "josh" },
                "include_profile": { "type": "boolean", "description": "Fetch chitta profile (default: false)", "default": false },
                "max_tokens": { "type": "integer", "description": "Token budget for the preamble (default: 1500)", "default": 1500 }
            },
            "required": ["project"]
        }"#,
    ),
    (
        "manas_ingest",
        "Accept raw text for background extraction into chitta.",
        r#"{
            "type": "object",
            "properties": {
                "text": { "type": "string", "description": "Raw text to ingest" },
                "project": { "type": "string", "description": "Project context" },
                "profile": { "type": "string", "description": "Chitta profile (default: chitta)", "default": "chitta" },
                "source": { "type": "string", "description": "Origin (e.g. hook:post, hook:compact)" }
            },
            "required": ["text", "project"]
        }"#,
    ),
];

pub async fn handle_mcp(
    State(state): State<Arc<HubState>>,
    headers: HeaderMap,
    body: String,
) -> impl IntoResponse {
    let wants_sse = headers
        .get("accept")
        .and_then(|v| v.to_str().ok())
        .map(|a| a.contains("text/event-stream"))
        .unwrap_or(false);

    let req: JsonRpcRequest = match serde_json::from_str(&body) {
        Ok(r) => r,
        Err(e) => {
            let resp = JsonRpcResponse::error(None, -32700, format!("parse error: {e}"));
            return wrap(resp, wants_sse, "manas-hub");
        }
    };

    if req.jsonrpc != "2.0" {
        let resp = JsonRpcResponse::error(req.id, -32600, "invalid jsonrpc version".into());
        return wrap(resp, wants_sse, "manas-hub");
    }

    let is_initialize = req.method == "initialize";

    let resp = match req.method.as_str() {
        "initialize" => handle_initialize(req.id),
        "notifications/initialized" => {
            return StatusCode::ACCEPTED.into_response();
        }
        "tools/list" => handle_tools_list(req.id),
        "tools/call" => handle_tools_call(req.id, req.params, &state).await,
        _ => JsonRpcResponse::error(req.id, -32601, format!("method not found: {}", req.method)),
    };

    let session_id = if is_initialize {
        uuid::Uuid::now_v7().to_string()
    } else {
        headers
            .get("mcp-session-id")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("manas-hub")
            .to_string()
    };

    wrap(resp, wants_sse, &session_id)
}

fn handle_initialize(id: Option<serde_json::Value>) -> JsonRpcResponse {
    JsonRpcResponse::success(
        id,
        serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": { "listChanged": false }
            },
            "serverInfo": {
                "name": "manas",
                "version": env!("CARGO_PKG_VERSION")
            }
        }),
    )
}

fn handle_tools_list(id: Option<serde_json::Value>) -> JsonRpcResponse {
    let tools: Vec<serde_json::Value> = TOOL_DEFS
        .iter()
        .map(|(name, desc, schema)| {
            serde_json::json!({
                "name": name,
                "description": desc,
                "inputSchema": serde_json::from_str::<serde_json::Value>(schema).unwrap(),
            })
        })
        .collect();

    JsonRpcResponse::success(id, serde_json::json!({ "tools": tools }))
}

async fn handle_tools_call(
    id: Option<serde_json::Value>,
    params: serde_json::Value,
    state: &HubState,
) -> JsonRpcResponse {
    let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or(serde_json::json!({}));

    match tool_name {
        "manas_wake_up" => {
            match wake_up::run(&state.chitta_url, &state.yojana_url, arguments).await {
                Ok(result) => JsonRpcResponse::success(
                    id,
                    serde_json::json!({
                        "content": [{ "type": "text", "text": result }]
                    }),
                ),
                Err(e) => JsonRpcResponse::success(
                    id,
                    serde_json::json!({
                        "content": [{ "type": "text", "text": format!("error: {e}") }],
                        "isError": true
                    }),
                ),
            }
        }
        "manas_ingest" => match ingest(state, arguments).await {
            Ok(result) => JsonRpcResponse::success(
                id,
                serde_json::json!({
                    "content": [{ "type": "text", "text": result }]
                }),
            ),
            Err(e) => JsonRpcResponse::success(
                id,
                serde_json::json!({
                    "content": [{ "type": "text", "text": format!("error: {e}") }],
                    "isError": true
                }),
            ),
        },
        _ => JsonRpcResponse::error(id, -32602, format!("unknown tool: {tool_name}")),
    }
}

async fn ingest(state: &HubState, args: serde_json::Value) -> anyhow::Result<String> {
    let text = args.get("text").and_then(|v| v.as_str()).unwrap_or("");
    let project = args.get("project").and_then(|v| v.as_str()).unwrap_or("");
    let profile = args
        .get("profile")
        .and_then(|v| v.as_str())
        .unwrap_or("chitta");
    let source = args.get("source").and_then(|v| v.as_str()).unwrap_or("mcp");

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/ingest", state.chitta_url))
        .json(&serde_json::json!({
            "text": text,
            "project": project,
            "profile": profile,
            "source": source,
            "max_importance": "medium",
        }))
        .send()
        .await?;

    let status = resp.status();
    if status.is_success() {
        Ok(format!("accepted ({})", status))
    } else {
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("chitta ingest returned {status}: {body}")
    }
}

fn wrap(resp: JsonRpcResponse, sse: bool, session_id: &str) -> axum::response::Response {
    let json = serde_json::to_string(&resp).unwrap();

    let mut response = if sse {
        let body = format!("event: message\ndata: {json}\n\n");
        (
            StatusCode::OK,
            [
                ("content-type", "text/event-stream"),
                ("cache-control", "no-cache"),
            ],
            body,
        )
            .into_response()
    } else {
        (StatusCode::OK, [("content-type", "application/json")], json).into_response()
    };

    response.headers_mut().insert(
        "mcp-session-id",
        session_id
            .parse()
            .unwrap_or_else(|_| "manas-hub".parse().unwrap()),
    );
    response
}
