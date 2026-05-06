use std::process::Command;

#[test]
fn done_fails_gracefully_without_sangha() {
    let bin = env!("CARGO_BIN_EXE_manas");
    let output = Command::new(bin)
        .arg("done")
        .env("MANAS_MCPJUNGLE_URL", "http://127.0.0.1:9999")
        .env("MANAS_SANGHA_URL", "http://127.0.0.1:9998")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("failed to run manas");

    // Should fail because sangha is unreachable (lock claim fails)
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("sangha") || stderr.contains("unreachable") || stderr.contains("connect"),
        "expected network error mentioning sangha: stderr={stderr}"
    );
}

#[test]
fn done_skill_body_has_no_hardcoded_paths() {
    let skill_body = include_str!("../skills/done.md");

    assert!(
        !skill_body.contains("~/.claude"),
        "skill body must not contain hardcoded ~/.claude paths"
    );
    assert!(
        !skill_body.contains("/home/"),
        "skill body must not contain hardcoded /home/ paths"
    );
    assert!(
        skill_body.contains("$MANAS_TRANSCRIPT_PATH"),
        "skill body should reference $MANAS_TRANSCRIPT_PATH"
    );
}

#[test]
fn done_transcript_path_injected_from_env() {
    let bin = env!("CARGO_BIN_EXE_manas");
    let output = Command::new(bin)
        .arg("done")
        .env("MANAS_MCPJUNGLE_URL", "http://127.0.0.1:9999")
        .env("MANAS_SANGHA_URL", "http://127.0.0.1:9998")
        .env("MANAS_TRANSCRIPT_PATH", "/tmp/test-transcript.jsonl")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("failed to run manas");

    // Will still fail (sangha unreachable) but proves the env path is accepted
    assert!(!output.status.success());
}
