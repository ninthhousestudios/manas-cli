//! End-to-end checks that the injected system prompt is read at runtime, not
//! frozen at compile time (manas-cli/9).

use std::path::{Path, PathBuf};
use std::process::Command;

/// Run `manas warm` with an isolated HOME and no `claude` on PATH, so the
/// adapter writes its scratch files and then fails to spawn. Returns
/// (stdout, session scratch dir).
fn warm_isolated(home: &Path, env: &[(&str, &str)]) -> (String, PathBuf) {
    let bin = env!("CARGO_BIN_EXE_manas");
    let mut cmd = Command::new(bin);
    cmd.arg("warm")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env("HOME", home)
        // An empty PATH guarantees the `claude` spawn fails, which keeps the
        // test from launching a real interactive session.
        .env("PATH", "")
        .env_remove("MANAS_INSTRUCTIONS");
    for (k, v) in env {
        cmd.env(k, v);
    }

    let output = cmd.output().expect("failed to run manas");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    let session = stdout
        .lines()
        .find_map(|l| l.trim().strip_prefix("session:"))
        .expect("warm should print a session id")
        .trim()
        .to_string();

    let scratch = home.join(".manas").join("sessions").join(session);
    (stdout, scratch)
}

fn temp_home(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("manas-instr-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("failed to create temp home");
    dir
}

#[test]
fn first_run_seeds_the_editable_file_and_later_edits_take_effect() {
    let home = temp_home("seed");
    let resolved = home.join(".manas").join("manas-instructions.md");

    let (stdout, scratch) = warm_isolated(&home, &[]);
    assert!(
        stdout.contains("prompt:") && stdout.contains("(seeded)"),
        "first run should report seeding the editable file: {stdout}"
    );
    let seeded = std::fs::read_to_string(&resolved).expect("first run should seed the file");
    assert!(
        seeded.contains("sutra"),
        "seeded file should hold the compiled-in instructions"
    );

    // Edit the resolved file — no rebuild — and the next run must inject it.
    std::fs::write(&resolved, "SENTINEL edited instructions\n").expect("failed to edit");
    let (stdout, scratch2) = warm_isolated(&home, &[]);
    assert!(
        !stdout.contains("(seeded)"),
        "second run should read the existing file, not reseed: {stdout}"
    );

    let injected = std::fs::read_to_string(scratch2.join("manas-instructions.md"))
        .expect("scratch copy should exist");
    assert!(
        injected.contains("SENTINEL edited instructions"),
        "edited text should reach the injected prompt: {injected}"
    );
    assert!(
        !injected.contains("sutra"),
        "stale compiled-in text should be gone: {injected}"
    );
    assert_ne!(scratch, scratch2, "each run gets its own session dir");
}

#[test]
fn injected_copy_carries_provenance() {
    let home = temp_home("provenance");
    let override_path = home.join("custom-instructions.md");
    std::fs::write(&override_path, "override body\n").expect("failed to write override");

    let (stdout, scratch) = warm_isolated(
        &home,
        &[(
            "MANAS_INSTRUCTIONS",
            override_path.to_str().expect("utf-8 path"),
        )],
    );

    assert!(
        stdout.contains(override_path.to_str().expect("utf-8 path")),
        "warm should state which source it used: {stdout}"
    );

    let injected = std::fs::read_to_string(scratch.join("manas-instructions.md"))
        .expect("scratch copy should exist");
    assert!(injected.starts_with("override body"));
    assert!(
        injected.contains("manas-instructions provenance:"),
        "injected text should carry provenance: {injected}"
    );
    assert!(
        injected.contains(override_path.to_str().expect("utf-8 path")),
        "provenance should name the source path: {injected}"
    );
    assert!(
        injected.contains("fnv1a64=") && injected.contains("mtime_epoch="),
        "provenance should carry a hash and mtime: {injected}"
    );
}

#[test]
fn unreadable_override_falls_back_to_compiled_in() {
    let home = temp_home("fallback");
    let missing = home.join("nope.md");

    let (stdout, scratch) = warm_isolated(
        &home,
        &[("MANAS_INSTRUCTIONS", missing.to_str().expect("utf-8 path"))],
    );

    assert!(
        stdout.contains("compiled-in"),
        "fallback should be reported: {stdout}"
    );
    let injected = std::fs::read_to_string(scratch.join("manas-instructions.md"))
        .expect("scratch copy should exist");
    assert!(
        injected.contains("sutra"),
        "fallback should inject the compiled-in instructions: {injected}"
    );
    assert!(injected.contains("source=compiled-in"));
}
