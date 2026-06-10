use std::process::Command;

#[test]
fn warm_prints_session_info_then_fails_without_claude() {
    let bin = env!("CARGO_BIN_EXE_manas");
    let output = Command::new(bin)
        .arg("warm")
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
        stdout.contains("manas:") && stdout.contains("chitta:") && stdout.contains("yojana:"),
        "expected service URLs in stdout: {stdout}"
    );
    assert!(
        stdout.contains("claude-code"),
        "expected adapter name in stdout: {stdout}"
    );

    assert!(!output.status.success());
    assert!(
        stderr.contains("claude") || stdout.contains("claude"),
        "expected error mentioning claude: stdout={stdout} stderr={stderr}"
    );
}

#[test]
fn health_checks_services() {
    let bin = env!("CARGO_BIN_EXE_manas");
    let output = Command::new(bin)
        .arg("health")
        .env("MANAS_CHITTA_URL", "http://127.0.0.1:19999")
        .env("MANAS_YOJANA_URL", "http://127.0.0.1:19998")
        .env("MANAS_SANGHA_URL", "http://127.0.0.1:19997")
        .output()
        .expect("failed to run manas");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("chitta:"), "should list chitta: {stdout}");
    assert!(stdout.contains("yojana:"), "should list yojana: {stdout}");
    assert!(stdout.contains("sangha:"), "should list sangha: {stdout}");
}

#[test]
fn binding_env_vars_include_service_urls() {
    let bin = env!("CARGO_BIN_EXE_manas");
    let output = Command::new(bin)
        .arg("warm")
        .env("MANAS_CHITTA_URL", "http://127.0.0.1:3100")
        .env("MANAS_YOJANA_URL", "http://127.0.0.1:4200")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("failed to run manas");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("chitta:") && stdout.contains("3100"),
        "expected chitta URL in output: {stdout}"
    );
    assert!(
        stdout.contains("yojana:") && stdout.contains("4200"),
        "expected yojana URL in output: {stdout}"
    );
}
