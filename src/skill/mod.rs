pub mod lock;

use std::path::PathBuf;

use anyhow::{Result, bail};

use crate::adapter::{HarnessAdapter, HarnessHandle};
use crate::binding::Binding;
use lock::{ClaimResult, LockClient, LockScope};

pub struct SkillDef {
    pub name: String,
    pub lock_resource: String,
    pub lock_scope: LockScope,
    pub lock_ttl_secs: u64,
    pub prompt: String,
    pub output_paths: Vec<PathBuf>,
}

#[derive(Debug)]
pub struct SkillOutput {
    pub stdout: String,
    pub exit_success: bool,
}

pub struct SkillShell<L: LockClient> {
    lock_client: L,
}

impl<L: LockClient> SkillShell<L> {
    pub fn new(lock_client: L) -> Self {
        Self { lock_client }
    }

    pub async fn run(
        &self,
        skill: &SkillDef,
        adapter: &dyn HarnessAdapter,
        binding: &Binding,
    ) -> Result<SkillOutput> {
        let session_id = binding.session_id.to_string();

        // 1. Claim lock
        match self
            .lock_client
            .claim(
                &skill.lock_resource,
                &session_id,
                skill.lock_scope.clone(),
                skill.lock_ttl_secs,
            )
            .await?
        {
            ClaimResult::Acquired => {}
            ClaimResult::AlreadyHeld { by_session } => {
                bail!(
                    "cannot run `{}`: lock `{}` held by session {}",
                    skill.name,
                    skill.lock_resource,
                    by_session
                );
            }
        }

        // 2. Run body, releasing lock on any exit path
        let result = self.run_body(skill, adapter, binding).await;

        // 3. Always release lock
        if let Err(e) = self
            .lock_client
            .release(&skill.lock_resource, &session_id)
            .await
        {
            eprintln!(
                "warning: failed to release lock `{}`: {}",
                skill.lock_resource, e
            );
        }

        result
    }

    async fn run_body(
        &self,
        skill: &SkillDef,
        adapter: &dyn HarnessAdapter,
        binding: &Binding,
    ) -> Result<SkillOutput> {
        let handle: HarnessHandle = adapter.launch(binding, Some(&skill.prompt)).await?;

        let output = handle.child.wait_with_output().await?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();

        Ok(SkillOutput {
            stdout,
            exit_success: output.status.success(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lock::mock::MockLockClient;

    fn test_skill() -> SkillDef {
        SkillDef {
            name: "test-skill".into(),
            lock_resource: "test-lock".into(),
            lock_scope: LockScope::Project,
            lock_ttl_secs: 120,
            prompt: "do nothing".into(),
            output_paths: vec![],
        }
    }

    fn test_binding() -> Binding {
        let config = crate::config::ManasConfig {
            manas_dir: PathBuf::from("/tmp/manas"),
            chitta_url: "http://localhost:3100".into(),
            yojana_url: "http://localhost:4200".into(),
            sangha_url: "http://localhost:3200".into(),
            smriti_url: "http://localhost:7333".into(),
            serve_port: 3000,
        };
        Binding::new(&config, PathBuf::from("/tmp/test-project"))
    }

    #[tokio::test]
    async fn lock_acquired_and_released_on_success() {
        let mock = MockLockClient::default();
        let shell = SkillShell::new(mock.clone());
        let skill = test_skill();
        let binding = test_binding();

        // FakeAdapter spawns `true` which exits 0 — body succeeds
        let _result = shell.run(&skill, &FakeAdapter, &binding).await;

        // FakeAdapter returns a process that exits immediately
        let claims = mock.claims.lock().unwrap();
        let releases = mock.releases.lock().unwrap();

        assert_eq!(claims.len(), 1);
        assert_eq!(claims[0], "test-lock");
        assert_eq!(releases.len(), 1);
        assert_eq!(releases[0], "test-lock");
    }

    #[tokio::test]
    async fn lock_released_on_body_error() {
        let mock = MockLockClient::default();
        let shell = SkillShell::new(mock.clone());
        let skill = test_skill();
        let binding = test_binding();

        // FailAdapter always errors on launch
        let result = shell.run(&skill, &FailAdapter, &binding).await;

        assert!(result.is_err());

        let claims = mock.claims.lock().unwrap();
        let releases = mock.releases.lock().unwrap();

        assert_eq!(claims.len(), 1, "lock should have been claimed");
        assert_eq!(
            releases.len(),
            1,
            "lock should have been released despite error"
        );
    }

    #[tokio::test]
    async fn lock_conflict_prevents_run() {
        let mock = MockLockClient::default();
        *mock.should_conflict.lock().unwrap() = true;

        let shell = SkillShell::new(mock.clone());
        let skill = test_skill();
        let binding = test_binding();

        let result = shell.run(&skill, &FakeAdapter, &binding).await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("held by session"),
            "error should mention conflict: {err}"
        );

        // No release should happen since we never acquired
        let releases = mock.releases.lock().unwrap();
        assert_eq!(releases.len(), 0);
    }

    // Adapter that spawns `true` (exits 0 immediately)
    struct FakeAdapter;

    #[async_trait::async_trait]
    impl HarnessAdapter for FakeAdapter {
        fn name(&self) -> &'static str {
            "fake"
        }

        async fn launch(
            &self,
            _binding: &Binding,
            _prompt: Option<&str>,
        ) -> Result<crate::adapter::HarnessHandle> {
            let child = tokio::process::Command::new("true")
                .stdout(std::process::Stdio::piped())
                .spawn()?;

            Ok(crate::adapter::HarnessHandle {
                child,
                transcript_path: None,
                scratch_dir: PathBuf::from("/tmp/fake"),
            })
        }

        fn transcript_path(&self, _binding: &Binding) -> Option<PathBuf> {
            None
        }

        async fn shutdown(&self, handle: &mut crate::adapter::HarnessHandle) -> Result<()> {
            handle.child.wait().await?;
            Ok(())
        }
    }

    // Adapter that always fails to launch
    struct FailAdapter;

    #[async_trait::async_trait]
    impl HarnessAdapter for FailAdapter {
        fn name(&self) -> &'static str {
            "fail"
        }

        async fn launch(
            &self,
            _binding: &Binding,
            _prompt: Option<&str>,
        ) -> Result<crate::adapter::HarnessHandle> {
            bail!("intentional launch failure")
        }

        fn transcript_path(&self, _binding: &Binding) -> Option<PathBuf> {
            None
        }

        async fn shutdown(&self, handle: &mut crate::adapter::HarnessHandle) -> Result<()> {
            handle.child.wait().await?;
            Ok(())
        }
    }
}
