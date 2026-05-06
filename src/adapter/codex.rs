use std::path::PathBuf;
use std::process::Stdio;

use anyhow::{Context, Result};
use tokio::process::Command;

use super::{HarnessAdapter, HarnessHandle};
use crate::binding::Binding;

pub struct CodexCliAdapter;

impl CodexCliAdapter {
    fn codex_home(binding: &Binding) -> PathBuf {
        scratch_dir(binding).join("codex")
    }

    fn write_mcp_config(binding: &Binding) -> Result<PathBuf> {
        let codex_home = Self::codex_home(binding);
        std::fs::create_dir_all(&codex_home)?;
        let config_path = codex_home.join("config.toml");

        let toml = format!(
            "[mcp_servers.manas]\nurl = \"{}/mcp\"\n\n\
             [mcp_servers.chitta]\nurl = \"{}/mcp\"\n\n\
             [mcp_servers.yojana]\nurl = \"{}/mcp\"\n",
            binding.manas_url, binding.chitta_url, binding.yojana_url,
        );

        std::fs::write(&config_path, &toml)?;
        Ok(config_path)
    }
}

#[async_trait::async_trait]
impl HarnessAdapter for CodexCliAdapter {
    fn name(&self) -> &'static str {
        "codex"
    }

    async fn launch(&self, binding: &Binding, prompt: Option<&str>) -> Result<HarnessHandle> {
        Self::write_mcp_config(binding)
            .context("failed to write MCP config for Codex CLI")?;

        let mut cmd = Command::new("codex");
        cmd.arg("exec");

        if let Some(p) = prompt {
            cmd.arg(p);
        }

        cmd.arg("--skip-git-repo-check")
            .arg("--dangerously-bypass-approvals-and-sandbox");

        cmd.env("CODEX_HOME", Self::codex_home(binding));

        for (key, val) in binding.env_vars() {
            cmd.env(&key, &val);
        }

        cmd.current_dir(&binding.project_root);
        cmd.stdin(Stdio::null());

        let child = cmd
            .spawn()
            .context("failed to spawn `codex` — is Codex CLI installed?")?;

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
        handle.child.wait().await.context("waiting for codex to exit")?;
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
