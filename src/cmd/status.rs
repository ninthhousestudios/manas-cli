use anyhow::Result;

pub async fn run() -> Result<()> {
    println!("manas status");
    println!();
    println!("  [stub] will show:");
    println!("    - active sessions (from ~/.manas/bindings.log)");
    println!("    - sangha session list");
    println!("    - held resource locks");
    println!("    - orphaned bindings (missing pid)");

    Ok(())
}
