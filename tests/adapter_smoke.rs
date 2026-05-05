use std::process::Command;

#[test]
fn warm_prints_session_info_then_fails_without_claude() {
    let bin = env!("CARGO_BIN_EXE_manas");
    let output = Command::new(bin)
        .arg("warm")
        .env("MANAS_MCPJUNGLE_URL", "http://127.0.0.1:9999")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("failed to run manas");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        stdout.contains("manas warm"),
        "expected 'manas warm' header in stdout: {stdout}"
    );
    assert!(
        stdout.contains("session:"),
        "expected session id in stdout: {stdout}"
    );
    assert!(
        stdout.contains("http://127.0.0.1:9999/v0/groups/full/mcp"),
        "expected mcpjungle endpoint in stdout: {stdout}"
    );
    assert!(
        stdout.contains("claude-code"),
        "expected adapter name in stdout: {stdout}"
    );

    // `claude` binary likely not on PATH in CI, so expect a spawn failure
    assert!(!output.status.success());
    assert!(
        stderr.contains("claude") || stdout.contains("claude"),
        "expected error mentioning claude: stdout={stdout} stderr={stderr}"
    );
}

#[test]
fn health_reads_config() {
    let bin = env!("CARGO_BIN_EXE_manas");
    let output = Command::new(bin)
        .arg("health")
        .env("MANAS_MCPJUNGLE_URL", "http://test.invalid:1234")
        .env_remove("MANAS_ADMIN_TOKEN")
        .output()
        .expect("failed to run manas");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("http://test.invalid:1234"));
    assert!(stdout.contains("admin token: NOT FOUND"));
}

#[test]
fn health_finds_admin_token_from_env() {
    let bin = env!("CARGO_BIN_EXE_manas");
    let output = Command::new(bin)
        .arg("health")
        .env("MANAS_ADMIN_TOKEN", "test-secret-token")
        .output()
        .expect("failed to run manas");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("admin token: configured"));
}

#[test]
fn binding_env_vars_are_complete() {
    let bin = env!("CARGO_BIN_EXE_manas");
    // warm will fail to spawn claude but will print the binding info first
    let output = Command::new(bin)
        .arg("warm")
        .env("MANAS_MCPJUNGLE_URL", "http://localhost:8080")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("failed to run manas");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify the endpoint encodes the tool group
    assert!(
        stdout.contains("/v0/groups/full/mcp"),
        "endpoint should include tool group path: {stdout}"
    );
}

