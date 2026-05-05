use std::path::PathBuf;

use anyhow::{Context, Result};

pub struct ManasConfig {
    pub manas_dir: PathBuf,
    pub mcpjungle_url: String,
}

#[allow(dead_code)]
impl ManasConfig {
    pub fn load() -> Result<Self> {
        let home = std::env::var("HOME").context("HOME not set")?;
        let manas_dir = PathBuf::from(&home).join(".manas");
        let mcpjungle_url = std::env::var("MANAS_MCPJUNGLE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:8080".to_string());

        Ok(Self {
            manas_dir,
            mcpjungle_url,
        })
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
