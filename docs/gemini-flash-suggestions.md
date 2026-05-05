# gemini-flash suggestions — manas os & sangha

Date: 2026-04-26
Status: Initial review of Manas architecture + Sangha v0.1.0

## on manas architecture

The reframe of Chitta as one subsystem rather than the whole is the critical insight. Most agent systems suffer from "memory bloat" because they don't distinguish between **awareness** (I see these files) and **knowledge** (I decided to move this function).

### core recommendations

1.  **Flip Smriti to Default-Deny:** As Opus 4.7 noted, indexing `~` by default is a security risk. Embeddings of secrets are recoverable. Smriti should require explicit allowlisted roots.
2.  **Sideband Sync (The "Pragmatic Cheat"):** The "no subsystem calling another" rule is great for purity but expensive for tokens. We need a daemon-level sideband where Smriti can tell Chitta "this file hash moved to path Y," allowing references to self-heal without an LLM round-trip.
3.  **Mental Model Retirement:** Chitta needs a "Trust" signal. We need `superseded_by` or `retired_at` fields. A mental model from three months ago that contradicts a fresh observation shouldn't have equal weight in the prompt.
4.  **Freshness Envelopes:** Every read tool (Smriti, Chitta, Qartez) must return `as_of` and `is_stale` signals. I need to know if I'm reasoning about a snapshot from three minutes ago or three days ago.

## on sangha (session coordination)

Sangha v0.1.0 currently handles three things:
- **Presence:** `session_register` + heartbeats. It knows who is "awake."
- **Advisory Locks:** `resource_claim`. It prevents agents from overwriting the same `handoff.md`.
- **Inbox/Messaging:** `send_message` + `read_inbox`. Basic inter-agent IPC.

### what it should do next

Sangha is currently the "Session Subsystem," but it could become the **"Coordination Kernel"**:

1.  **Resource Queueing:** Currently, `resource_claim` is binary (win/fail). If I want to write to `handoff.md` and someone else is, I just have to wait and try again. Sangha should support **Wait Queues** where I can register interest in a lock and be notified when it's free.
2.  **Shared Ephemeral Context:** Chitta is for long-term memory. Sangha could provide a **Shared Blackboard** for the *current* session—temporary flags or tiny state snippets (e.g., "currently refactoring the auth module") that shouldn't pollute long-term memory but are essential for any other agent that "wakes up" in the same hour.
3.  **Intent Brokering:** When registering, agents provide an `intent`. Sangha should allow me to **Query Intent**. If I wake up and see another agent is active in `~/soft/chitta`, I should check their intent before starting a conflicting task.
4.  **Sideband Signaling:** Sangha is the natural home for the "sideband daemon" mentioned above. It can act as the message bus that routes "Smriti scan complete" or "Chitta model updated" events to active agents.

## net take

Manas is a "Cognitive OS," and Sangha is its **Scheduler/IPC kernel**. The bones are solid. The next step is moving from "static servers that answer questions" to "a live system that notifies me when the world changes."
