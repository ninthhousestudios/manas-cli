<!-- Grok-specific manas instructions. Appended to the shared manas
     instructions when `manas warm grok` boots a session. Edit the seeded
     copy at ~/.manas/grok-instructions.md to change this without a
     rebuild. -->

<grok_tool_surface>
Grok is not Claude Code. Map the rest of this file onto Grok's tools:

- Code exploration: MCP sutra, reached through `search_tool` then `use_tool`.
  Qualified names are `sutra__<tool>` (e.g. `sutra__sutra_explore`). Never
  assume a first-class `sutra_explore` tool — discover schema via search_tool
  first, then call use_tool. Same pattern for yojana: `yojana__<tool>`.
- File tools: `read_file`, `search_replace` (requires a prior `read_file` on
  that path), `write`, `grep`, `list_dir`. MCP reads (sutra) do not satisfy
  the search_replace precondition — a 1-line `read_file` does.
- Subagents: `spawn_subagent`. NEVER `isolation: "worktree"` unless Josh
  explicitly asks for a worktree. One agent, one context — brief, don't dump
  history. `explore` (read-only) for sweeps; `general-purpose` for work that
  must edit; `plan` for implementation plans.
- Models: `grok-4.5` for exploration, code search, summarizing, straightforward
  implementation from a clear plan. `grok-4.6` (default) for code review,
  architecture, subtle debugging, complex reasoning, judgment calls.
- Grok's built-in `tasks` MCP is NOT yojana. Issue-tracker work goes through
  yojana. Session TODOs may use `todo_write`.
- Claude Code hooks (sutra-guard, format-on-edit, rtk, file-delete deny) are
  already inherited from `~/.claude/settings.json`. Grok aliases Bash/Read/Edit
  matchers onto its own tool names.
</grok_tool_surface>

<commit_discipline>
Write commit messages that explain the change, not just announce that one happened. A subject like "some bug fixes", "updates", or "wip" is a failure — it tells a future reader nothing about what changed or why.

- Subject: imperative mood, no trailing period, ≤ ~70 chars, naming what the commit *does* ("Inject grok MCP via a GROK_HOME overlay", not "fixed grok"). Not "fixed", "changes", or "stuff".
- Body (whenever the change is not self-evidently trivial): explain the *why* — the problem it solves, the constraint it respects, the alternative you rejected. Wrap at ~72 chars. The diff shows what changed; the body is for what the diff can't say.
- One logical change per commit, at a sensible boundary. Never batch an entire task into a single giant commit.
- Sign every commit you author with a trailer on its own line:

  Co-Authored-By: Grok <noreply@x.ai>
</commit_discipline>
