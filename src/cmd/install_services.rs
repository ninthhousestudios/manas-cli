use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use tokio::process::Command;

use crate::config::ManasConfig;

const MANAS_SERVICE: &str = "manas.service";

pub async fn run() -> Result<()> {
    let config = ManasConfig::load()?;
    let unit_dir = systemd_user_dir(&config)?;
    std::fs::create_dir_all(&unit_dir).context("failed to create systemd user unit directory")?;

    let unit_path = unit_dir.join(MANAS_SERVICE);
    let exe = std::env::current_exe().context("failed to resolve current executable")?;
    let unit = manas_service_unit(&exe, config.serve_port);

    std::fs::write(&unit_path, unit)
        .with_context(|| format!("failed to write {}", unit_path.display()))?;

    run_systemctl(["daemon-reload"]).await?;
    run_systemctl(["enable", "--now", MANAS_SERVICE]).await?;

    println!("installed and enabled {}", unit_path.display());
    println!("service: systemctl --user status {MANAS_SERVICE}");
    Ok(())
}

fn systemd_user_dir(config: &ManasConfig) -> Result<PathBuf> {
    if let Ok(dir) = std::env::var("XDG_CONFIG_HOME") {
        return Ok(PathBuf::from(dir).join("systemd/user"));
    }

    let home = std::env::var("HOME").context("HOME not set")?;
    let home = if home.is_empty() {
        config
            .manas_dir
            .parent()
            .map(Path::to_path_buf)
            .context("HOME not set and manas_dir has no parent")?
    } else {
        PathBuf::from(home)
    };
    Ok(home.join(".config/systemd/user"))
}

fn manas_service_unit(exe: &Path, port: u16) -> String {
    format!(
        r#"[Unit]
Description=Manas MCP hub
After=network.target

[Service]
Type=simple
ExecStart={} serve --port {}
Restart=on-failure
RestartSec=2

[Install]
WantedBy=default.target
"#,
        systemd_quote(exe),
        port
    )
}

fn systemd_quote(path: &Path) -> String {
    let raw = path.to_string_lossy();
    let escaped = raw.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

async fn run_systemctl<const N: usize>(args: [&str; N]) -> Result<()> {
    let status = Command::new("systemctl")
        .arg("--user")
        .args(args)
        .status()
        .await
        .context("failed to run systemctl --user")?;

    if !status.success() {
        bail!("systemctl --user exited with {status}");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_runs_manas_serve_on_configured_port() {
        let unit = manas_service_unit(Path::new("/tmp/manas"), 3000);

        assert!(unit.contains("ExecStart=\"/tmp/manas\" serve --port 3000"));
        assert!(unit.contains("WantedBy=default.target"));
        assert!(unit.contains("Restart=on-failure"));
    }
}
