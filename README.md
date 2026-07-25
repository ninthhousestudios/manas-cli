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

Session-lifecycle instructions (sangha registration, chitta health checks, sutra/smriti tool preferences, yojana discipline, observation protocol) are injected at launch via `--append-system-prompt-file`. This means:

- `manas warm claude` — top-level session gets full manas operating instructions + MCP servers
- `claude` (bare) — no manas instructions, no manas MCP servers
- Subagents spawned via the Agent tool — inherit MCP tool access but **not** the appended system prompt, so they won't perform session-lifecycle rituals

General-purpose instructions (personality, naming conventions, commit discipline) stay in `~/CLAUDE.md` and are visible to all sessions including subagents.

### Where the instructions are read from

Resolution happens at launch, not at compile time — editing the live file takes effect on the next `manas warm`, with no rebuild:

1. `$MANAS_INSTRUCTIONS`, if set.
2. `~/.manas/manas-instructions.md`. If it doesn't exist, `manas` seeds it from the copy compiled into the binary (`src/adapter/manas-instructions.md`), so the editable file is discoverable rather than opt-in.
3. The compiled-in copy, if neither is readable (a warning says why).

`manas warm` prints the source it used on the `prompt:` line, and the injected text ends with a provenance comment naming the source path, its mtime, and a content hash — so a running session can answer "which instructions am I running?" by reading its own system prompt. The exact bytes injected are also written to `~/.manas/sessions/<id>/manas-instructions.md`.

Symlinks are read through, and both the `prompt:` line and the provenance comment name the resolved target rather than the link. A dangling link falls back to the compiled-in copy with a warning instead of being re-seeded — seeding would follow the link and write through to a checkout that has moved.

### Recommended dev setup

Point the live path at your checkout, so the committed file is what actually runs and git stays the source of truth:

```sh
ln -s "$PWD/src/adapter/manas-instructions.md" ~/.manas/manas-instructions.md
```

Without this, the seeded copy at `~/.manas/manas-instructions.md` diverges silently: edits to `src/adapter/manas-instructions.md` would then only affect fresh installs. `$MANAS_INSTRUCTIONS` achieves the same thing per-process, but the symlink applies to every `manas` invocation regardless of environment.

