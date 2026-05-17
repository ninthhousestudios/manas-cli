# manas-cli

[![License: AGPL v3](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](https://www.gnu.org/licenses/agpl-3.0)

Ops surface for the [manas](https://github.com/ninthhousestudios/manas) ecosystem. A single `manas` binary that ties the subsystems together for health checks, session lifecycle, and a composed HTTP MCP server.

## Install

```bash
cargo install --path .
```

## Commands

### `manas health`

Check connectivity to all manas subsystems (chitta, yojana, sangha, smriti, sutra).

### `manas warm [harness]`

Boot a rich session — loads memory, handoff context, and task state, then launches the specified harness. Supported harnesses: `claude-code` (default), `codex`, `gemini`, `opencode`.

### `manas done`

Session shutdown: store observations, write handoff, revoke bindings.

### `manas reflect`

Between-session maintenance: consolidate observations into mental models.

### `manas status`

Show active sessions, bindings, and lock state across subsystems.

### `manas install-services`

Install `~/.config/systemd/user/manas.service`, reload user systemd, and enable/start `manas serve` by default.

### `manas serve [-p PORT]`

Run the manas HTTP MCP server (default port 3000). Composes tools from multiple subsystems into a single MCP endpoint.

## Harness adapters

manas-cli includes adapter modules for launching different AI coding agents with the manas ecosystem pre-configured:

- **Claude Code** — default, streamable-HTTP MCP. Injects manas session-lifecycle instructions via `--append-system-prompt-file` so they apply to the top-level session but not to subagents.
- **Codex** — OpenAI Codex CLI
- **Gemini** — Google Gemini CLI
- **OpenCode** — open-source alternative

### Instruction split

Session-lifecycle instructions (sangha registration, chitta health checks, sutra/smriti tool preferences, yojana discipline, observation protocol) are compiled into the `manas` binary from `src/adapter/manas-instructions.md` and injected at launch via `--append-system-prompt-file`. This means:

- `manas warm claude` — top-level session gets full manas operating instructions + MCP servers
- `claude` (bare) — no manas instructions, no manas MCP servers
- Subagents spawned via the Agent tool — inherit MCP tool access but **not** the appended system prompt, so they won't perform session-lifecycle rituals

General-purpose instructions (personality, naming conventions, commit discipline) stay in `~/CLAUDE.md` and are visible to all sessions including subagents.

