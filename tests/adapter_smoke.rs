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
        stdout.contains("chitta:") && stdout.contains("yojana:") && stdout.contains("smriti:"),
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
fn warm_grok_writes_overlay_home_then_fails_without_grok() {
    let home = {
        let dir = std::env::temp_dir().join(format!("manas-grok-smoke-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp home");
        dir
    };
    let grok_user = home.join(".grok");
    std::fs::create_dir_all(&grok_user).expect("grok home");
    std::fs::write(grok_user.join("AGENTS.md"), "USER-GLOBAL-AGENTS\n").expect("agents");
    std::fs::write(grok_user.join("config.toml"), "[ui]\nyolo = false\n").expect("config");

    let bin = env!("CARGO_BIN_EXE_manas");
    let output = Command::new(bin)
        .arg("warm")
        .arg("grok")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env("HOME", &home)
        .env("PATH", "")
        .env_remove("MANAS_INSTRUCTIONS")
        .env_remove("MANAS_GROK_INSTRUCTIONS")
        .output()
        .expect("failed to run manas");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        stdout.contains("adapter:") && stdout.contains("grok"),
        "expected grok adapter in stdout: {stdout}"
    );
    assert!(!output.status.success());
    assert!(
        stderr.contains("grok") || stdout.contains("grok"),
        "expected error mentioning grok: stdout={stdout} stderr={stderr}"
    );

    let session = stdout
        .lines()
        .find_map(|l| l.trim().strip_prefix("session:"))
        .expect("warm should print a session id")
        .trim();
    let scratch = home.join(".manas").join("sessions").join(session);
    let overlay = scratch.join("grok-home");

    let config = std::fs::read_to_string(overlay.join("config.toml")).expect("overlay config");
    assert!(config.contains("yolo = false"), "{config}");
    assert!(config.contains("[mcp_servers.yojana]"), "{config}");
    assert!(config.contains("[mcp_servers.sutra]"), "{config}");

    let agents_meta =
        std::fs::symlink_metadata(overlay.join("AGENTS.md")).expect("overlay AGENTS.md");
    assert!(
        agents_meta.file_type().is_symlink(),
        "user AGENTS.md should be symlinked, not copied"
    );

    let user_agents = std::fs::read_to_string(grok_user.join("AGENTS.md")).expect("user agents");
    assert_eq!(user_agents, "USER-GLOBAL-AGENTS\n");
    let user_config = std::fs::read_to_string(grok_user.join("config.toml")).expect("user config");
    assert!(
        !user_config.contains("yojana"),
        "bare grok config must stay untouched: {user_config}"
    );

    let rules = std::fs::read_to_string(overlay.join("rules").join("manas.md")).expect("rules");
    assert!(rules.contains("sutra"), "{rules}");
    assert!(
        rules.contains("grok_tool_surface") && rules.contains("Co-Authored-By: Grok"),
        "{rules}"
    );

    let injected =
        std::fs::read_to_string(scratch.join("manas-instructions.md")).expect("scratch copy");
    assert_eq!(injected, rules);
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
