# Codex Initial Code Review

Date: 2026-05-05
Scope: initial foundation review of `manas-cli` Rust CLI, tests, and nearby design docs.

## Findings

### P1: `done` creates a new session instead of closing the active session

`manas done` calls `Binding::new(...)` at [src/cmd/done.rs:18](../../src/cmd/done.rs#L18), which generates a fresh UUID and rich MCP endpoint. That means `done` does not use the existing `MANAS_SESSION_ID`, does not attach to the binding started by `warm`, and cannot revoke or mark the real active session as closed. It also causes transcript auto-detection to look under a path for the newly generated session rather than the session being summarized.

This cuts against the shutdown contract in `docs/boot-contract.md`: `done` should operate on the active binding, not create a second one. The CLI needs a binding resolver that prefers current session env vars, then `bindings.log`, and only creates a new binding for commands that actually boot a new harness.

### P1: Skill failures still produce a successful `manas done` exit

`SkillShell::run_body` records `exit_success` at [src/skill/mod.rs:95](../../src/skill/mod.rs#L95), but [src/cmd/done.rs:38](../../src/cmd/done.rs#L38) only prints a warning and then returns `Ok(())` at [src/cmd/done.rs:46](../../src/cmd/done.rs#L46). If the LLM body fails to store observations or write `docs/handoff.md`, automation will still see `manas done` as successful.

For a shutdown command, non-zero skill body exit should be a command failure unless there is an explicit `--best-effort` mode. `output_paths` is also unused at [src/skill/mod.rs:17](../../src/skill/mod.rs#L17), so the shell never verifies that required artifacts were actually produced.

### P1: Project lock TTL can expire during normal skill execution

`done` claims the `handoff` lock with a 300 second TTL at [src/cmd/done.rs:31](../../src/cmd/done.rs#L31). `SkillShell::run` claims once at [src/skill/mod.rs:43](../../src/skill/mod.rs#L43), runs the body at [src/skill/mod.rs:66](../../src/skill/mod.rs#L66), and releases at [src/skill/mod.rs:69](../../src/skill/mod.rs#L69). It never heartbeats while the body runs, even though the trait has an unused `heartbeat` method at [src/skill/lock.rs:18](../../src/skill/lock.rs#L18).

An LLM wrap-up can easily exceed five minutes. After TTL expiry, another session can acquire the same lock and both sessions can write `docs/handoff.md`. The shell should run a heartbeat task for long-running bodies and cancel it after body completion, with tests using a fake lock client and controlled time.

### P1: The test suite can launch a real Claude Code process

`tests/adapter_smoke.rs` assumes `claude` is absent: [tests/adapter_smoke.rs:33](../../tests/adapter_smoke.rs#L33). Both `warm_prints_session_info_then_fails_without_claude` and `binding_env_vars_are_complete` run the actual binary at [tests/adapter_smoke.rs:6](../../tests/adapter_smoke.rs#L6) and [tests/adapter_smoke.rs:77](../../tests/adapter_smoke.rs#L77). On a developer machine or CI image with Claude Code installed, these tests may launch a real interactive harness, hang, mutate local Claude state, or fail for the wrong reason.

The test should isolate `PATH` to a temp directory or use an injected/fake adapter. The current tests pass in this environment because `claude` is unavailable, not because the behavior is deterministic.

### P1: `warm` still bypasses the hard boot contract

`warm` prints a binding and launches the harness, but the security and lifecycle steps are TODOs: health gate, token minting, binding log append, token revoke, resource release, and binding close are all absent at [src/cmd/warm.rs:19](../../src/cmd/warm.rs#L19) and [src/cmd/warm.rs:35](../../src/cmd/warm.rs#L35). `Binding::new` also always leaves `mcp_token` as `None` at [src/binding.rs:51](../../src/binding.rs#L51).

This is acceptable for a scaffold only if it is treated as insecure/dev behavior. Before relying on `warm` as a real rich session, the implementation needs to enforce the boot contract from `docs/boot-contract.md`: health gate, mode-specific endpoint validation, per-session auth where applicable, durable binding records, and teardown/GC.

### P2: Claude transcript discovery is probably wrong and is not covered

[src/adapter/claude_code.rs:83](../../src/adapter/claude_code.rs#L83) calls `md5_hash`, but the implementation at [src/adapter/claude_code.rs:112](../../src/adapter/claude_code.rs#L112) uses Rust's `DefaultHasher`, not MD5. That hash is also not an adapter contract with Claude Code. `done` then uses that synthetic path to find the project transcript directory at [src/cmd/done.rs:60](../../src/cmd/done.rs#L60).

The result is that transcript discovery can silently return `None`, and the current `done_transcript_path_injected_from_env` test does not catch it because the command fails before the skill body runs and the provided `/tmp/test-transcript.jsonl` does not exist. Transcript path resolution should be based on confirmed Claude Code storage semantics or an explicit env var from the active session, with unit tests around existing and missing paths.

### P2: Sangha lock responses are parsed by brittle text matching

`claim` treats a response as a conflict only if `/result/content/0/text` contains `"already held"` or `"conflict"` at [src/skill/lock.rs:227](../../src/skill/lock.rs#L227). Otherwise it returns `Acquired` at [src/skill/lock.rs:236](../../src/skill/lock.rs#L236). It also reports `by_session: session_id.to_string()` at [src/skill/lock.rs:231](../../src/skill/lock.rs#L231), which is the current session, not necessarily the holder.

This should parse a structured Sangha response, or Sangha should expose a stable tool result shape for lock acquisition. Text matching makes lock safety depend on human wording.

### P2: HTTP calls have no explicit timeout

`SanghaLockClient` builds a default reqwest client at [src/skill/lock.rs:53](../../src/skill/lock.rs#L53), and all MCP calls use `.send().await` without a request timeout. A dead local service usually fails fast, but a blackholed or half-open address can hang `manas done` indefinitely.

Set a conservative default timeout on the client, and consider a separate longer timeout for LLM body execution rather than subsystem coordination calls.

### P2: Session config files are written without restrictive permissions

The Claude adapter writes MCP config at [src/adapter/claude_code.rs:37](../../src/adapter/claude_code.rs#L37). Once `mcp_token` is populated, that file can contain a bearer token at [src/adapter/claude_code.rs:25](../../src/adapter/claude_code.rs#L25). The code relies on process umask for both the session directory and file permissions.

Because this is credential material, create `~/.manas` and session directories with owner-only permissions where supported, and write token-bearing files as `0600`. The admin credential doc already requires `0600` for `~/.manas/admin-token`; session config should follow the same standard.

### P2: Project roots are not canonicalized

The boot contract says `MANAS_PROJECT_ROOT` is canonicalized. `warm` uses `std::env::current_dir()` at [src/cmd/warm.rs:11](../../src/cmd/warm.rs#L11), and `done` does the same at [src/cmd/done.rs:16](../../src/cmd/done.rs#L16). `Binding::new` stores whatever path it receives at [src/binding.rs:52](../../src/binding.rs#L52).

Different spellings of the same path, especially symlinks, can create different Sangha scopes, transcript lookup paths, and binding records. Canonicalize at binding creation or before calling it, and decide how to handle deleted or virtual working directories.

### P2: `Cargo.lock` is ignored for a binary crate

`.gitignore` contains `Cargo.lock`, and `git ls-files Cargo.lock` returns nothing. For a binary application, the lockfile should normally be committed so CI and operators build the same dependency graph. Remove `Cargo.lock` from `.gitignore` and commit the lockfile unless this repository is intentionally library-only, which `Cargo.toml` says it is not.

### P3: Formatting and lint hygiene are not clean yet

`cargo fmt --check` reports formatting drift in `src/adapter/claude_code.rs`, `src/binding.rs`, `src/cmd/done.rs`, `src/cmd/warm.rs`, `src/skill/lock.rs`, `src/skill/mod.rs`, and `tests/adapter_smoke.rs`.

`cargo clippy --all-targets --all-features` succeeds when run as `RUSTC_WRAPPER= cargo clippy --all-targets --all-features`, but reports warnings for unused API surface and simplifiable code. The most notable warnings are unused `output_paths`, unused `heartbeat`, unused `LockScope::User`, nested `if` chains in transcript discovery, and `.last()` on a double-ended iterator in SSE parsing.

## Positive Foundation

The code is small and readable. The central `Binding` shape is a good boundary for adapter implementations, and `BootMode` already maps modes to mcpjungle tool-group endpoints. The lock client is trait-based, which makes the skill shell testable. The existing unit tests already cover lock release on body errors and lock conflict behavior, which is the right direction for concurrency-sensitive code.

The docs are also doing useful work. `docs/boot-contract.md` is specific enough to catch several implementation gaps, and `docs/admin-cred.md` gives a clear credential resolution model.

## Verification

Commands run:

```bash
cargo test
cargo fmt --check
cargo clippy --all-targets --all-features
RUSTC_WRAPPER= cargo clippy --all-targets --all-features
```

Results:

- `cargo test`: passed, 10 tests total.
- `cargo fmt --check`: failed with formatting diffs.
- `cargo clippy --all-targets --all-features`: blocked by `sccache: Operation not permitted`.
- `RUSTC_WRAPPER= cargo clippy --all-targets --all-features`: passed with warnings.

## Suggested Fix Order

1. Make tests deterministic by preventing real harness launch in smoke tests.
2. Introduce active binding resolution and make `done` operate on the current session.
3. Treat non-zero skill body exit and missing required output paths as command failures.
4. Add lock heartbeat support around long-running skill bodies.
5. Finish the `warm` boot contract pieces or explicitly mark the command as dev-only until they exist.
6. Fix transcript discovery through a tested adapter contract.
7. Add HTTP timeouts and structured Sangha lock response parsing.
8. Commit `Cargo.lock`, run `cargo fmt`, and make clippy clean enough for CI.
