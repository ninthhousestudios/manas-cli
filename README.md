# manas-cli

[![License: MPL 2.0](https://img.shields.io/badge/License-MPL_2.0-brightgreen.svg)](https://opensource.org/licenses/MPL-2.0)

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

### `manas serve [-p PORT]`

Run the manas HTTP MCP server (default port 3000). Composes tools from multiple subsystems into a single MCP endpoint.

## Harness adapters

manas-cli includes adapter modules for launching different AI coding agents with the manas ecosystem pre-configured:

- **Claude Code** — default, stdio MCP
- **Codex** — OpenAI Codex CLI
- **Gemini** — Google Gemini CLI
- **OpenCode** — open-source alternative

## License

MPL-2.0. See [LICENSE](LICENSE).
