use std::path::{Path, PathBuf};
use std::process::Stdio;

use anyhow::{Context, Result};
use tokio::process::Command;
use toml_edit::{Array, DocumentMut, Item, Table, value};

use super::{HarnessAdapter, HarnessHandle};
use crate::binding::Binding;
use crate::instructions;

pub struct CodexCliAdapter;

impl CodexCliAdapter {
    fn codex_home() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        PathBuf::from(home).join(".manas").join("codex")
    }

    /// Splice the manas MCP servers into `$CODEX_HOME/config.toml`, leaving
    /// every other key alone.
    ///
    /// Codex owns this file too — it persists the model, reasoning effort,
    /// per-tool approval modes and project trust levels here — so the write has
    /// to be a merge. Rewriting the file wholesale silently discarded all of
    /// that on every launch.
    fn write_mcp_config(binding: &Binding) -> Result<PathBuf> {
        let codex_home = Self::codex_home();
        std::fs::create_dir_all(&codex_home)?;
        let config_path = codex_home.join("config.toml");

        let mut doc = read_config(&config_path)?;
        merge_mcp_servers(&mut doc, &binding.yojana_url)?;

        write_atomic(&config_path, doc.to_string().as_bytes())
            .with_context(|| format!("writing {}", config_path.display()))?;
        Ok(config_path)
    }

    /// Codex reads its global conventions from `$CODEX_HOME/AGENTS.md`, and
    /// nothing else writes that path — `CODEX_HOME` points inside `~/.manas`,
    /// so unlike `config.toml` this one is ours to overwrite.
    fn write_instructions() -> Result<PathBuf> {
        let codex_home = Self::codex_home();
        std::fs::create_dir_all(&codex_home)?;
        let path = codex_home.join("AGENTS.md");
        std::fs::write(&path, instructions::combined())?;
        Ok(path)
    }
}

/// Set only the keys manas owns, in place. Sibling keys under
/// `[mcp_servers.yojana]` — per-tool `approval_mode` tables, say — are left
/// standing.
fn merge_mcp_servers(doc: &mut DocumentMut, yojana_url: &str) -> Result<()> {
    let servers = table_at(doc.as_table_mut(), "mcp_servers")?;
    // Only `[mcp_servers.<name>]` headers get emitted; a bare `[mcp_servers]`
    // would be noise in a file the user also edits.
    servers.set_implicit(true);

    table_at(servers, "yojana")?["url"] = value(format!("{yojana_url}/mcp"));

    let sutra = table_at(servers, "sutra")?;
    sutra["command"] = value("sutra");
    let mut args = Array::new();
    args.push("serve");
    args.push("--stdio");
    sutra["args"] = value(args);

    Ok(())
}

fn read_config(path: &Path) -> Result<DocumentMut> {
    match std::fs::read_to_string(path) {
        Ok(existing) => existing.parse::<DocumentMut>().with_context(|| {
            format!(
                "{} is not valid TOML — refusing to overwrite it",
                path.display()
            )
        }),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(DocumentMut::new()),
        Err(e) => Err(e).with_context(|| format!("reading {}", path.display())),
    }
}

/// Borrow `parent[key]` as a table, creating it if absent. Errors rather than
/// replacing it if the user has that key as something else, on the same
/// principle as refusing to overwrite unparseable TOML.
fn table_at<'a>(parent: &'a mut Table, key: &str) -> Result<&'a mut Table> {
    parent
        .entry(key)
        .or_insert_with(|| Item::Table(Table::new()))
        .as_table_mut()
        .with_context(|| format!("`{key}` in the codex config is not a table"))
}

/// Write through a temp file so an interrupted write cannot leave a truncated
/// config behind — the failure mode this whole function exists to prevent.
fn write_atomic(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("toml.manas-tmp");
    std::fs::write(&tmp, contents)?;
    std::fs::rename(&tmp, path)
}

#[async_trait::async_trait]
impl HarnessAdapter for CodexCliAdapter {
    fn name(&self) -> &'static str {
        "codex"
    }

    async fn launch(&self, binding: &Binding, prompt: Option<&str>) -> Result<HarnessHandle> {
        Self::write_mcp_config(binding).context("failed to write MCP config for Codex CLI")?;
        Self::write_instructions().context("failed to write manas instructions for Codex CLI")?;

        let mut cmd = Command::new("codex");

        if let Some(p) = prompt {
            cmd.arg("exec").arg(p);
            cmd.stdin(Stdio::null());
        }

        cmd.env("CODEX_HOME", Self::codex_home());

        for (key, val) in binding.env_vars() {
            cmd.env(&key, &val);
        }

        cmd.current_dir(&binding.project_root);

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
        handle
            .child
            .wait()
            .await
            .context("waiting for codex to exit")?;
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Shaped after a real `~/.manas/codex/config.toml`: everything here was
    /// destroyed by the old wholesale write.
    const EXISTING: &str = r#"model = "gpt-5.5"
model_reasoning_effort = "high"

[mcp_servers.yojana]
url = "http://127.0.0.1:9999/mcp"

[mcp_servers.yojana.tools.yojana_task]
approval_mode = "approve"

[projects."/home/josh/soft/manas/sutra"]
trust_level = "trusted"
"#;

    #[test]
    fn merge_preserves_user_keys_and_updates_ours() {
        let mut doc = EXISTING.parse::<DocumentMut>().expect("fixture parses");
        merge_mcp_servers(&mut doc, "http://127.0.0.1:4200").expect("merge succeeds");
        let out = doc.to_string();

        assert!(out.contains(r#"model = "gpt-5.5""#));
        assert!(out.contains(r#"model_reasoning_effort = "high""#));
        assert!(out.contains("[mcp_servers.yojana.tools.yojana_task]"));
        assert!(out.contains(r#"trust_level = "trusted""#));

        assert!(out.contains(r#"url = "http://127.0.0.1:4200/mcp""#));
        assert!(!out.contains("9999"));
        assert!(out.contains(r#"command = "sutra""#));
        assert!(out.contains(r#"args = ["serve", "--stdio"]"#));
        // A bare `[mcp_servers]` header would mean we un-implicited the parent.
        assert!(!out.contains("\n[mcp_servers]"));
    }

    #[test]
    fn merge_into_empty_config_writes_both_servers() {
        let mut doc = DocumentMut::new();
        merge_mcp_servers(&mut doc, "http://127.0.0.1:4200").expect("merge succeeds");
        let out = doc.to_string();

        assert!(out.contains("[mcp_servers.yojana]"));
        assert!(out.contains("[mcp_servers.sutra]"));
        assert!(out.parse::<DocumentMut>().is_ok());
    }

    #[test]
    fn unparseable_config_is_not_overwritten() {
        let dir = std::env::temp_dir().join(format!("manas-codex-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("config.toml");
        std::fs::write(&path, "this is not = = toml\n").expect("write fixture");

        assert!(read_config(&path).is_err());
        assert_eq!(
            std::fs::read_to_string(&path).expect("fixture still there"),
            "this is not = = toml\n"
        );
    }
}
