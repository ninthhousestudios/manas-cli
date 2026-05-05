# freshness envelopes

Status: principle
Date: 2026-04-26
Source: gemini-flash review (`gemini-flash-suggestions.md`), endorsed cross-subsystem.

## the rule

Every read tool across manas subsystems (chitta, smriti, qartez, sangha) MUST return two fields on its response envelope:

- `as_of: i64` — Unix milliseconds. The timestamp the underlying data was last refreshed/indexed/observed. **Not** the time of the call.
- `is_stale: bool` — server's judgment that the data is older than the subsystem's freshness threshold.

Callers (LLMs and humans) need to know whether they are reasoning about a snapshot from three minutes ago or three days ago. Without these fields, agents silently make decisions on stale data and there is no way for downstream code (or another agent) to detect it.

## why

Manas is a federation of pull-shaped read servers. None of them push updates. The cost of keeping all subsystems perfectly fresh is too high; the cost of letting an agent silently use stale data is also too high. Freshness envelopes are the cheap middle path: the server is honest about its age, the caller decides what to do.

This is also the reason we can defer sangha's event-bus / push-notification work (see `sangha/docs/defer-until-required.md`). Pull + freshness envelopes gets ~90% of the value at ~10% of the complexity of push.

## what `as_of` means per subsystem

| Subsystem | `as_of` source |
|---|---|
| **chitta** | `record_time` of the most recent write touching the returned data, OR query time if the read is over fully-current state. |
| **smriti** | Last scan time for the path/root the result came from (per-root, not global). |
| **qartez** | Index build time for the workspace the result came from. |
| **sangha** | Server time at response (state is real-time), plus `last_heartbeat` per session in `list_sessions`. |

## what `is_stale` means

Each subsystem defines its own threshold. Suggested defaults:

- **chitta:** never stale by this definition (memory is the source of truth). Always `false`. Field still present for envelope uniformity.
- **smriti:** `true` if last scan of this root is older than 24h (configurable per root).
- **qartez:** `true` if the index is older than the most recent commit on the workspace's git HEAD.
- **sangha:** `true` for a session if `last_heartbeat` is older than `2 * heartbeat_interval`.

These are starting points. Each subsystem owns its own threshold and may expose configuration.

## per-operation response tier (2026-05-03 addendum)

The principle as originally written outsources the response to `is_stale: true` to the caller (the LLM). LLMs are bad at invalidating their own context, so the architecture review ([`docs/arch-review-2026-05-03.md`](../../docs/arch-review-2026-05-03.md)) introduces a per-operation response tier:

| Tier | Behavior on `is_stale=true` |
|---|---|
| 1. Announce | Return data + flag. Caller decides. |
| 2. Refuse | Return error; require caller to trigger refresh. **Withhold the content** so the LLM can't anchor. |
| 3. Self-heal | Refresh in-band before returning. |

Per operation, suggested defaults:

| Subsystem | Operation | Tier |
|---|---|---|
| chitta | all | 1 (chitta is never stale by definition) |
| sangha | all | 1 (real-time) |
| sutra | `read` | **2** — returning a function whose file changed is dangerous |
| sutra | `map`, `grep`, `outline`, `find` | 1 |
| sutra | `impact`, `deps`, `calls` | 1 with caveat |
| smriti | `read` | **2** — content may not match indexed metadata |
| smriti | `find`, `map` | 1 |
| smriti | `scan` | 3 (self-heals by definition) |
| kosha (planned) | `search`, `read` | 1 (segment text is the working copy) |

Subsystems update their MCP responses to honor tier 2 by sending the staleness signal *instead of* the content, not alongside it. This makes the in-context-anchoring failure mode structurally impossible.

## non-goals

- This is not a cache-invalidation protocol. Callers don't refresh the server; they just decide whether to trust the answer.
- This is not a push/notify channel. If a caller needs fresher data they re-call.
- This is not a freshness *guarantee*. `is_stale: false` means "the server believes this is fresh"; it is still a snapshot.

## conformance tasks per subsystem

- **chitta:** add `as_of` and `is_stale` to all read responses (`get_memory`, `search_memories`, `list_recent_memories`, `health_check`).
- **smriti:** include in tool response envelopes from day one (still pre-implementation, cheap to bake in). See `smriti/docs/handoff.md`.
- **qartez:** external, request via upstream — at minimum index-build-time should be queryable.
- **sangha:** track in `sangha/docs/todo-next.md` alongside `list_sessions`.

## related principles

- **Mental model retirement** (chitta) — `superseded_by` / `retired_at` fields so old models don't compete with fresh ones in retrieval. Same family of problem (age-aware reasoning), different mechanism.
- **Sideband sync** between smriti and chitta — keeps freshness *higher* by propagating events; complementary, not a replacement for envelopes.
