## First Step — Manas Wake-Up

**At the start of every conversation, do this first:** call `manas_wake_up` with `project` set to the current project slug/name and `workspace_path` set to `$CWD` when available. This loads Josh's Chitta profile, current Yojana context, and refreshes/registers the Sutra code index before any other exploration.

## Yojana — Issue Tracker

Yojana is a local MCP-based issue tracker running as a systemd user service on port 4200. It provides task management with state machines, edges (dependencies/relations), and context shapes for agent workflows.

- Service: `systemctl --user {start|stop|status} yojana`
- Binary: `~/.cargo/bin/yojana` (built from `~/soft/manas/yojana`)
- DB: `~/.yojana/yojana.db`
- MCP endpoint: `http://127.0.0.1:4200/mcp`
- Tools: `yojana_project`, `yojana_task`, `yojana_edge`, `yojana_query`, `yojana_ready`, `yojana_context`

### Triage discipline

When tasks come out of an explicit triage process (a review, a decompose, a planning session), set the status accurately on creation rather than letting `needs-triage` default. **`needs-triage` means *untriaged*, not *just created*.**

Status by slice_type:

| Task is | Status on creation |
|---|---|
| AFK and ready to execute | `ready-for-agent` |
| HITL and ready for human attention (design Q, grilling, review) | `ready-for-human` |
| Genuinely unsorted, scope unclear | `needs-triage` |
| Waiting on a question/clarification from a human | `needs-info` |
| Actively in flight | `in-progress` |

Full status enum and transitions: `~/soft/manas/yojana/README.md` § "Status model."

### Project handoffs

Project handoffs live in the yojana project's `handoff` field, not in `docs/handoff.md` on disk. Update via:

```
yojana_project action=update slug=<project> handoff="..."
```

Content: where the project IS (state, in-progress streams, recent landings) plus pointers to next-up tasks by `slug/N`. Keep it tight — readers fetch via `yojana_project action=get`. Cross-project queues belong in tracking, not in any one project's handoff.

If a project still has a legacy `docs/handoff.md`, archive prior content to `.handoffs/{datetime}.md` (git-tracked, project root) and replace `docs/handoff.md` with a one-screen pointer at the yojana queries.

### Tracking across streams

`yojana_query status="in-progress"` (cross-project, omit `project=`) returns everything actively underway. Cap at 3-5 in-progress at a time — more means you've started things you haven't finished and the list lies.

For a hand-curated "next up" lane across projects, tag tasks with `now` and query with `yojana_query tag="now"`.

## Sutra MCP

`manas_wake_up` calls `sutra_status` for the workspace at session start. Call `sutra_status` directly later if the workspace changes or you need a fresh status check. Use `sutra_add_root` only when you need to force a reparse.

Use sutra tools instead of built-in file tools for code exploration:

| Instead of | Use |
|---|---|
| Glob / find | `sutra_map` |
| Grep / rg | `sutra_grep` or `sutra_find` |
| Read (code) | `sutra_read` |

Before editing a load-bearing file, call `sutra_impact` first.
Use `Glob`/`Grep`/`Read` only for non-code content.
If a built-in code tool is denied by the guard, use the sutra equivalent.

## Smriti MCP

For non-code file searches (documents, configs, data files), prefer smriti over shell commands:

| Instead of | Use |
|---|---|
| `find` / `ls` | `smriti_find` with `path` (glob) or `ext` |
| `grep` (content) | `smriti_find` with `query` |
| `cat` / `head` | `smriti_read` |

`smriti_find` with `path` is particularly useful for locating files by name — e.g., `smriti_find(path="**/sync.sh")` returns all matching files across indexed roots. Faster and more targeted than shell `find` since it queries the index.

If smriti returns "database disk image is malformed", the FTS/vec virtual tables may be corrupted. Fall back to shell commands and file a smriti bug.

## Sangha — Session Coordination

After `manas_wake_up`, if sangha tools are available, call `mcp__sangha__session_register` with `project` set to `$CWD` and `branch` from git.

## Chitta — Working Model of Josh

Chitta is the working model of Josh — what he values, how he works, what he prefers, what mental models he uses. Not a general memory store.

- Service: `systemctl --user {start|stop|status} chitta`
- Binary: `~/.cargo/bin/chitta` (built from `~/soft/manas/chitta`)
- DB: `postgresql://localhost/chitta`
- MCP endpoint: `http://127.0.0.1:3100/mcp`
- Tools: `mcp__chitta__health_check`, `store_memory`, `get_memory`, `search_memories`, `update_memory`, `delete_memory`, `list_recent_memories`

`manas_wake_up` calls `get_profile` at session start to load the always-on profile (top ~30 working-model entries by effective score). If Chitta appears unavailable, call `mcp__chitta__health_check`; if it fails, **immediately tell Josh**. If Josh gives a Chitta memory id, use `get_memory`; prefixes work. For context-specific retrieval, use `search_memories` with `applies_to` facets (domains, skills, projects, situations).

### What goes in Chitta

Only content that models Josh as a person — observations, decisions, episodes, and consolidated traits/values/preferences/patterns/mental_models.

**Observations** — 1-3 sentence notes about Josh's preferences, values, corrections, or patterns. `memory_type: "observation"`, `profile: "josh"`. Store proactively during sessions (see "During-Session Observations" below).

**Decisions** — only when they carry working-model signal (about Josh's values, preferences, patterns). Must supply `metadata.rationale` (non-empty) and `metadata.rejected_alternatives` (>= 1 entry). Project-artifact decisions ("we picked Postgres for chitta") belong in yojana.

**Episodes** — session-bounded units written by the `done` skill, with `derivations` pointing at the session's observations.

**NOT doc summaries** — the doc on disk is the source of truth.

**NOT project handoffs** — those live in yojana's `handoff` field.

**NOT project-artifact decisions** — route to yojana.

**NOT domain knowledge** — that belongs to vidya (planned).

### What goes in docs/

Living artifacts — specs, plans, principles. Git-tracked, human-editable. Maintain `docs/index.md` as a manifest when creating or moving docs.

### What goes in CLAUDE.md

Only agent operating instructions. Not project knowledge, not decisions.

## CLAUDE.md Policy

When editing or creating CLAUDE.md files, `search_memories query:"CLAUDE.md policy"` in Chitta first. Full policy lives there.

## During-Session Observations

During sessions, proactively store observations in Chitta without being asked.
Use `store_memory` with `memory_type: "observation"`, `profile: "josh"`,
and topical tags. Keep content to 1-3 sentences.

**Store when:**
- Josh corrects something or pushes back (captures preferences/values)
- An approach is tried and fails (negative knowledge)
- A non-obvious constraint or requirement surfaces
- Something would be hard to reconstruct from the transcript alone

**Decisions:** Only use `memory_type: "decision"` when the decision carries
working-model signal (about Josh's values, preferences, patterns) AND you can
supply `metadata.rationale` (non-empty string) and
`metadata.rejected_alternatives` (array with >= 1 entry). The server
hard-rejects decisions missing these fields. Project-artifact decisions
("we picked Postgres for chitta") belong in yojana, not chitta. When in
doubt, demote to `observation`.

**Don't store:**
- Routine code changes with no design significance
- Things already captured in docs or code
- Trivial exchanges
- Content that merely restates what's in the transcript
- Project-artifact decisions (route to yojana instead)

No announcement, no asking permission. Just include the store_memory call
alongside the normal response.
