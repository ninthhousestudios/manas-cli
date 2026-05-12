use std::path::PathBuf;

use anyhow::{Context, Result};
use tokio::process::Command;

use super::{chitta_token, HarnessAdapter, HarnessHandle};
use crate::binding::Binding;

pub struct GeminiCliAdapter;

impl GeminiCliAdapter {
    fn write_mcp_config(binding: &Binding) -> Result<PathBuf> {
        let config_dir = scratch_dir(binding).join(".gemini");
        std::fs::create_dir_all(&config_dir)?;
        let config_path = config_dir.join("settings.json");

        let mut chitta_entry = serde_json::json!({
            "type": "http",
            "url": format!("{}/mcp", binding.chitta_url),
        });
        if let Some(token) = chitta_token()? {
            chitta_entry["headers"] = serde_json::json!({
                "Authorization": format!("Bearer {}", token),
            });
        }

        let config = serde_json::json!({
            "mcpServers": {
                "manas": {
                    "type": "http",
                    "url": format!("{}/mcp", binding.manas_url),
                },
                "chitta": chitta_entry,
                "yojana": {
                    "type": "http",
                    "url": format!("{}/mcp", binding.yojana_url),
                },
                "sangha": {
                    "type": "http",
                    "url": format!("{}/mcp", binding.sangha_url),
                },
                "smriti": {
                    "type": "http",
                    "url": format!("{}/mcp", binding.smriti_url),
                },
            }
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
            .context("failed to write MCP config for Gemini CLI")?;

        let mut cmd = Command::new("gemini");

        if let Some(p) = prompt {
            cmd.arg("-p").arg(p);
        }

        cmd.arg("--yolo");

        cmd.env("GEMINI_CLI_TRUST_WORKSPACE", "true");

        for (key, val) in binding.env_vars() {
            cmd.env(&key, &val);
        }

        let scratch = scratch_dir(binding);
        cmd.current_dir(&scratch);

        let child = cmd
            .spawn()
            .context("failed to spawn `gemini` — is Gemini CLI installed?")?;

        Ok(HarnessHandle {
            child,
            transcript_path: None,
            scratch_dir: scratch,
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
        handle.child.wait().await.context("waiting for gemini to exit")?;
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
