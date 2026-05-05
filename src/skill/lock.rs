use anyhow::{bail, Context, Result};

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
}

impl SanghaLockClient {
    pub fn new(sangha_url: &str, project: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: sangha_url.trim_end_matches('/').to_string(),
            project: project.to_string(),
        }
    }

    async fn mcp_call(&self, method: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": method,
                "arguments": params,
            }
        });

        let resp = self
            .client
            .post(format!("{}/mcp", self.base_url))
            .json(&body)
            .send()
            .await
            .context("sangha unreachable")?;

        let status = resp.status();
        let text = resp.text().await.context("reading sangha response")?;

        if !status.is_success() {
            bail!("sangha returned {}: {}", status, text);
        }

        let parsed: serde_json::Value =
            serde_json::from_str(&text).context("parsing sangha response")?;

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

        // sangha returns the claim result in the MCP response content
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
