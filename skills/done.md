# Done — Session Wrap-Up

**Execute this workflow. Do not just describe it.**

## Steps

### Step 1: Identify the transcript path

The transcript path is available as `$MANAS_TRANSCRIPT_PATH`.
If it is unset or the file does not exist, note this and continue — the
session summary will lack a transcript pointer.

### Step 2: Review for missed observations

Scan the conversation for things worth storing as observations that weren't
captured during the session:

- Decisions made (with rationale and what was rejected)
- Corrections or pushback from Josh (captures preferences/values)
- Approaches tried and failed (negative knowledge)
- Non-obvious constraints or requirements discovered

For each, call `store_memory` with:
- `memory_type: "observation"`
- `profile: "chitta"`
- Tags for the topic
- 1-3 sentence content

### Step 3: Generate session summary

Create a session summary covering:
- **What was worked on** — the main task(s) or question(s)
- **Outcomes** — what was accomplished, what changed
- **Decisions made** — reference any decision-type memories stored
- **Open threads** — what's unfinished or needs follow-up
- **Transcript pointer** — `$MANAS_TRANSCRIPT_PATH` value from Step 1

Call `store_memory` with:
- `memory_type: "session_summary"`
- `profile: "chitta"`
- `tags: ["session-summary"]` plus topic tags
- `event_time`: current time

### Step 4: Write handoff

If `docs/handoff.md` exists, archive it first:
```bash
mkdir -p .handoffs
cp docs/handoff.md ".handoffs/$(date +%Y-%m-%dT%H-%M-%S).md"
```

Then overwrite `docs/handoff.md` with forward-looking notes for the next session:
- What's in progress
- What to pick up next
- Any blockers or context the next session needs
- Do NOT include history — that's in the session summary

### Step 5: Confirm

Report what was stored:
- Number of observations captured
- Session summary id
- Handoff written
