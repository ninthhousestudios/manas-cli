use std::path::PathBuf;

use anyhow::{Context, Result};
use tokio::process::Command;

use super::{HarnessAdapter, HarnessHandle};
use crate::binding::Binding;
use crate::instructions;

pub struct OpencodeAdapter;

impl OpencodeAdapter {
    fn opencode_home() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        PathBuf::from(home).join(".manas").join("opencode")
    }

    /// Write the manas config and instructions into a manas-owned dir and hand
    /// opencode its path via `OPENCODE_CONFIG`. That env var *merges* onto the
    /// user's global config rather than replacing it (verified: `opencode debug
    /// config` shows global providers/auth surviving), so we only declare what
    /// manas owns and inherit the rest.
    ///
    /// The previous implementation wrote `opencode.json` into the user's repo,
    /// clobbering any hand-written project config and littering their working
    /// tree — moving it here keeps `binding.project_root` untouched.
    ///
    /// opencode has no system-prompt flag; its `instructions` field takes a list
    /// of files whose contents are folded into the system prompt. An absolute
    /// path to a manas-owned `AGENTS.md` is the injection lever — verified with
    /// a one-shot run that reported the file's sentinel back from context.
    fn write_config(binding: &Binding) -> Result<PathBuf> {
        let dir = Self::opencode_home();
        std::fs::create_dir_all(&dir)?;

        let instructions_path = dir.join("AGENTS.md");
        std::fs::write(
            &instructions_path,
            instructions::combined_for(instructions::Harness::Opencode),
        )?;

        let config_path = dir.join("opencode.json");
        let config = serde_json::json!({
            "$schema": "https://opencode.ai/config.json",
            "instructions": [instructions_path.to_string_lossy()],
            "mcp": {
                "yojana": {
                    "type": "remote",
                    "url": format!("{}/mcp", binding.yojana_url),
                },
                "sutra": {
                    "type": "local",
                    "command": ["sutra", "serve", "--stdio"],
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
        let config_path =
            Self::write_config(binding).context("failed to write config for opencode")?;

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
        handle
            .child
            .wait()
            .await
            .context("waiting for opencode to exit")?;
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
