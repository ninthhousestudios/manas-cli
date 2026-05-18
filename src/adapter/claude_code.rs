use std::path::PathBuf;

use anyhow::{Context, Result};
use tokio::process::Command;

use super::{HarnessAdapter, HarnessHandle, chitta_token};
use crate::binding::Binding;

pub struct ClaudeCodeAdapter;

const MANAS_INSTRUCTIONS: &str = include_str!("manas-instructions.md");

impl ClaudeCodeAdapter {
    fn mcp_config_path(binding: &Binding) -> PathBuf {
        scratch_dir(binding).join("mcp.json")
    }

    fn instructions_path(binding: &Binding) -> PathBuf {
        scratch_dir(binding).join("manas-instructions.md")
    }

    fn write_mcp_config(binding: &Binding) -> Result<PathBuf> {
        let config_path = Self::mcp_config_path(binding);
        std::fs::create_dir_all(config_path.parent().unwrap())?;

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
            }
        });

        std::fs::write(&config_path, serde_json::to_string_pretty(&config)?)?;
        Ok(config_path)
    }

    fn write_instructions(binding: &Binding) -> Result<PathBuf> {
        let path = Self::instructions_path(binding);
        std::fs::create_dir_all(path.parent().unwrap())?;
        std::fs::write(&path, MANAS_INSTRUCTIONS)?;
        Ok(path)
    }
}

#[async_trait::async_trait]
impl HarnessAdapter for ClaudeCodeAdapter {
    fn name(&self) -> &'static str {
        "claude-code"
    }

    async fn launch(&self, binding: &Binding, prompt: Option<&str>) -> Result<HarnessHandle> {
        let mcp_config = Self::write_mcp_config(binding)
            .context("failed to write MCP config for Claude Code")?;
        let instructions = Self::write_instructions(binding)
            .context("failed to write manas instructions for Claude Code")?;

        let mut cmd = Command::new("claude");

        if let Some(p) = prompt {
            cmd.arg("-p").arg(p);
        }

        cmd.arg("--mcp-config")
            .arg(&mcp_config)
            .arg("--strict-mcp-config")
            .arg("--append-system-prompt-file")
            .arg(&instructions);

        for (key, val) in binding.env_vars() {
            cmd.env(&key, &val);
        }

        cmd.current_dir(&binding.project_root);

        let child = cmd
            .spawn()
            .context("failed to spawn `claude` — is Claude Code installed?")?;

        let transcript_path = self.transcript_path(binding);

        Ok(HarnessHandle {
            child,
            transcript_path,
            scratch_dir: scratch_dir(binding),
        })
    }

    fn transcript_path(&self, binding: &Binding) -> Option<PathBuf> {
        let home = std::env::var("HOME").ok()?;
        let project_hash = format!(
            "{:x}",
            md5_hash(binding.project_root.to_string_lossy().as_bytes())
        );
        Some(
            PathBuf::from(home)
                .join(".claude")
                .join("projects")
                .join(project_hash)
                .join(format!("{}.jsonl", binding.session_id)),
        )
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
            .context("waiting for claude to exit")?;
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

fn md5_hash(data: &[u8]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    data.hash(&mut hasher);
    hasher.finish()
}
