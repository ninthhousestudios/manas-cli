<sutra_mcp>
Use sutra tools instead of built-in file tools for code:
- Glob/find → `sutra_map`
- Grep/rg → `sutra_grep` or `sutra_find`
- Read (code) → `sutra_read`
Run `sutra_status` first to verify workspace freshness; `sutra_add_root` only to force a reparse. Call `sutra_impact` before editing a load-bearing file. Built-in Glob/Grep/Read are for non-code content only — if the guard denies a built-in code tool, use the sutra equivalent.
When exploring an unfamiliar area of the codebase, start with `sutra_explore(query, workspace)` — one call replaces iterative map/outline/grep cycles. It returns a ranked symbol list with literal `sutra_read` fetch instructions and a strategy hint (read_top_n, read_all, narrow_query, explore_component). Use the strategy to decide your next action rather than reasoning about navigation yourself. For qualified symbol names (containing `::`), it falls through to exact lookup automatically.
Projects can define human-readable aliases for components, files, and symbols in `.sutra/aliases.toml`. Use `sutra_resolve` to look up domain terms (e.g. "being detail cards") → code locations. Check for an aliases file before doing broad searches for a domain concept.

sutra_read discipline: always discover the symbol name before reading. Use `sutra_explore` or `sutra_grep` first — don't guess names. Guessed names fail often (e.g. `Db::save_snapshot` when it's actually `Db::insert_snapshot`). Explore-then-read is one extra call; guess-and-miss is two calls plus a red error.

Lessons system: sutra maintains a cross-project lessons store (~/.sutra/lessons.db) for code-anchored knowledge that future editors need.
- **Store**: `sutra_remember(text, anchors)` — anchors are symbol names or file paths where the lesson applies. Sutra auto-enriches with import patterns and category tags.
- **Surface**: lessons appear inline in `sutra_read`, `sutra_impact`, and `sutra_orient` when anchors match. `sutra_lessons` does explicit search.
- **Cite**: when closing a task that validates a lesson, call `sutra_remember(cite="<lesson_id>", source_tasks=["<task_id>"])`. Citations build confidence; uncited lessons decay and are eventually archived.
- **Scope**: lessons attach to technologies and patterns, not projects. A lesson learned in one workspace surfaces wherever its anchors match.
When you learn something a future editor of this code needs to know — a hidden constraint, a non-obvious invariant, a failure mode — store it with `sutra_remember`. Don't store routine facts already visible in the code.
</sutra_mcp>

<smriti_cli>
For non-code files (docs, configs, data), prefer smriti over shell:
- find/ → `smriti find --path (glob)` - much faster than `find`
</smriti_cli>

<yojana_issue_tracker>
Local MCP issue tracker (tasks, state machines, edges, context shapes). systemd user service.
- Service: `systemctl --user {start|stop|status} yojana` | Binary: `~/.cargo/bin/yojana` | DB: `~/.yojana/yojana.db` | Endpoint: `http://127.0.0.1:4200/mcp`
- Tools: yojana_project, yojana_task, yojana_edge, yojana_query, yojana_ready, yojana_context
- Yojana has subprojects. e.g., sutra/needs-designing is a subproject of sutra; adityas/site is a subproject of adityas.

<triage_discipline>
When tasks come from an explicit triage process (review, decompose, planning), set status accurately on creation. `needs-triage` means *untriaged*, not *just created*. Status by slice_type:
- AFK, ready to execute → `ready-for-agent`
- HITL, ready for human (design Q, grilling, review) → `ready-for-human`
- Genuinely unsorted, scope unclear → `needs-triage`
- Waiting on human clarification → `needs-info`
- Actively in flight → `in-progress`
Full enum and transitions: `~/soft/manas/yojana/README.md` § "Status model."
</triage_discipline>

<capture_discipline>
Close-out fields are mined by vidhi-reflect for cross-project lessons — write them for a future reader with no transcript.
- Closing a `bug`: root_cause is REQUIRED — the mechanism (why it broke), 1-3 sentences, not a restatement of the fix. Genuinely unknown → write "unknown:" plus what was ruled out. The fix itself goes in execution_record.
- Closing any task where execution diverged from plan: record the divergence in execution_record — failed approaches, surprises, workarounds. Uneventful execution needs no record; "went as planned" entries dilute mining.
- `wontfix` requires a closing comment saying why — rejected approaches are negative knowledge worth as much as fixes.
- Set category at creation (bug/enhancement/experiment). A bug found and fixed mid-review is still category=bug.
- decisions entries carry rationale and the strongest rejected alternative.
- done means landed: if the branch is unmerged, the service not redeployed, or a verification step pending at close, say so in execution_record AND file the follow-up task. Closing over silent pending work is the gap (yojana/32-33, justifier/1, swisseph.dart/2).
</capture_discipline>

<project_handoffs>
Handoffs live in the yojana project's `handoff` field, NOT `docs/handoff.md`. Update via `yojana_project action=update slug=<project> handoff="..."`. Content: where the project IS (state, in-progress streams, recent landings) plus next-up pointers by `slug/N`. Keep tight — readers fetch via `yojana_project action=get`. Cross-project queues belong in tracking, not one project's handoff. If a legacy `docs/handoff.md` exists, archive its content to `.handoffs/{datetime}.md` (git-tracked, project root) and replace it with a one-screen pointer to the yojana queries.
</project_handoffs>

<stream_tracking>
`yojana_query status="in-progress"` (omit `project=` for cross-project) returns everything underway — cap at 3-5; more means you've started things you haven't finished and the list lies. For a hand-curated "next up" lane, tag tasks `now` and query `yojana_query tag="now"`.
</stream_tracking>
</yojana_issue_tracker>

<chitta_josh_model>
Working model of Josh — values, preferences, patterns, mental models. NOT a general memory store. systemd user service.
- Service: `systemctl --user {start|stop|status} chitta` | Binary: `~/.cargo/bin/chitta` | DB: `postgresql://localhost/chitta` | Endpoint: `http://127.0.0.1:3100/mcp`
- Tools: mcp__chitta__health_check, get_profile, store_memory, get_memory, search_memories, update_memory, delete_memory, list_recent_memories
- `get_profile` loads the model — only run when told directly.
- If Chitta seems unavailable, call `mcp__chitta__health_check`; if that fails, **immediately tell Josh**.
- Given a memory id, use `get_memory` (prefixes work). For context-specific retrieval, `search_memories` with `applies_to` facets (domains, skills, projects, situations).

<what_goes_in_chitta>
Only content modeling Josh as a person.
- Observations — 1-3 sentence notes on preferences, values, corrections, patterns. `memory_type:"observation"`, `profile:"josh"`. Stored proactively (see during_session_observations).
- Decisions — only with working-model signal. MUST supply non-empty `metadata.rationale` and `metadata.rejected_alternatives` (≥1); the server hard-rejects otherwise. When in doubt, demote to observation.
- Episodes — session-bounded units written by the `done` skill, with `derivations` pointing at the session's observations.
NOT: doc summaries (disk is source of truth), project handoffs (→ yojana), project-artifact decisions like "we picked Postgres for chitta" (→ yojana), domain knowledge (→ vidya, planned).
</what_goes_in_chitta>

<during_session_observations>
Proactively store observations without being asked — no announcement, no permission. `store_memory` with `memory_type:"observation"`, `profile:"josh"`, topical tags, 1-3 sentences. Store when:
- Josh corrects something or pushes back (preferences/values)
- An approach is tried and fails (negative knowledge)
- A non-obvious constraint or requirement surfaces
- It would be hard to reconstruct from the transcript alone
Don't store: routine code changes, things already in docs/code, trivial exchanges, content that just restates the transcript, project-artifact decisions.
</during_session_observations>
</chitta_josh_model>

<artifact_routing>
- Living artifacts (specs, plans, principles) → git-tracked, human-editable `docs/`
- Agent operating instructions → `CLAUDE.md` only (never project knowledge or decisions)
</artifact_routing>

<engineering_lessons>
See the lessons system section in `<sutra_mcp>` above. Short version: `sutra_remember` to store, `sutra_remember(cite=...)` to cite on task close-out.
</engineering_lessons>
