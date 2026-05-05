use anyhow::Result;

pub async fn run() -> Result<()> {
    println!("manas reflect");
    println!();
    println!("  [stub] will run between-session maintenance:");
    println!("    1. claim sangha `reflect:user` lock");
    println!("    2. batch-load unconsolidated observations from chitta");
    println!("    3. batch-load active mental models from chitta");
    println!("    4. invoke LLM body (group, synthesize, retire)");
    println!("    5. write models with provenance, release lock");

    Ok(())
}
