use std::path::PathBuf;

use anyhow::{Context, Result};
use tokio::process::Command;

use super::{HarnessAdapter, HarnessHandle};
use crate::binding::Binding;
use crate::instructions;

pub struct GeminiCliAdapter;

impl GeminiCliAdapter {
    fn gemini_home() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        PathBuf::from(home).join(".manas").join("gemini")
    }

    /// Antigravity (`agy`) has no system-prompt flag and does not auto-load
    /// context files from the working directory unless that directory is an
    /// added workspace root. It *does* load `GEMINI.md`/`AGENTS.md` from every
    /// directory passed via `--add-dir`, so the manas instructions ride in as
    /// an `AGENTS.md` in a manas-owned dir added to the workspace — nothing is
    /// written into the user's repo. Verified empirically: a one-shot run with
    /// `--add-dir` pointing here surfaces the file's contents in context, while
    /// the same file left only in the cwd does not.
    fn write_instructions() -> Result<PathBuf> {
        let dir = Self::gemini_home();
        std::fs::create_dir_all(&dir)?;
        std::fs::write(
            dir.join("AGENTS.md"),
            instructions::combined_for(instructions::Harness::Gemini),
        )?;
        Ok(dir)
    }

    fn write_mcp_config(binding: &Binding) -> Result<PathBuf> {
        let config_dir = binding.project_root.join(".gemini");
        std::fs::create_dir_all(&config_dir)?;
        let config_path = config_dir.join("settings.json");

        let existing: serde_json::Value = if config_path.exists() {
            let raw = std::fs::read_to_string(&config_path)?;
            serde_json::from_str(&raw).unwrap_or_else(|_| serde_json::json!({}))
        } else {
            serde_json::json!({})
        };

        let mut config = match existing {
            serde_json::Value::Object(map) => map,
            _ => serde_json::Map::new(),
        };
        config.insert(
            "mcpServers".to_string(),
            serde_json::json!({
                "yojana": {
                    "type": "http",
                    "url": format!("{}/mcp", binding.yojana_url),
                },
                "sutra": {
                    "command": "sutra",
                    "args": ["serve", "--stdio"],
                },
            }),
        );

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
        let instructions_dir = Self::write_instructions()
            .context("failed to write manas instructions for Antigravity CLI")?;

        let mut cmd = Command::new("agy");

        // `--add-dir` makes `agy` load the `AGENTS.md` we just wrote there —
        // the only lever it exposes that keeps the file out of the user's repo.
        cmd.arg("--add-dir").arg(&instructions_dir);

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
