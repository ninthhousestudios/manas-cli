use anyhow::Result;

pub async fn run() -> Result<()> {
    println!("manas warm");
    println!();
    println!("  [stub] will boot a rich session:");
    println!("    1. health gate (mcpjungle + required subsystems)");
    println!("    2. mint session token (Tool Group = full)");
    println!("    3. write binding env + harness config");
    println!("    4. launch harness");
    println!("    5. teardown on exit (revoke token, release locks)");

    Ok(())
}
