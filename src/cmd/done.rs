use anyhow::Result;

pub async fn run() -> Result<()> {
    println!("manas done");
    println!();
    println!("  [stub] will run session shutdown:");
    println!("    1. claim sangha `handoff` lock");
    println!("    2. invoke LLM body (review session, store observations)");
    println!("    3. generate session summary → chitta");
    println!("    4. write docs/handoff.md");
    println!("    5. release lock, revoke binding");

    Ok(())
}
