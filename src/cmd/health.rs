use anyhow::Result;

use crate::config::ManasConfig;

pub async fn run() -> Result<()> {
    let config = ManasConfig::load()?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()?;

    println!("manas health\n");

    for (name, url) in [
        ("chitta", &config.chitta_url),
        ("yojana", &config.yojana_url),
        ("sangha", &config.sangha_url),
        ("smriti", &config.smriti_url),
    ] {
        let status = match client.get(format!("{url}/health")).send().await {
            Ok(resp) if resp.status().is_success() => "ok".to_string(),
            Ok(resp) => format!("unhealthy ({})", resp.status()),
            Err(e) => format!("unreachable ({})", e),
        };
        println!("  {name}: {status}");
    }

    Ok(())
}
