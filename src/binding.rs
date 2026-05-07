use std::path::PathBuf;

use uuid::Uuid;

use crate::config::ManasConfig;

#[derive(Debug, Clone)]
pub struct Binding {
    pub session_id: Uuid,
    pub manas_url: String,
    pub chitta_url: String,
    pub yojana_url: String,
    pub sangha_url: String,
    pub smriti_url: String,
    pub project_root: PathBuf,
    pub transcript_path: Option<PathBuf>,
}

impl Binding {
    pub fn new(config: &ManasConfig, project_root: PathBuf) -> Self {
        Self {
            session_id: Uuid::now_v7(),
            manas_url: config.serve_url(),
            chitta_url: config.chitta_url.clone(),
            yojana_url: config.yojana_url.clone(),
            sangha_url: config.sangha_url.clone(),
            smriti_url: config.smriti_url.clone(),
            project_root,
            transcript_path: None,
        }
    }

    pub fn env_vars(&self) -> Vec<(String, String)> {
        let mut vars = vec![
            ("MANAS_SESSION_ID".into(), self.session_id.to_string()),
            (
                "MANAS_PROJECT_ROOT".into(),
                self.project_root.display().to_string(),
            ),
            ("MANAS_URL".into(), self.manas_url.clone()),
            ("MANAS_CHITTA_URL".into(), self.chitta_url.clone()),
            ("MANAS_YOJANA_URL".into(), self.yojana_url.clone()),
            ("MANAS_SANGHA_URL".into(), self.sangha_url.clone()),
            ("MANAS_SMRITI_URL".into(), self.smriti_url.clone()),
        ];

        if let Some(ref path) = self.transcript_path {
            vars.push((
                "MANAS_TRANSCRIPT_PATH".into(),
                path.display().to_string(),
            ));
        }

        vars
    }
}
