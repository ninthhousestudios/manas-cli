<sutra_mcp>
For code, use sutra instead of built-in file tools: Glob/find → `sutra_map`, Grep/rg → `sutra_grep`, Read → `sutra_read`. Built-in Glob/Grep/Read are for non-code content only — if the guard denies a built-in code tool, use the sutra equivalent.

Exploration protocol (all agents, subagents included):
1. Any symbol, component, or domain term → `sutra_explore` first. It resolves `.sutra/aliases.toml` domain terms as its top tier, falls through to exact lookup for qualified names (`::`), and returns ranked symbols with literal `sutra_read` fetch instructions plus a strategy hint (read_top_n / read_all / narrow_query / explore_component) — follow the hint rather than reasoning about navigation yourself.
2. `sutra_grep`/`sutra_map` only when explore returns nothing or the query is a literal string pattern.
3. Never `sutra_read` a guessed symbol name — discover it in step 1.

`sutra_workspace(path=)` verifies freshness (`action="reparse"` forces a reparse). `sutra_impact` before editing a load-bearing file. `sutra_context(symbol)` packs a symbol's deps + dependents into a token budget — ideal for subagent briefings.

Lessons (`~/.sutra/lessons.db`, cross-project; anchored to technologies and patterns, not projects, so a lesson surfaces wherever its anchors match):
- **Store**: `sutra_remember(text, anchors)` — anchors are symbol names or file paths; sutra auto-enriches with import patterns and category tags. Store hidden constraints, non-obvious invariants, and failure modes a future editor needs. Not routine facts already visible in the code.
- **Surface**: lessons appear inline in `sutra_read`, `sutra_impact`, and `sutra_orient` when anchors match; `sutra_lessons(query=)` searches explicitly — run it before writing a new module or component.
- **Cite**: `sutra_remember(cite="<lesson_id>", source_tasks=["<task_id>"])` when closing a task that validated one. Citations build confidence; uncited lessons decay and are archived.
</sutra_mcp>

<coding_discipline>
`[enforced]` = a sutra forbidden_pattern rule in vidhi/language-rules/; the guard blocks or warns at edit time. `[analyzer]` = shared toolchain baseline (Rust workspace `[lints.clippy]`; Dart house analysis_options.yaml, canonical copy in vidhi/vidhi-dart/). `[prose-only]` = you are the only enforcement.
<rust>
No Clone-Driven Development: never `.clone()`/`.to_owned()` to bypass the borrow checker — design reference structures (`&`, `&mut`) and explicit lifetimes (`'a`) instead. [enforced]
Prefer Slices Over Containers: read-only arguments take `&str`/`&[T]`, never `String`/`Vec<T>`. Don't make the caller heap-allocate just to pass an argument. [prose-only]
Avoid Premature Dynamic Allocation: don't reach for `Box<dyn Trait>` or `Arc<Mutex<T>>` to escape generic constraints or lifetime bounds. Static dispatch (`impl Trait`, generics) unless a heterogeneous collection or true runtime dispatch demands otherwise. [prose-only]
Graph & Tree Semantics: traversing graphs, trees, or stacks (compilers, trackers), thread lightweight identifiers (`i64`, internal indices) through recursive scopes — not heap-allocated strings or cloned state structs. [prose-only]
Panic Discipline: recoverable failures propagate (`Result`, `?`); violated invariants panic via `.expect("invariant: ...")` naming what would have to break. Never bare `.unwrap()` outside tests. [enforced + clippy unwrap_used]
No Lint Silencing: never `#[allow(...)]` to quiet a lint — fix it, use `#[expect(lint, reason = "...")]`, or waive via sutra with rationale. `unsafe` blocks state their invariant as the waiver rationale. [enforced]
</rust>

<dart/flutter>
Strict Typing: no `dynamic` or implicit `Object` to escape strict typing — use generics (`<T>`) or explicit interface abstractions. [enforced]
Enforce Const Constructors: maximize `const` variables and constructors. In UI compilation paths and structural collections, omitting `const` costs heap allocations and breaks rendering optimizations. [analyzer]
Defensive Null Safety: don't spam the `!` null-assertion operator to clear type errors — use `??`, `?.`, or an explicit `if (x != null)` block to establish safe execution tracks. [enforced]
Cascade & Collection Operators: use cascades (`..`) and collection-if/collection-for instead of imperative boilerplate to build and mutate maps and lists. [analyzer]
No Lint Silencing: never `// ignore:` or `// ignore_for_file:` to quiet a diagnostic — fix it or waive via sutra with rationale (Dart has no `#[expect]` analog). [enforced]
</dart/flutter>

Types-first gate: for a non-trivial new Rust/Dart unit (module or subsystem with a real data model), Josh may invoke `vidhi-types-first` — type skeleton (data types, signatures, error taxonomy, no bodies) reviewed before implementation. When a task qualifies and it wasn't invoked, suggest it in one line; don't start it unbidden.
</coding_discipline>

<smriti_cli>
For non-code files (docs, configs, data), prefer `smriti find --path <glob>` over shell `find` — much faster.
</smriti_cli>

<yojana_issue_tracker>
Local MCP issue tracker (tasks, state machines, edges, context shapes); systemd user service. Projects nest: `sutra/needs-designing` is a subproject of `sutra`, `adityas/site` of `adityas`.

<triage_discipline>
When tasks come from an explicit triage process (review, decompose, planning), set status accurately on creation. `needs-triage` means *untriaged*, not *just created*. Status by slice_type:
- AFK, ready to execute → `ready-for-agent`
- HITL, ready for human (design Q, grilling, review) → `ready-for-human`
- Genuinely unsorted, scope unclear → `needs-triage`
- Waiting on human clarification → `needs-info`
- Actively in flight → `in-progress`
Full enum and transitions: `~/soft/manas/yojana/README.md` § "Status model."
</triage_discipline>

<execution_discipline>
"Do <project>/<N>" means: `yojana_task action=get`, then **immediately** `yojana_task action=update status="in-progress"` — before any exploration, edit or delegation. Not at the end, not batched into the closing update.

This is not bookkeeping. The status machine has no direct edge from `ready-for-agent`/`ready-for-human` to `done`, so a close that skips it is rejected and costs a wasted round trip; `in-progress` is also the only signal that the work is underway while it is underway (see stream_tracking).
</execution_discipline>

<capture_discipline>
Close-out fields are mined by vidhi-reflect for cross-project lessons — write them for a reader with no transcript.
- Closing a `bug`: root_cause is REQUIRED — the mechanism (why it broke), 1-3 sentences, not a restatement of the fix; the fix goes in execution_record. Genuinely unknown → "unknown:" plus what was ruled out.
- execution_record when execution diverged from plan: failed approaches, surprises, workarounds. Uneventful execution needs no record — "went as planned" entries dilute mining.
- `wontfix` requires a closing comment saying why — rejected approaches are negative knowledge worth as much as fixes.
- Set category at creation (bug/enhancement/experiment). A bug found and fixed mid-review is still category=bug.
- decisions entries carry rationale and the strongest rejected alternative.
- done means landed. Branch unmerged, service not redeployed, or a verification step pending at close? Say so in execution_record AND file the follow-up task — closing over silent pending work is the gap (yojana/32-33, justifier/1, swisseph.dart/2).
</capture_discipline>

<stream_tracking>
`yojana_query status="in-progress"` (omit `project=` for cross-project) returns everything underway — cap at 3-5; more means you've started things you haven't finished and the list lies. For a hand-curated "next up" lane, tag tasks `now` and query `yojana_query tag="now"`.
</stream_tracking>
</yojana_issue_tracker>

<artifact_routing>
- Living artifacts (specs, plans, principles) → git-tracked, human-editable `docs/`
- Agent operating instructions → `CLAUDE.md` only (never project knowledge or decisions)
</artifact_routing>
