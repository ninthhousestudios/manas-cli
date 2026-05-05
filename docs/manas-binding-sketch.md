# manas — binding layer architecture sketch

Status: sketch (updated 2026-05-03 — sutra has replaced qartez; report-vs-view split called out)
Date: 2026-04-29
Context: Identifies a gap in the manas architecture — there is no place where a concept that lives in chitta, sutra, kosha, and smriti is recognized as the same concept. This sketch proposes two projects that close that gap, ordered by which to build first. Companion to `docs/manas-architecture.md` and `docs/roadmap.md`.

**2026-05-03 update:** the `darshana` project is now understood as two surfaces, not one — a precomputed `darshana-report` (read by the agent at rich boot) and an interactive `darshana-view` (`manas concept "X"`). The report ships first; it has clearer success criteria and is easier to validate. Phase 5 of the current roadmap gates the entire darshana decision on the two experiments E1 (chitta path-resolution audit) and E2 (cross-tier-query frequency) defined in the roadmap and todo. The original sketch below remains accurate in spirit; treat references to "qartez" as referring to the in-house sutra.

## the question this answers

manas has four perception/comprehension tiers (smriti, sutra, kosha, chitta), each with its own ontology — paths/hashes, functions/calls, pages/citations, memories/decisions. Each is locally coherent. Across them there is no shared notion of *concept* — no place where "Saturn-Mars conjunction" in a kosha page, "transit_check()" in sutra, and a chitta decision about astrology UI are recognized as bearing on the same thing.

If manas is collaborative cognition, this missing piece matters. Cognition is not just perception + memory; it includes *binding* — the integration of representations across modalities into a single referent. This sketch is about whether to build that binding tier, and how.

## the gap: binding vs learning

Two distinct things often get conflated under "knowledge graph":

- **Binding.** Recognizing that a thing seen in one tier is the same thing seen in another. Periodic extraction, dedup-by-identity across tiers, mostly static graph. graphify-shaped.
- **Learning.** The graph is reweighted by which traversals were useful; edges decay if unvisited; new concepts emerge from co-activation. A research project, not a tool.

This sketch is about binding. Learning is named here only to keep it out of the build.

## the two projects

These look like one project but aren't. They have very different costs and very different failure modes.

### project A — darshana (unified view)

**One sentence:** A view that aggregates what the existing tiers already know and renders cross-tier relationships from edges those tiers already encode.

A UI/CLI affordance. Lives in manas-cli (and eventually manas-gui). Its inputs are chitta memories, sutra/qartez nodes, kosha books/pages, smriti paths/hashes. Its output is a navigable surface — a graph view, a side panel that floats while editing, a `manas concept "X"` query that shows everything across tiers that touches X.

**What's new:** the rendering and the joins.
**What's not new:** the underlying data; nothing about what each tier stores changes.

Build cost: weeks. Failure mode: nobody uses it twice.

### project B — the binding tier (name tbd: *prajna*, *samhita*, *medha*)

**One sentence:** A fifth tier whose nodes are *concepts*, whose edges are *cross-tier relations*, and which downstream tools can query as a first-class subsystem.

This is the hard knowledge-graph problem. It has to project sutra functions, smriti files, kosha pages, and chitta memories into one node space, with stable identity that survives all four tiers' edits. It owns its own database. It emits and consumes events from the other tiers. It exposes MCP tools (`prajna_concept`, `prajna_neighbors`, `prajna_explain`, etc.).

Build cost: months. Failure mode: a beautiful concept graph nobody queries that drifts out of sync with the tiers it summarizes.

Naming: open. Candidates carry different weights.
- **prajna** (प्रज्ञा) — integrative wisdom; cognitive-metaphor consistent with chitta/buddhi/manas; suggests insight rather than mere indexing.
- **samhita** (संहिता) — "joined together," a compilation; structurally accurate, less mystical.
- **medha** (मेधा) — retentive, integrating intelligence; close to "comprehension that binds."

## why darshana before prajna

graphify earns its keep through one sharp consumer: the PreToolUse hook that makes the assistant read `GRAPH_REPORT.md` before grepping. Without that, the graph would be a pretty visualization nobody opens. The lesson is not "build a graph"; it is **find the moment of use first, then build the smallest graph that serves it.**

For a manas concept layer, the test is whether you can name a moment of use that the existing tiers can't satisfy with a couple of joined queries. Candidates:

- "When I'm editing file X, surface chitta decisions, kosha passages, and smriti notes about the same concept."
- "Show me what's connected across my repos that I never noticed."
- "Navigate my second brain as a map, not as four separate query interfaces."

Almost all the edges these queries want **already exist implicitly** across the four DBs:
- a chitta memory mentions a path → smriti has that path → sutra parses that file's code
- a kosha book has a content_hash → smriti has that hash → chitta has decisions tagged with that hash
- a sutra symbol's source file is a smriti path that is referenced in a chitta session summary

darshana materializes those joins. If after a month of using darshana you keep wishing the edges were richer than naive joins produce, that is the signal to build prajna — and you will know exactly what shape it needs because you will have a list of "I wanted to see X but it wasn't there."

Building prajna first risks defining concepts before knowing which ones the system actually needs. darshana is the cheapest way to find out.

## project A in more detail — darshana

### what it owns

- A read-only join layer over the four tiers' MCP endpoints (or their underlying DBs, if a sideband path is faster).
- A renderer: graph (NetworkX → vis.js or similar), list, side-panel.
- A small set of CLI/GUI affordances: `manas concept "X"`, `manas around <path>`, `manas connect <a> <b>`.
- A one-page summary analogous to graphify's `GRAPH_REPORT.md` — god-nodes across the whole system, surprising cross-tier connections, suggested questions. Read by an agent on session start (rich boot).

### what it deliberately does *not* own

- Its own database. Every relationship it shows must be recoverable from the source tiers.
- Concept identity that the source tiers don't already have. If smriti says two paths share a content_hash, darshana can show that. It cannot decide that two differently-hashed PDFs are "the same book."
- Long-lived state. Caches are fine; authority is not.
- Embeddings. Use what's already there (kosha page embeddings, optional smriti shallow embeddings). Don't introduce a new model.

### the join surface

The minimum useful joins, none of which require a new data model:

| join | bridge | use |
|---|---|---|
| chitta memory ↔ smriti path | path string in `metadata` | "memories about this file" |
| chitta memory ↔ kosha book | content_hash | "what we've decided about this book" |
| smriti path ↔ sutra symbol | source_file equality | "what code lives at this path" |
| smriti hash ↔ kosha book | content_hash | "what's inside this PDF" |
| sutra symbol ↔ chitta decision | path string in `implemented_in` | "decisions that touched this code" |
| kosha page ↔ chitta citation | book + page tuple | "passages we've cited" |

These are the edges of darshana's graph. There is no extraction step. The "graph" is the materialized view of these joins for a given query.

### the moment-of-use bar

darshana ships when at least one of these is true and observed:
- The user opens it more than once a week without being prompted.
- An agent reads its session-start summary and references it in a non-trivial response.
- A query produces an answer none of the four tiers individually could.

If none of these happens after a month, the project is shelved. Better to know.

## project B in more detail — the binding tier

Built only if darshana surfaces durable demand for richer edges.

### what would justify it

darshana fails on questions like:
- "What concepts does this codebase share with that paper?" — concept-level, not path-level.
- "Where in my notes do I discuss the same idea that's in this code module?" — semantic identity across modalities.
- "What's the through-line in everything I've worked on this year about Vedic astrology?" — cross-domain conceptual clustering.

Naive joins cannot answer these. They need extracted concept nodes and inferred semantic edges with confidence labels.

### what it would own

- **Concept nodes.** Stable IDs, multi-tier provenance (a concept may have evidence in chitta, sutra, kosha simultaneously), aliases.
- **Cross-tier edges.** `mentioned_in`, `implemented_by`, `cited_at`, `same_as`, `semantically_similar_to`. Each tagged `EXTRACTED | INFERRED | AMBIGUOUS` with a confidence score. Borrowed verbatim from graphify's epistemic discipline.
- **Extraction pipeline.** Periodic LLM pass over new chitta memories, kosha pages, sutra symbols, smriti documents. Outputs concept nodes + cross-references. Cached by content_hash where possible.
- **Topology clustering.** Leiden or similar on the resulting graph — cheap, no embedding store, human-inspectable communities.

### what it deliberately does *not* own

- Authority over what files exist (smriti) or what code is shaped like (sutra) or what was decided (chitta). It is *derived* from those.
- A vector database. Embeddings, if used at all, come from kosha. Topology does the clustering.
- Real-time consistency. The binding tier is allowed to lag the source tiers; it announces its as_of and is_stale like the others.
- A claim of "learning." It extracts on a schedule. Reweighting by use is out of scope.

### the seam

The binding tier subscribes to the source tiers' event streams (smriti scan events, chitta write events, kosha ingestion events, sutra reindex events) and re-extracts incrementally. It exposes MCP tools and an event stream of its own that darshana then renders.

This means darshana doesn't change much when the binding tier arrives — it gains a richer source of edges, but its rendering and join model stay the same.

## graduation criteria — from darshana to prajna

Concrete signals to start building the binding tier:

1. **A list of unanswered questions.** Real ones, accumulated over weeks of darshana use. Not hypothetical.
2. **Repeated naive-join failure.** Specific cases where the path/hash/symbol bridges produced wrong or empty answers and a concept-level node would have helped.
3. **Cross-domain queries.** Demand for "what connects my notes about X to my code about Y" where X and Y do not share a file or hash.
4. **A user who asks for it.** If only the architecture wants prajna and the user doesn't, defer.

Until at least three of those land, prajna stays a sketch.

## reference — graphify (what landed and why)

Pulled into this sketch from graphify (`~/soft/graphify`):

- **Confidence triad on every edge** (`EXTRACTED | INFERRED | AMBIGUOUS` with score). Honest about found vs guessed. Should land in prajna verbatim, and probably in any cross-tier inference darshana ever does.
- **Topology clustering instead of embeddings.** Leiden over LLM-emitted similarity edges. Avoids a vector DB for the cross-document conceptual layer. Plausible default for prajna.
- **One-page summary as the always-on context.** graphify's `GRAPH_REPORT.md` is read by every grep call via a PreToolUse hook. darshana should produce an analogous `MANAS_REPORT.md` (god-concepts across tiers, surprising joins, suggested questions) consumed at session start.
- **Rationale-as-nodes.** graphify mines `# WHY:` / `# HACK:` / docstring rationale into first-class graph nodes. Independent of this sketch but worth lifting into sutra.
- **Single skill, many platforms.** graphify's `<platform> install` matrix is a packaging move worth mirroring once manas has something stable to distribute.

Not pulled:
- graphify's flat schema (no content addressing, paths as identity). manas's BLAKE3 model is stronger; don't regress.
- graphify's `add <url>` ingest path. Different product (Penpax). Not for manas.
- graphify's "one graph per project" assumption. manas is home-directory-scoped from the start.

## reference — graphify pipeline at a glance

```
detect → extract → build → cluster → analyze → report → export
```

- `detect`: filesystem walk + `.graphifyignore` (~smriti's job in manas).
- `extract`: tree-sitter for code (~sutra's job), Claude subagents for docs/papers/images (~kosha + a piece prajna would do).
- `build`: NetworkX merge.
- `cluster`: Leiden community detection (graph topology only).
- `analyze`: god nodes, surprising connections, suggested questions.
- `report`: GRAPH_REPORT.md.
- `export`: graph.json, graph.html, optional Obsidian/GraphML/Cypher; MCP server (`graphify --mcp`) exposing `query_graph`, `get_node`, `get_neighbors`, `shortest_path`.

## boundary cases

| case | who handles it | why |
|---|---|---|
| "What files mention Saturn?" | smriti FTS | concept-as-string search; no binding needed. |
| "What pages discuss Saturn?" | kosha | page-level retrieval. |
| "What does this codebase say about transits?" | sutra grep + kosha for any included docs | code structure + nearby prose. |
| "Show everything across my system that touches `bphs.pdf`" | darshana (project A) | naive joins on content_hash and path are sufficient. |
| "What concepts in my Vedic astrology notes also appear in my code?" | prajna (project B) | requires extracted concept nodes; no naive bridge. |
| "How has my model of Saturn evolved over time?" | chitta mental models | already chitta's job; darshana renders the timeline. |

## where this fits in the roadmap

darshana slots after the tiers it depends on are stable: **post phase 4** (chitta v0.0.4 + manas-cli foundation), and ideally after kosha exists in some form. It's small enough to be a sub-phase of manas-cli rather than a tier of its own.

prajna is **deferred** — moved to roadmap's deferred table with trigger "darshana surfaces a list of cross-tier questions naive joins can't answer."

## what this sketch is not

- An implementation plan. The next-steps doc would specify the actual join queries, the report-generation code, the MCP surface for darshana.
- A commitment to build prajna. This sketch deliberately argues for *not* building it yet.
- A renaming proposal. The Sanskrit names are placeholders; the user picks.
- A claim that manas needs a cognitive-learning layer. That's a separate ambition, named here only so it doesn't get smuggled in under "knowledge graph."
