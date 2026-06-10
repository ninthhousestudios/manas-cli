use anyhow::{Result, bail};

use crate::adapter::claude_code::ClaudeCodeAdapter;
use crate::adapter::codex::CodexCliAdapter;
use crate::adapter::gemini::GeminiCliAdapter;
use crate::adapter::opencode::OpencodeAdapter;
use crate::adapter::{self, HarnessAdapter};
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
    println!("  chitta:   {}", binding.chitta_url);
    println!("  yojana:   {}", binding.yojana_url);
    println!("  smriti:   {}", binding.smriti_url);
    println!("  project:  {}", binding.project_root.display());
    println!("  adapter:  {}", adapter.name());

    // Pre-warm chitta's embedding model while the adapter sets up.
    let chitta_url = binding.chitta_url.clone();
    let warm_handle = tokio::spawn(async move {
        warm_chitta_model(&chitta_url).await;
    });

    let mut handle = adapter.launch(&binding, None).await?;

    // Best-effort: don't block on the warm if the adapter launched fast.
    let _ = warm_handle.await;

    let status = handle.child.wait().await?;

    if !status.success() {
        bail!("harness exited with {}", status);
    }

    Ok(())
}

async fn warm_chitta_model(chitta_url: &str) {
    let token = match adapter::chitta_token() {
        Ok(Some(t)) => t,
        _ => {
            eprintln!("  model:    skipped (no chitta token)");
            return;
        }
    };

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
    {
        Ok(c) => c,
        Err(_) => return,
    };

    match client
        .post(format!("{chitta_url}/model/warm"))
        .bearer_auth(&token)
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            println!("  model:    warm");
        }
        Ok(resp) => {
            eprintln!("  model:    warm failed ({})", resp.status());
        }
        Err(e) => {
            eprintln!("  model:    warm failed ({e})");
        }
    }
}
