## in progress

- **manas-harness/10**: opencode adapter — committed (ce9005b), Josh testing. Verify `OPENCODE_CONFIG` env var actually works with opencode, or if the project-root `opencode.json` placement is sufficient on its own. May need to drop the env var and rely purely on CWD discovery.

## pick up next

- **health endpoints** (chitta/10, yojana/12, manas/12): add `GET /health` to chitta (outside bearer auth layer), yojana, and sangha. All three are MCP-only servers today. Pattern: mount a simple axum GET handler returning `{"status":"ok"}` before the auth/MCP layers.
- Consider adding `opencode.json` to `.gitignore` in projects where `manas warm opencode` is used, since the adapter writes it into the project root.

## context

- sangha.service was fixed this session (added `--http` flag) — it's running now.
- smriti runs on port 7333, sangha on 3200. Both are now wired into all four adapters.
- codex adapter had a pre-existing TOML bug where `bearer_token_env_var` was outside its `[mcp_servers.chitta]` section — fixed in ce9005b.
