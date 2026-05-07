use anyhow::{bail, Result};

use crate::adapter::claude_code::ClaudeCodeAdapter;
use crate::adapter::codex::CodexCliAdapter;
use crate::adapter::gemini::GeminiCliAdapter;
use crate::adapter::opencode::OpencodeAdapter;
use crate::adapter::HarnessAdapter;
use crate::binding::Binding;
use crate::config::ManasConfig;

pub async fn run(harness: &str) -> Result<()> {
    let config = ManasConfig::load()?;

    let project_root = std::env::current_dir()?;
    let binding = Binding::new(&config, project_root);

    let adapter: Box<dyn HarnessAdapter> = match harness {
        "claude-code" | "cc" => Box::new(ClaudeCodeAdapter),
        "codex" => Box::new(CodexCliAdapter),
        "gemini" => Box::new(GeminiCliAdapter),
        "opencode" | "oc" => Box::new(OpencodeAdapter),
        _ => bail!("unknown harness: {harness} (expected: claude-code, codex, gemini, opencode)"),
    };

    println!("manas warm — booting rich session");
    println!("  session:  {}", binding.session_id);
    println!("  manas:    {}", binding.manas_url);
    println!("  chitta:   {}", binding.chitta_url);
    println!("  yojana:   {}", binding.yojana_url);
    println!("  sangha:   {}", binding.sangha_url);
    println!("  smriti:   {}", binding.smriti_url);
    println!("  project:  {}", binding.project_root.display());
    println!("  adapter:  {}", adapter.name());
    println!();

    let mut handle = adapter.launch(&binding, None).await?;

    let status = handle.child.wait().await?;

    if !status.success() {
        bail!("harness exited with {}", status);
    }

    Ok(())
}
