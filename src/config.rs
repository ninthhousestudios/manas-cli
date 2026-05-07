use std::path::PathBuf;

use anyhow::{Context, Result};

pub struct ManasConfig {
    pub manas_dir: PathBuf,
    pub chitta_url: String,
    pub yojana_url: String,
    pub sangha_url: String,
    pub smriti_url: String,
    pub sutra_url: String,
    pub serve_port: u16,
}

#[allow(dead_code)]
impl ManasConfig {
    pub fn load() -> Result<Self> {
        let home = std::env::var("HOME").context("HOME not set")?;
        let manas_dir = PathBuf::from(&home).join(".manas");
        let chitta_url = std::env::var("MANAS_CHITTA_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:3100".to_string());
        let yojana_url = std::env::var("MANAS_YOJANA_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:4200".to_string());
        let sangha_url = std::env::var("MANAS_SANGHA_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:3200".to_string());
        let smriti_url = std::env::var("MANAS_SMRITI_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:7333".to_string());
        let sutra_url = std::env::var("MANAS_SUTRA_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:3201".to_string());
        let serve_port = std::env::var("MANAS_SERVE_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(3000);

        Ok(Self {
            manas_dir,
            chitta_url,
            yojana_url,
            sangha_url,
            smriti_url,
            sutra_url,
            serve_port,
        })
    }

    pub fn serve_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.serve_port)
    }

    pub fn admin_token(&self) -> Result<Option<String>> {
        if let Ok(token) = std::env::var("MANAS_ADMIN_TOKEN") {
            return Ok(Some(token));
        }

        let token_path = self.manas_dir.join("admin-token");
        match std::fs::read_to_string(&token_path) {
            Ok(token) => Ok(Some(token.trim().to_string())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e).context("failed to read admin token"),
        }
    }

    pub fn sessions_dir(&self) -> PathBuf {
        self.manas_dir.join("sessions")
    }

    pub fn bindings_log(&self) -> PathBuf {
        self.manas_dir.join("bindings.log")
    }
}
