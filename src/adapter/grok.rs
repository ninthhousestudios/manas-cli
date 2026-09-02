use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tokio::process::Command;
use toml_edit::{Array, DocumentMut, Item, Table, value};

use super::{HarnessAdapter, HarnessHandle};
use crate::binding::Binding;
use crate::instructions;

pub struct GrokAdapter;

impl GrokAdapter {
    fn user_home() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        PathBuf::from(home).join(".grok")
    }

    fn overlay_home(binding: &Binding) -> PathBuf {
        scratch_dir(binding).join("grok-home")
    }

    /// Grok's TUI has no `--mcp-config` / `--append-system-prompt-file`.
    /// `GROK_CONFIG` overlays cannot add MCP servers (allowlisted keys only).
    /// A session-scoped `GROK_HOME` is the remaining lever that keeps
    /// `~/.grok/config.toml` and `~/.grok/AGENTS.md` as the bare-`grok`
    /// surface — equivalent to Claude Code's split between `~/CLAUDE.md` and
    /// `--mcp-config` / `--append-system-prompt-file`.
    ///
    /// Everything except `config.toml` and `rules/` is symlinked from the
    /// user's grok home so auth, sessions, and trust survive. `config.toml`
    /// is a merge (user keys plus manas MCP). `rules/manas.md` carries the
    /// injected prompt; a matching copy lands in the session scratch dir so
    /// a stale prompt is diagnosable the same way Claude's is.
    fn write_overlay(binding: &Binding) -> Result<PathBuf> {
        let overlay = Self::overlay_home(binding);
        std::fs::create_dir_all(&overlay)?;

        let user_home = Self::user_home();
        match std::fs::read_dir(&user_home) {
            Ok(entries) => {
                for entry in entries {
                    let entry =
                        entry.with_context(|| format!("reading {}", user_home.display()))?;
                    let name = entry.file_name();
                    if skip_overlay_entry(&name) {
                        continue;
                    }
                    let dest = overlay.join(&name);
                    if dest.symlink_metadata().is_ok() {
                        continue;
                    }
                    std::os::unix::fs::symlink(entry.path(), &dest).with_context(|| {
                        format!("symlink {} -> {}", entry.path().display(), dest.display())
                    })?;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => {
                return Err(e).with_context(|| format!("reading {}", user_home.display()));
            }
        }

        let config_path = overlay.join("config.toml");
        let mut doc = read_config(&user_home.join("config.toml"))?;
        merge_mcp_servers(&mut doc, &binding.yojana_url)?;
        write_atomic(&config_path, doc.to_string().as_bytes())
            .with_context(|| format!("writing {}", config_path.display()))?;
        Ok(overlay)
    }

    fn write_instructions(binding: &Binding) -> Result<PathBuf> {
        let text = format!(
            "{}\n\n{}",
            instructions::resolve().text.trim_end(),
            instructions::resolve_harness(instructions::Harness::Grok).text
        );

        let scratch_path = scratch_dir(binding).join("manas-instructions.md");
        if let Some(parent) = scratch_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&scratch_path, &text)?;

        let rules_dir = Self::overlay_home(binding).join("rules");
        std::fs::create_dir_all(&rules_dir)?;
        copy_user_rules(&Self::user_home().join("rules"), &rules_dir)?;
        std::fs::write(rules_dir.join("manas.md"), &text)?;
        Ok(scratch_path)
    }
}

fn skip_overlay_entry(name: &std::ffi::OsStr) -> bool {
    matches!(
        name.to_str(),
        Some("config.toml" | "config.toml.lock" | "rules" | "leader.sock")
    )
}

fn copy_user_rules(src: &Path, dest: &Path) -> Result<()> {
    let entries = match std::fs::read_dir(src) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e).with_context(|| format!("reading {}", src.display())),
    };
    for entry in entries {
        let entry = entry.with_context(|| format!("reading {}", src.display()))?;
        let name = entry.file_name();
        if name == "manas.md" {
            continue;
        }
        let meta = entry
            .file_type()
            .with_context(|| format!("stat {}", entry.path().display()))?;
        if !meta.is_file() {
            continue;
        }
        let to = dest.join(&name);
        std::fs::copy(entry.path(), &to)
            .with_context(|| format!("copy {} -> {}", entry.path().display(), to.display()))?;
    }
    Ok(())
}

fn merge_mcp_servers(doc: &mut DocumentMut, yojana_url: &str) -> Result<()> {
    let servers = table_at(doc.as_table_mut(), "mcp_servers")?;
    servers.set_implicit(true);

    let yojana = table_at(servers, "yojana")?;
    yojana["url"] = value(format!("{yojana_url}/mcp"));
    yojana["enabled"] = value(true);

    let sutra = table_at(servers, "sutra")?;
    sutra["command"] = value("sutra");
    let mut args = Array::new();
    args.push("serve");
    args.push("--stdio");
    sutra["args"] = value(args);
    sutra["enabled"] = value(true);

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

fn table_at<'a>(parent: &'a mut Table, key: &str) -> Result<&'a mut Table> {
    parent
        .entry(key)
        .or_insert_with(|| Item::Table(Table::new()))
        .as_table_mut()
        .with_context(|| format!("`{key}` in the grok config is not a table"))
}

fn write_atomic(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("toml.manas-tmp");
    std::fs::write(&tmp, contents)?;
    std::fs::rename(&tmp, path)
}

#[async_trait::async_trait]
impl HarnessAdapter for GrokAdapter {
    fn name(&self) -> &'static str {
        "grok"
    }

    async fn launch(&self, binding: &Binding, prompt: Option<&str>) -> Result<HarnessHandle> {
        let grok_home =
            Self::write_overlay(binding).context("failed to write GROK_HOME overlay")?;
        Self::write_instructions(binding).context("failed to write manas instructions for Grok")?;

        let mut cmd = Command::new("grok");

        if let Some(p) = prompt {
            cmd.arg("-p").arg(p);
        }

        cmd.env("GROK_HOME", &grok_home);

        for (key, val) in binding.env_vars() {
            cmd.env(&key, &val);
        }

        cmd.current_dir(&binding.project_root);

        let child = cmd
            .spawn()
            .context("failed to spawn `grok` — is Grok CLI installed?")?;

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
            .context("waiting for grok to exit")?;
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

    const EXISTING: &str = r#"
[cli]
installer = "internal"

[ui]
yolo = false
permission_mode = "always-approve"

[mcp_servers.linear]
url = "https://mcp.linear.app/mcp"
"#;

    #[test]
    fn merge_preserves_user_keys_and_updates_ours() {
        let mut doc: DocumentMut = EXISTING.parse().expect("fixture is valid TOML");
        merge_mcp_servers(&mut doc, "http://127.0.0.1:4200").expect("merge");
        let text = doc.to_string();
        assert!(text.contains("installer = \"internal\""));
        assert!(text.contains("yolo = false"));
        assert!(text.contains("https://mcp.linear.app/mcp"));
        assert!(text.contains("http://127.0.0.1:4200/mcp"));
        assert!(text.contains("command = \"sutra\""));
    }

    #[test]
    fn merge_into_empty_config_writes_both_servers() {
        let mut doc = DocumentMut::new();
        merge_mcp_servers(&mut doc, "http://127.0.0.1:4200").expect("merge");
        let text = doc.to_string();
        assert!(text.contains("[mcp_servers.yojana]"));
        assert!(text.contains("[mcp_servers.sutra]"));
        assert!(!text.contains("[mcp_servers]\n"));
    }

    #[test]
    fn unparseable_config_is_not_overwritten() {
        let dir = std::env::temp_dir().join(format!("manas-grok-bad-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("config.toml");
        std::fs::write(&path, "this is not toml {{{").expect("write");
        let err = read_config(&path).expect_err("should refuse");
        assert!(err.to_string().contains("not valid TOML"));
    }
}
