pub mod claude_code;
pub mod codex;
pub mod gemini;
pub mod opencode;

use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::binding::Binding;

pub fn chitta_token() -> Result<Option<String>> {
    if let Ok(token) = std::env::var("CHITTA_TOKEN") {
        return Ok(Some(token));
    }

    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    let candidates = [
        PathBuf::from(&home)
            .join(".chitta")
            .join("bearer-token.txt"),
        PathBuf::from(&home)
            .join(".config")
            .join("chitta")
            .join("bearer-token.txt"),
    ];

    for path in candidates {
        match std::fs::read_to_string(&path) {
            Ok(token) => return Ok(Some(token.trim().to_string())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => {
                return Err(e).with_context(|| {
                    format!("failed to read chitta token from {}", path.display())
                });
            }
        }
    }

    Ok(None)
}

#[allow(dead_code)]
pub struct HarnessHandle {
    pub child: tokio::process::Child,
    pub transcript_path: Option<PathBuf>,
    pub scratch_dir: PathBuf,
}

#[allow(dead_code)]
#[async_trait::async_trait]
pub trait HarnessAdapter: Send + Sync {
    fn name(&self) -> &'static str;

    async fn launch(&self, binding: &Binding, prompt: Option<&str>) -> Result<HarnessHandle>;

    fn transcript_path(&self, binding: &Binding) -> Option<PathBuf>;

    async fn shutdown(&self, handle: &mut HarnessHandle) -> Result<()>;
}
