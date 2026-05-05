use anyhow::Result;

use crate::config::ManasConfig;

pub async fn run() -> Result<()> {
    let config = ManasConfig::load()?;

    println!("manas health\n");
    println!("  mcpjungle: {}", config.mcpjungle_url);

    match config.admin_token()? {
        Some(_) => println!("  admin token: configured"),
        None => {
            println!("  admin token: NOT FOUND");
            println!("    run `manas init` to configure, or set MANAS_ADMIN_TOKEN");
        }
    }

    // Dev-mode detection: unauthenticated request to mcpjungle succeeds = no ACL.
    // When implemented, this will:
    //   1. GET {mcpjungle_url}/api/v0/health without auth
    //   2. If 200 with full tool surface → dev mode → loud warning
    //   3. If 401 → enterprise mode → check admin token validity
    println!("\n  WARNING: health checks not yet implemented");
    println!("  (will detect mcpjungle dev mode and subsystem liveness)");

    Ok(())
}
