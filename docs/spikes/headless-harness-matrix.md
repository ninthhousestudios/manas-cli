# headless harness matrix

Status: complete
Date: 2026-05-04
Task: manas-harness/3
Author: spike agent

Answers OQ-3 from boot-contract.md §8.

---

## methodology

Test environment:
- MCP servers: yojana at `http://127.0.0.1:4200/mcp` (no auth), chitta at `http://127.0.0.1:3100/mcp` (bearer token)
- Each CLI was probed headlessly with stdin redirected from `/dev/null` where needed
- Test configs written to `/tmp/spike-headless/<harness>/` (no global config pollution for Claude, Gemini)
- Codex used `CODEX_HOME=/tmp/spike-headless/codex-home` to avoid polluting `~/.codex/`
- opencode used a project-scoped `opencode.json` in the test directory

Smoke test prompt (all harnesses): "Call the yojana_query tool with filter_status=["in-progress"] and limit=3. Return the task titles."
Bearer token test: call `health_check` via chitta MCP server with the user's real chitta bearer token.

---

## claude code (2.1.128)

### headless invocation

```
claude -p "..." \
  --mcp-config /path/to/mcp.json \
  --dangerously-skip-permissions \
  --model claude-haiku-4-5
```

Flag: `-p` / `--print`. Emits final text response and exits.
Output formats: `--output-format text|json|stream-json`.

### mcp config method

JSON file passed via `--mcp-config`. Can be repeated. `--strict-mcp-config` suppresses all other MCP sources.

HTTP server format:

```json
{
  "mcpServers": {
    "yojana": {
      "type": "http",
      "url": "http://127.0.0.1:4200/mcp"
    }
  }
}
```

**Gotcha:** `"url"` alone (without `"type": "http"`) fails validation with "Does not adhere to MCP server configuration schema". The `type` field is required.

### bearer token support

Via `headers` in the config:

```json
{
  "mcpServers": {
    "chitta": {
      "type": "http",
      "url": "http://127.0.0.1:3100/mcp",
      "headers": {
        "Authorization": "Bearer <token>"
      }
    }
  }
}
```

Token is written to disk in the config file. This is noted as OQ-4 in the boot contract — acceptable for v0 (single-session, revoked on exit).

### smoke test result

**PASS** (exit 0)

```
Here are the in-progress tasks (1 result found):
1. SPIKE: headless tool-calling across CC / Gemini / Codex / opencode (manas-harness/3)
```

Bearer token test (chitta): **PASS** — health_check succeeded through bearer auth.

### gotchas

- `--strict-mcp-config` is useful for adapter v1 to prevent CLAUDE.md-configured servers from leaking in
- Warning about stdin data received can appear when piping; suppress with `< /dev/null` or ignore (non-fatal)
- `--bare` mode skips CLAUDE.md auto-discovery and OAuth/keychain reads — good for manas boot in minimal mode

---

## gemini cli (0.40.0)

### headless invocation

```
GEMINI_CLI_TRUST_WORKSPACE=true gemini -p "..." --yolo
```

Flag: `-p` / `--prompt`. Emits final text response and exits.
Output formats: `-o` / `--output-format text|json|stream-json`.

**Gotcha (critical):** Gemini refuses to run headlessly in untrusted directories with exit 55 and message "not running in a trusted directory". Fix: set `GEMINI_CLI_TRUST_WORKSPACE=true` or use `--skip-trust`. The manas adapter must export this env var.

### mcp config method

Project-scoped `settings.json` at `.gemini/settings.json` in the working directory, or user-scoped at `~/.gemini/settings.json`. Config written by `gemini mcp add` or manually.

HTTP server format:

```json
{
  "mcpServers": {
    "yojana": {
      "url": "http://127.0.0.1:4200/mcp",
      "type": "http"
    }
  }
}
```

**Gotcha:** `"transport"` key is rejected with "Unrecognized key(s) in object: 'transport'". Use `"type"` instead.

The `--allowed-mcp-server-names` CLI flag can restrict which configured servers are active per-invocation.

### bearer token support

Via `headers` in settings.json:

```json
{
  "mcpServers": {
    "chitta": {
      "url": "http://127.0.0.1:3100/mcp",
      "type": "http",
      "headers": {
        "Authorization": "Bearer <token>"
      }
    }
  }
}
```

Also available via `gemini mcp add <name> <url> -t http -H "Authorization: Bearer <token>"`.

### smoke test result

**PASS** (exit 0)

```
The task title is:
- SPIKE: headless tool-calling across CC / Gemini / Codex / opencode
```

Bearer token test (chitta): **PASS** — health_check succeeded through bearer auth.

### gotchas

- `GEMINI_CLI_TRUST_WORKSPACE=true` is a hard requirement for any non-interactive invocation outside pre-trusted directories. The manas adapter must always set this env var.
- `--yolo` auto-approves all tool calls; required for unattended runs. Alternatively `--approval-mode yolo`.
- Gemini picks up `.gemini/settings.json` from cwd, which makes per-session config straightforward: write to `~/.manas/sessions/<id>/.gemini/settings.json` and invoke from that dir, or use `~/.gemini/settings.json` for the session (risks leaking between concurrent sessions if they share home).
- `ripgrep` absent warning is cosmetic, non-fatal.

---

## codex cli (codex-cli 0.125.0)

### headless invocation

```
CODEX_HOME=/tmp/codex-session codex exec \
  --skip-git-repo-check \
  --dangerously-bypass-approvals-and-sandbox \
  "..." < /dev/null
```

Subcommand: `exec` (alias `e`). Reads prompt from argument or stdin. `--json` flag emits JSONL events; `-o` writes final message to a file.

**Gotcha (critical):** Without `< /dev/null`, `codex exec` blocks waiting for stdin for ~3 seconds then proceeds. In a scripted context, always redirect stdin.

### mcp config method

Codex stores MCP servers in `$CODEX_HOME/config.toml` (default `~/.codex/config.toml`). The `CODEX_HOME` env var redirects all config, state, and auth — ideal for per-session isolation.

TOML format:

```toml
[mcp_servers.yojana]
url = "http://127.0.0.1:4200/mcp"
```

HTTP servers use `url =`; stdio servers use a different structure. The `codex mcp add <name> --url <url>` command populates this automatically.

**Gotcha:** `CODEX_HOME` must not be under `/tmp` — Codex refuses to install helper binaries there. Use `~/.manas/sessions/<id>/codex/` instead. The `WARNING: Refusing to create helper binaries under temporary dir` is non-fatal for MCP calls but the adapter should use a persistent-ish path.

### bearer token support

Via `bearer_token_env_var` in config.toml:

```toml
[mcp_servers.chitta]
url = "http://127.0.0.1:3100/mcp"
bearer_token_env_var = "CHITTA_TOKEN"
```

The token is read from the named env var at runtime — **not written to disk**. This is the best token handling of all four harnesses for security (OQ-4 in boot contract).

The `codex mcp add` flag is `--bearer-token-env-var <ENV_VAR>`.

### smoke test result

**PASS** (exit 0, non-fatal error log)

```
- mcpjungle PR: per-client Tool Group binding
- SPIKE: headless tool-calling across CC / Gemini / Codex / opencode
```

Log line: `ERROR codex_core::session: failed to record rollout items: thread ... not found` — this is a non-fatal bookkeeping bug when running from a non-standard CODEX_HOME. Does not affect MCP tool calls.

Bearer token test (chitta): **PASS** — health_check succeeded through env-var-based bearer auth.

### model constraint

Codex on this machine is authenticated via a ChatGPT account. Many models (`gpt-4.1-mini`, `o4-mini`) are unavailable; `gpt-5.5` (the configured default) works. The manas adapter must use the user's configured model or allow override.

### gotchas

- `CODEX_HOME` isolation is the cleanest per-session config mechanism of any harness — use it.
- `--ignore-user-config` drops user config entirely; then you must re-specify model. Use CODEX_HOME isolation instead to inherit model while isolating MCP config.
- MCP config cannot be passed inline via `-c` (TOML array parsing fails); must be in config.toml.
- Inline `-c 'mcp_servers=[...]'` fails with "invalid type: sequence, expected a map".

---

## opencode (1.14.20)

### headless invocation

opencode does not run fully headlessly in a single command. The intended pattern for non-interactive use is:

```
# Step 1 — start server (can be long-lived or per-session)
opencode serve --port <PORT> &

# Step 2 — send prompt and collect response
opencode run --attach http://127.0.0.1:<PORT> \
  --dangerously-skip-permissions \
  --format json \
  "..."
```

`opencode run` without `--attach` starts a server, posts the message, and **exits immediately** without waiting for the LLM response. This appears to be a design choice (or bug) in 1.14.20 — the run command is fire-and-forget when self-hosting. With `--attach` to a running server, it blocks until the LLM response completes and streams events to stdout.

Output format: `--format json` emits JSONL events (type: text, tool_use, step_start, step_finish, error). `--format default` emits formatted text.

### mcp config method

`opencode.json` in the project directory (or `~/.config/opencode/opencode.json` globally). Project-level config merges with global config.

JSON format:

```json
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "yojana": {
      "type": "remote",
      "url": "http://127.0.0.1:4200/mcp",
      "enabled": true
    }
  }
}
```

`type: "remote"` for HTTP; `type: "local"` for stdio.

### bearer token support

Via `headers` in `opencode.json`:

```json
{
  "mcp": {
    "chitta": {
      "type": "remote",
      "url": "http://127.0.0.1:3100/mcp",
      "enabled": true,
      "headers": {
        "Authorization": "Bearer <token>"
      }
    }
  }
}
```

Token is written to disk in opencode.json (same OQ-4 concern as CC and Gemini).

### smoke test result

**PASS with complexity** (exit 0, requires two-process pattern)

Event stream confirmed yojana_query tool call and result:
```json
{"type":"tool_use","part":{"tool":"yojana_yojana_query","state":{"status":"completed","input":{"status":"in-progress","limit":3},"output":"[...SPIKE task...]"}}}
```

Final text response confirmed task title.

Bearer token test (chitta): **PASS** — health_check tool called successfully via chittars (bearer auth) through the running server.

### model constraint

opencode's global config uses ollama as provider (ollama was not running during the spike). Override via `"model"` in project `opencode.json`. opencode uses its own model namespace: `"opencode/claude-haiku-4-5"` not `"anthropic/claude-haiku-4-5"`.

### gotchas

- **Two-process requirement**: the manas adapter must manage the opencode server lifecycle (start, wait for ready, invoke, collect, shutdown). This is significantly more complex than the other three harnesses.
- Broken plugin at `~/.opencode/plugins/notification.js` causes a non-fatal ERROR on every startup; cosmetic noise.
- `OPENCODE_SERVER_PASSWORD` should be set to secure the HTTP server when running multi-user or in shared environments.
- opencode server picks up `opencode.json` from cwd at connection time, not server start time — the project context is per-connection, not per-server. This enables one server serving multiple project contexts.
- Config file is read on `opencode serve` and on `opencode run --attach` connect — both paths must have access to the MCP config.

---

## summary matrix

| Harness | Headless? | HTTP MCP? | Bearer token? | Tool-calling headless? | Notes |
|---|---|---|---|---|---|
| Claude Code 2.1.128 | YES (`-p`) | YES (`type: "http"`) | YES (inline headers in config) | **YES** | Cleanest; `--strict-mcp-config` locks surface |
| Gemini CLI 0.40.0 | YES (`-p`) | YES (`type: "http"`) | YES (inline headers in config) | **YES** | Requires `GEMINI_CLI_TRUST_WORKSPACE=true` and `--yolo` |
| Codex CLI 0.125.0 | YES (`exec`) | YES (TOML `url =`) | YES (env var ref; best security) | **YES** | CODEX_HOME isolation; `< /dev/null`; avoid `/tmp` |
| opencode 1.14.20 | PARTIAL (`run --attach`) | YES (`type: "remote"`) | YES (inline headers in config) | **YES (with serve daemon)** | Requires two-process lifecycle |

---

## recommendations

### tier 1 — full adapter support (v1)

**Claude Code** and **Gemini CLI** are straightforward single-process headless runners. Both support HTTP MCP with bearer tokens inline in config. CC is the tightest: `--strict-mcp-config` prevents any server leakage from the user's ambient CLAUDE.md config, which is exactly what manas-cli's binding contract requires. These two should be in the v1 adapter set.

**Codex CLI** passes all requirements. The `CODEX_HOME` isolation mechanism is actually the cleanest of any harness for per-session config management. The env-var bearer token pattern (no token on disk) is a security bonus. Include in v1 with the caveat that the CODEX_HOME path must not be under `/tmp`.

### tier 2 — deferred (post-v1)

**opencode** works, but the two-process pattern (serve + run --attach) adds lifecycle complexity that the other three don't require. The manas adapter would need to: start the server, poll for readiness, invoke `run --attach`, collect output, and shut down the server. This is non-trivial and the failure modes (server crash mid-session, port conflicts) need careful handling. Defer opencode to v2 unless there's a specific demand. File a tracking issue.

### no-tools fallback

None of the four harnesses need a no-tools fallback for the headless case — all four can call MCP tools headlessly. The OQ-3 concern was specifically about Codex; it is resolved: Codex exec does support HTTP MCP with bearer auth.

If a future harness (e.g. a raw API runner with no MCP client) is added, the no-tools path is: the adapter receives the prompt body, calls the LLM directly via API, emits the response. Tool results would need to be pre-injected into context or omitted. The boot contract already supports this pattern conceptually (`MANAS_BOOT_MODE=minimal` with no MCP binding).

### per-harness adapter config shape (sketch)

| Harness | Session isolation | MCP injection | Token injection |
|---|---|---|---|
| CC | `--mcp-config /path/mcp.json --strict-mcp-config` | JSON file with headers | headers field |
| Gemini | Write `.gemini/settings.json` in session scratch dir, invoke from there | settings.json mcpServers | headers field |
| Codex | `CODEX_HOME=~/.manas/sessions/<id>/codex` | config.toml `[mcp_servers.X]` | `bearer_token_env_var` + env export |
| opencode | Project `opencode.json` in session scratch dir | opencode.json mcp section | headers field |
