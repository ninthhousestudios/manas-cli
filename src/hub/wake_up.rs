
use anyhow::{Context, Result};

pub async fn run(
    chitta_url: &str,
    yojana_url: &str,
    args: serde_json::Value,
) -> Result<String> {
    let project = args
        .get("project")
        .and_then(|v| v.as_str())
        .context("missing required field: project")?;
    let profile = args
        .get("profile")
        .and_then(|v| v.as_str())
        .unwrap_or("josh");
    let max_tokens: usize = args
        .get("max_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(1500) as usize;
    let include_profile = args
        .get("include_profile")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let profile_future = async {
        if include_profile {
            fetch_profile(chitta_url, profile).await
        } else {
            Ok(Vec::new())
        }
    };

    let (profile_entries, tasks) = tokio::join!(profile_future, fetch_tasks(yojana_url, project));

    let mut sections = Vec::new();
    let mut source_profile_entries = 0;
    let mut source_tasks = 0;

    if let Ok(entries) = profile_entries {
        if !entries.is_empty() {
            source_profile_entries = entries.len();
            let mut lines = vec!["## profile context".to_string()];
            for entry in &entries {
                lines.push(format!("- {}", entry));
            }
            sections.push(lines.join("\n"));
        }
    }

    if let Ok(tsks) = tasks {
        if !tsks.is_empty() {
            source_tasks = tsks.len();
            let mut lines = vec!["## open tasks".to_string()];
            for t in &tsks {
                lines.push(format!("- {}", t));
            }
            sections.push(lines.join("\n"));
        }
    }

    if sections.is_empty() {
        return Ok("no context available".into());
    }

    let mut preamble = sections.join("\n\n");
    let char_budget = max_tokens * 4;
    if preamble.len() > char_budget {
        preamble.truncate(char_budget);
        if let Some(last_newline) = preamble.rfind('\n') {
            preamble.truncate(last_newline);
        }
        preamble.push_str("\n\n(truncated)");
    }

    preamble.push_str(&format!(
        "\n\n---\nsources: {} profile entries, {} tasks",
        source_profile_entries, source_tasks
    ));

    Ok(preamble)
}

async fn fetch_profile(chitta_url: &str, profile: &str) -> Result<Vec<String>> {
    let client = reqwest::Client::new();

    let resp = mcp_call(
        &client,
        chitta_url,
        "get_profile",
        serde_json::json!({
            "profile": profile,
        }),
    )
    .await?;

    let content_text = extract_mcp_text(&resp)?;
    let parsed: serde_json::Value =
        serde_json::from_str(&content_text).unwrap_or_else(|_| serde_json::json!([]));

    let mut lines = Vec::new();
    if let Some(arr) = parsed.get("entries").and_then(|v| v.as_array()) {
        for mem in arr {
            let mtype = mem
                .get("memory_type")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            let content = mem.get("content").and_then(|v| v.as_str()).unwrap_or("");
            let date = mem.get("event_time").and_then(|v| v.as_str()).unwrap_or("");
            let short_date = date.get(..10).unwrap_or(date);
            let summary = if content.len() > 120 {
                format!("{}...", &content[..120])
            } else {
                content.to_string()
            };
            lines.push(format!("[{mtype}] {summary} ({short_date})"));
        }
    }

    Ok(lines)
}

async fn fetch_tasks(yojana_url: &str, project: &str) -> Result<Vec<String>> {
    let client = reqwest::Client::new();

    let resp = mcp_call(
        &client,
        yojana_url,
        "yojana_query",
        serde_json::json!({
            "project": project,
            "status": "in_progress",
        }),
    )
    .await;

    let in_progress = parse_tasks(&resp.unwrap_or_default());

    let resp = mcp_call(
        &client,
        yojana_url,
        "yojana_query",
        serde_json::json!({
            "project": project,
        }),
    )
    .await;

    let all_open = parse_tasks(&resp.unwrap_or_default());

    let mut lines = Vec::new();
    for t in &in_progress {
        lines.push(t.clone());
    }
    for t in &all_open {
        if !in_progress.contains(t) {
            lines.push(t.clone());
        }
    }

    Ok(lines)
}

fn parse_tasks(resp: &serde_json::Value) -> Vec<String> {
    let content_text = extract_mcp_text(resp).unwrap_or_default();
    let parsed: serde_json::Value =
        serde_json::from_str(&content_text).unwrap_or_else(|_| serde_json::json!([]));

    let mut lines = Vec::new();
    if let Some(arr) = parsed.as_array() {
        for task in arr {
            let hid = task.get("human_id").and_then(|v| v.as_str()).unwrap_or("?");
            let status = task.get("status").and_then(|v| v.as_str()).unwrap_or("?");
            let title = task.get("title").and_then(|v| v.as_str()).unwrap_or("?");
            let blocked = task
                .get("blocked")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let marker = if blocked { " [BLOCKED]" } else { "" };
            lines.push(format!("{hid} [{status}]{marker} {title}"));
        }
    }
    lines
}

fn extract_mcp_text(resp: &serde_json::Value) -> Result<String> {
    resp.pointer("/result/content/0/text")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .context("no text content in MCP response")
}

async fn mcp_call(
    client: &reqwest::Client,
    base_url: &str,
    tool_name: &str,
    arguments: serde_json::Value,
) -> Result<serde_json::Value> {
    let init_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "manas-hub", "version": env!("CARGO_PKG_VERSION") }
        }
    });

    let init_resp = client
        .post(format!("{base_url}/mcp"))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .json(&init_body)
        .send()
        .await
        .context("service unreachable")?;

    let session_id = init_resp
        .headers()
        .get("mcp-session-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_default();

    let _ = init_resp.text().await;

    let call_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": tool_name,
            "arguments": arguments,
        }
    });

    let resp = client
        .post(format!("{base_url}/mcp"))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .header("mcp-session-id", &session_id)
        .json(&call_body)
        .send()
        .await
        .context("tool call failed")?;

    let text = resp.text().await.context("reading response")?;

    let json_line = text
        .lines()
        .filter(|line| line.starts_with("data: "))
        .map(|line| &line[6..])
        .filter(|s| !s.is_empty())
        .last()
        .unwrap_or(&text);

    serde_json::from_str(json_line).context("parsing response JSON")
}
