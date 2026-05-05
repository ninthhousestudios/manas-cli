use anyhow::{bail, Result};

use crate::adapter::claude_code::ClaudeCodeAdapter;
use crate::adapter::HarnessAdapter;
use crate::binding::{Binding, BootMode};
use crate::config::ManasConfig;

pub async fn run() -> Result<()> {
    let config = ManasConfig::load()?;

    let project_root = std::env::current_dir()?;
    let binding = Binding::new(BootMode::Rich, &config.mcpjungle_url, project_root);

    println!("manas warm — booting rich session");
    println!("  session:  {}", binding.session_id);
    println!("  endpoint: {}", binding.mcp_endpoint);
    println!("  project:  {}", binding.project_root.display());

    // TODO: health gate — verify mcpjungle + required subsystems are up
    // TODO: mint session token via mcpjungle admin API
    // TODO: record binding in ~/.manas/bindings.log

    let adapter = ClaudeCodeAdapter;
    println!("  adapter:  {}", adapter.name());
    println!();

    let mut handle = adapter.launch(&binding, None).await?;

    let status = handle.child.wait().await?;

    if !status.success() {
        bail!("harness exited with {}", status);
    }

    // TODO: revoke token, release sangha resources, mark binding row

    Ok(())
}
