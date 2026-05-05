use std::path::PathBuf;

use uuid::Uuid;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum BootMode {
    Minimal,
    Rich,
}

impl BootMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Minimal => "minimal",
            Self::Rich => "rich",
        }
    }

    pub fn tool_group(&self) -> &'static str {
        match self {
            Self::Minimal => "code",
            Self::Rich => "full",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Binding {
    pub session_id: Uuid,
    pub mode: BootMode,
    pub mcp_endpoint: String,
    pub mcp_token: Option<String>,
    pub project_root: PathBuf,
}

impl Binding {
    pub fn new(mode: BootMode, mcpjungle_url: &str, project_root: PathBuf) -> Self {
        let session_id = Uuid::now_v7();
        let mcp_endpoint = format!(
            "{}/v0/groups/{}/mcp",
            mcpjungle_url.trim_end_matches('/'),
            mode.tool_group()
        );

        Self {
            session_id,
            mode,
            mcp_endpoint,
            mcp_token: None,
            project_root,
        }
    }

    pub fn env_vars(&self) -> Vec<(String, String)> {
        let mut vars = vec![
            ("MANAS_MCP_ENDPOINT".into(), self.mcp_endpoint.clone()),
            ("MANAS_SESSION_ID".into(), self.session_id.to_string()),
            (
                "MANAS_PROJECT_ROOT".into(),
                self.project_root.display().to_string(),
            ),
            ("MANAS_BOOT_MODE".into(), self.mode.as_str().into()),
        ];

        if let Some(ref token) = self.mcp_token {
            vars.push(("MANAS_MCP_TOKEN".into(), token.clone()));
        }

        vars
    }
}
