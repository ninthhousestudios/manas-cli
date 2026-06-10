use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::adapter::HarnessAdapter;
use crate::adapter::claude_code::ClaudeCodeAdapter;
use crate::binding::Binding;
use crate::config::ManasConfig;
use crate::skill::lock::{LockScope, SanghaLockClient};
use crate::skill::{SkillDef, SkillShell};

static SKILL_PROMPT: &str = include_str!("../../skills/done.md");

pub async fn run() -> Result<()> {
    let config = ManasConfig::load()?;
    let project_root = std::env::current_dir().context("cannot determine project root")?;

    let mut binding = Binding::new(&config, project_root.clone());

    let transcript_path = resolve_transcript_path(&binding);
    binding.transcript_path = transcript_path;

    let lock_client =
        SanghaLockClient::new(&config.sangha_url, &project_root.display().to_string());
    let shell = SkillShell::new(lock_client);

    let skill = SkillDef {
        name: "done".into(),
        lock_resource: "handoff".into(),
        lock_scope: LockScope::Project,
        lock_ttl_secs: 300,
        prompt: SKILL_PROMPT.into(),
        output_paths: vec![PathBuf::from("docs/handoff.md")],
    };

    let output = shell.run(&skill, &ClaudeCodeAdapter, &binding).await?;

    if !output.exit_success {
        eprintln!("warning: skill body exited with non-zero status");
    }

    if !output.stdout.is_empty() {
        print!("{}", output.stdout);
    }

    Ok(())
}

fn resolve_transcript_path(binding: &Binding) -> Option<PathBuf> {
    // 1. Explicit env var from parent session
    if let Ok(path) = std::env::var("MANAS_TRANSCRIPT_PATH") {
        let p = PathBuf::from(&path);
        if p.exists() {
            return Some(p);
        }
    }

    // 2. Auto-detect via adapter (most recent transcript for this project)
    let adapter = ClaudeCodeAdapter;
    if let Some(dir) = adapter
        .transcript_path(binding)
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
    {
        if dir.exists() {
            if let Ok(mut entries) = std::fs::read_dir(&dir) {
                let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
                while let Some(Ok(entry)) = entries.next() {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                        if let Ok(meta) = path.metadata() {
                            if let Ok(modified) = meta.modified() {
                                if newest.as_ref().map_or(true, |(t, _)| modified > *t) {
                                    newest = Some((modified, path));
                                }
                            }
                        }
                    }
                }
                return newest.map(|(_, p)| p);
            }
        }
    }

    None
}
