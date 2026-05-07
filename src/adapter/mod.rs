pub mod claude_code;
pub mod codex;
pub mod gemini;
pub mod opencode;

use std::path::PathBuf;

use anyhow::Result;

use crate::binding::Binding;

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
