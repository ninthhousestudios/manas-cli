use std::sync::Arc;

use anyhow::{bail, Context, Result};
use tokio::sync::Mutex;

#[async_trait::async_trait]
pub trait LockClient: Send + Sync {
    async fn claim(
        &self,
        resource: &str,
        session_id: &str,
        scope: LockScope,
        ttl_secs: u64,
    ) -> Result<ClaimResult>;

    async fn release(&self, resource: &str, session_id: &str) -> Result<()>;

    async fn heartbeat(&self, resource: &str, session_id: &str, ttl_secs: u64) -> Result<()>;
}

#[derive(Debug, Clone)]
pub enum LockScope {
    Project,
    User,
}

impl LockScope {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Project => "project",
            Self::User => "__user__",
        }
    }
}

#[derive(Debug)]
pub enum ClaimResult {
    Acquired,
    AlreadyHeld { by_session: String },
}

pub struct SanghaLockClient {
    client: reqwest::Client,
    base_url: String,
    project: String,
    mcp_session: Arc<Mutex<Option<String>>>,
    request_id: Arc<Mutex<u64>>,
}

impl SanghaLockClient {
    pub fn new(sangha_url: &str, project: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: sangha_url.trim_end_matches('/').to_string(),
            project: project.to_string(),
            mcp_session: Arc::new(Mutex::new(None)),
            request_id: Arc::new(Mutex::new(0)),
        }
    }

    async fn next_id(&self) -> u64 {
        let mut id = self.request_id.lock().await;
        *id += 1;
        *id
    }

    async fn ensure_initialized(&self) -> Result<()> {
        let mut session = self.mcp_session.lock().await;
        if session.is_some() {
            return Ok(());
        }

        let id = self.next_id().await;
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "manas-cli", "version": env!("CARGO_PKG_VERSION") }
            }
        });

        let resp = self
            .client
            .post(format!("{}/mcp", self.base_url))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .json(&body)
            .send()
            .await
            .context("sangha unreachable")?;

        if !resp.status().is_success() {
            bail!("sangha initialize failed: {}", resp.status());
        }

        let mcp_session_id = resp
            .headers()
            .get("mcp-session-id")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .context("sangha did not return mcp-session-id header")?;

        // Consume the SSE body
        let _ = resp.text().await;

        // Send initialized notification
        let notify_body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        });

        let _ = self
            .client
            .post(format!("{}/mcp", self.base_url))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .header("mcp-session-id", &mcp_session_id)
            .json(&notify_body)
            .send()
            .await;

        *session = Some(mcp_session_id);
        Ok(())
    }

    async fn register_session(&self, session_id: &str) -> Result<()> {
        self.mcp_call(
            "session_register",
            serde_json::json!({
                "project": self.project,
                "intent": format!("lock client for session {}", session_id),
            }),
        )
        .await?;
        Ok(())
    }

    async fn mcp_call(&self, method: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        self.ensure_initialized().await?;

        let session = self.mcp_session.lock().await;
        let session_id = session.as_ref().unwrap();

        let id = self.next_id().await;
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": {
                "name": method,
                "arguments": params,
            }
        });

        let resp = self
            .client
            .post(format!("{}/mcp", self.base_url))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .header("mcp-session-id", session_id)
            .json(&body)
            .send()
            .await
            .context("sangha unreachable")?;

        let status = resp.status();
        let text = resp.text().await.context("reading sangha response")?;

        if !status.is_success() {
            bail!("sangha returned {}: {}", status, text);
        }

        // Parse SSE response — find the last "data:" line containing JSON
        let json_line = text
            .lines()
            .filter(|line| line.starts_with("data: "))
            .map(|line| &line[6..])
            .filter(|s| !s.is_empty())
            .last()
            .context("no JSON payload in sangha SSE response")?;

        let parsed: serde_json::Value =
            serde_json::from_str(json_line).context("parsing sangha response JSON")?;

        if let Some(error) = parsed.get("error") {
            bail!("sangha error: {}", error);
        }

        Ok(parsed)
    }
}

#[async_trait::async_trait]
impl LockClient for SanghaLockClient {
    async fn claim(
        &self,
        resource: &str,
        session_id: &str,
        scope: LockScope,
        ttl_secs: u64,
    ) -> Result<ClaimResult> {
        // Register session on first claim (sangha requires it)
        {
            let session = self.mcp_session.lock().await;
            if session.is_none() {
                drop(session);
                self.ensure_initialized().await?;
                self.register_session(session_id).await?;
            }
        }

        let result = self
            .mcp_call(
                "resource_claim",
                serde_json::json!({
                    "resource": resource,
                    "project": self.project,
                    "scope": scope.as_str(),
                    "ttl_seconds": ttl_secs,
                }),
            )
            .await?;

        if let Some(content) = result.pointer("/result/content/0/text") {
            let text = content.as_str().unwrap_or("");
            if text.contains("already held") || text.contains("conflict") {
                return Ok(ClaimResult::AlreadyHeld {
                    by_session: session_id.to_string(),
                });
            }
        }

        Ok(ClaimResult::Acquired)
    }

    async fn release(&self, resource: &str, _session_id: &str) -> Result<()> {
        self.mcp_call(
            "resource_release",
            serde_json::json!({
                "resource": resource,
                "project": self.project,
            }),
        )
        .await?;

        Ok(())
    }

    async fn heartbeat(&self, resource: &str, _session_id: &str, ttl_secs: u64) -> Result<()> {
        self.mcp_call(
            "resource_claim",
            serde_json::json!({
                "resource": resource,
                "project": self.project,
                "ttl_seconds": ttl_secs,
            }),
        )
        .await?;

        Ok(())
    }
}

#[cfg(test)]
pub mod mock {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[derive(Default, Clone)]
    pub struct MockLockClient {
        pub claims: Arc<Mutex<Vec<String>>>,
        pub releases: Arc<Mutex<Vec<String>>>,
        pub should_conflict: Arc<Mutex<bool>>,
    }

    #[async_trait::async_trait]
    impl LockClient for MockLockClient {
        async fn claim(
            &self,
            resource: &str,
            _session_id: &str,
            _scope: LockScope,
            _ttl_secs: u64,
        ) -> Result<ClaimResult> {
            if *self.should_conflict.lock().unwrap() {
                return Ok(ClaimResult::AlreadyHeld {
                    by_session: "other-session".into(),
                });
            }
            self.claims.lock().unwrap().push(resource.to_string());
            Ok(ClaimResult::Acquired)
        }

        async fn release(&self, resource: &str, _session_id: &str) -> Result<()> {
            self.releases.lock().unwrap().push(resource.to_string());
            Ok(())
        }

        async fn heartbeat(&self, _resource: &str, _session_id: &str, _ttl_secs: u64) -> Result<()> {
            Ok(())
        }
    }
}
