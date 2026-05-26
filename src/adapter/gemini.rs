use std::path::PathBuf;

use anyhow::{Context, Result};
use tokio::process::Command;

use super::{HarnessAdapter, HarnessHandle, chitta_token};
use crate::binding::Binding;

pub struct GeminiCliAdapter;

impl GeminiCliAdapter {
    fn write_mcp_config(binding: &Binding) -> Result<PathBuf> {
        let config_dir = binding.project_root.join(".gemini");
        std::fs::create_dir_all(&config_dir)?;
        let config_path = config_dir.join("settings.json");

        let mut config: serde_json::Value = if config_path.exists() {
            let existing = std::fs::read_to_string(&config_path)?;
            serde_json::from_str(&existing).unwrap_or_else(|_| serde_json::json!({}))
        } else {
            serde_json::json!({})
        };

        let mut chitta_entry = serde_json::json!({
            "type": "http",
            "url": format!("{}/mcp", binding.chitta_url),
        });
        if let Some(token) = chitta_token()? {
            chitta_entry["headers"] = serde_json::json!({
                "Authorization": format!("Bearer {}", token),
            });
        }

        config["mcpServers"] = serde_json::json!({
            "chitta": chitta_entry,
            "yojana": {
                "type": "http",
                "url": format!("{}/mcp", binding.yojana_url),
            },
            "smriti": {
                "type": "http",
                "url": format!("{}/mcp", binding.smriti_url),
            },
            "sutra": {
                "command": "sutra",
                "args": ["serve", "--stdio"],
            },
        });

        std::fs::write(&config_path, serde_json::to_string_pretty(&config)?)?;
        Ok(config_path)
    }
}

#[async_trait::async_trait]
impl HarnessAdapter for GeminiCliAdapter {
    fn name(&self) -> &'static str {
        "gemini"
    }

    async fn launch(&self, binding: &Binding, prompt: Option<&str>) -> Result<HarnessHandle> {
        Self::write_mcp_config(binding)
            .context("failed to write MCP config for Antigravity CLI")?;

        let mut cmd = Command::new("agy");

        if let Some(p) = prompt {
            cmd.arg("-p").arg(p);
        }

        for (key, val) in binding.env_vars() {
            cmd.env(&key, &val);
        }

        cmd.current_dir(&binding.project_root);

        let child = cmd
            .spawn()
            .context("failed to spawn `agy` — is Antigravity CLI installed?")?;

        Ok(HarnessHandle {
            child,
            transcript_path: None,
            scratch_dir: scratch_dir(binding),
        })
    }

    fn transcript_path(&self, _binding: &Binding) -> Option<PathBuf> {
        None
    }

    async fn shutdown(&self, handle: &mut HarnessHandle) -> Result<()> {
        if let Some(id) = handle.child.id() {
            unsafe {
                libc::kill(id as i32, libc::SIGTERM);
            }
        }
        handle
            .child
            .wait()
            .await
            .context("waiting for agy to exit")?;
        Ok(())
    }
}

fn scratch_dir(binding: &Binding) -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(home)
        .join(".manas")
        .join("sessions")
        .join(binding.session_id.to_string())
}
