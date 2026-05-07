use std::path::PathBuf;

use anyhow::{Context, Result};
use tokio::process::Command;

use super::{HarnessAdapter, HarnessHandle};
use crate::binding::Binding;

pub struct OpencodeAdapter;

impl OpencodeAdapter {
    fn write_mcp_config(binding: &Binding) -> Result<PathBuf> {
        let config_path = binding.project_root.join("opencode.json");

        let config = serde_json::json!({
            "mcp": {
                "manas": {
                    "type": "remote",
                    "url": format!("{}/mcp", binding.manas_url),
                },
                "chitta": {
                    "type": "remote",
                    "url": format!("{}/mcp", binding.chitta_url),
                },
                "yojana": {
                    "type": "remote",
                    "url": format!("{}/mcp", binding.yojana_url),
                },
                "sangha": {
                    "type": "remote",
                    "url": format!("{}/mcp", binding.sangha_url),
                },
                "smriti": {
                    "type": "remote",
                    "url": format!("{}/mcp", binding.smriti_url),
                },
            }
        });

        std::fs::write(&config_path, serde_json::to_string_pretty(&config)?)?;
        Ok(config_path)
    }
}

#[async_trait::async_trait]
impl HarnessAdapter for OpencodeAdapter {
    fn name(&self) -> &'static str {
        "opencode"
    }

    async fn launch(&self, binding: &Binding, prompt: Option<&str>) -> Result<HarnessHandle> {
        let config_path = Self::write_mcp_config(binding)
            .context("failed to write MCP config for opencode")?;

        let mut cmd = Command::new("opencode");

        if let Some(p) = prompt {
            cmd.arg("run").arg(p);
        }

        cmd.env("OPENCODE_CONFIG", &config_path);

        for (key, val) in binding.env_vars() {
            cmd.env(&key, &val);
        }

        cmd.current_dir(&binding.project_root);

        let child = cmd
            .spawn()
            .context("failed to spawn `opencode` — is opencode installed?")?;

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
        handle.child.wait().await.context("waiting for opencode to exit")?;
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
