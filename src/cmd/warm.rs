use anyhow::{bail, Result};

use crate::adapter::claude_code::ClaudeCodeAdapter;
use crate::adapter::HarnessAdapter;
use crate::binding::Binding;
use crate::config::ManasConfig;

pub async fn run() -> Result<()> {
    let config = ManasConfig::load()?;

    let project_root = std::env::current_dir()?;
    let binding = Binding::new(&config, project_root);

    println!("manas warm — booting rich session");
    println!("  session:  {}", binding.session_id);
    println!("  manas:    {}", binding.manas_url);
    println!("  chitta:   {}", binding.chitta_url);
    println!("  yojana:   {}", binding.yojana_url);
    println!("  project:  {}", binding.project_root.display());

    let adapter = ClaudeCodeAdapter;
    println!("  adapter:  {}", adapter.name());
    println!();

    let mut handle = adapter.launch(&binding, None).await?;

    let status = handle.child.wait().await?;

    if !status.success() {
        bail!("harness exited with {}", status);
    }

    Ok(())
}
