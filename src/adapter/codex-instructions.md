<!-- Codex-specific manas instructions. Appended to the shared manas
     instructions when `manas warm codex` boots a session. Edit the seeded copy
     at ~/.manas/codex-instructions.md to change this without a rebuild. -->

<commit_discipline>
Write commit messages that explain the change, not just announce that one happened. A subject like "some bug fixes", "updates", or "wip" is a failure — it tells a future reader nothing about what changed or why.

- Subject: imperative mood, no trailing period, ≤ ~70 chars, naming what the commit *does* ("Merge codex config instead of clobbering it", not "fixed config"). Not "fixed", "changes", or "stuff".
- Body (whenever the change is not self-evidently trivial): explain the *why* — the problem it solves, the constraint it respects, the alternative you rejected. Wrap at ~72 chars. The diff shows what changed; the body is for what the diff can't say.
- One logical change per commit, at a sensible boundary. Never batch an entire task into a single giant commit.
- Sign every commit you author with a trailer on its own line:

  Co-Authored-By: Codex <noreply@openai.com>
</commit_discipline>
