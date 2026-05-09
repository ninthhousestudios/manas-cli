# manas-cli — single endpoint design

Status: design-session prep
Date: 2026-05-08
Context: brainstorming session on karma + usage report analysis. Builds on manas-hub-design.md.

---

## the problem

Today agents connect to 5+ MCP servers (sutra, smriti, chitta, yojana, sangha, plus manas serve for compound tools). Each needs its own hook for enforcement. manas serve should become THE endpoint — one connection, one hook to check, all tools routed through it.

This is the realization of what manas-hub-design.md started. That doc introduced `manas serve` for compound tools (wake_up, ingest). This extends it to be the primary MCP surface.

## what manas serve becomes

### compound tools (from hub design, already planned)

- `manas_wake_up` — session-start context injection
- `manas_ingest` — background extraction into chitta

### vidhi tools (new — two-layer skills as MCP tools)

vidhi is cross-system: it touches yojana (task state), sutra (code intelligence), git (commits), cargo (verification). Per principle 9, cross-tier composition belongs in manas-cli.

The two-layer skill model maps cleanly:
- **Rust shell** (manas serve tool): fetch task from yojana, set up context via sutra, run verification, commit on success, update task status
- **LLM body**: actual implementation/review reasoning

Candidate tools:
- `vidhi_implement` — TDD implementation of a yojana task
- `vidhi_review` — parallel lens-based code review
- `vidhi_verify` — run cargo check/test/clippy/fmt, report structured results

### enforcement via single endpoint

With manas serve as the gateway:
- Sangha registration enforced as precondition (no register → no tools)
- Tool Group ACL enforced at serve level (minimal vs rich boot)
- Cost model enforcement possible (budget per session)
- One hook checks one endpoint

### also CLI

`manas implement <task-id>`, `manas review <diff>`, `manas verify` as CLI commands that invoke the same compound logic. Useful for karma dispatch and human invocation.

## topology change

```
Before (current):
  agent ──► sutra (stdio)
         ──► smriti (stdio)
         ──► chitta (http)
         ──► yojana (http)
         ──► sangha (http)
         ──► manas serve (stdio, compound only)

After:
  agent ──► manas serve (stdio, primary)
              ├── proxies: sutra, smriti, chitta, yojana, sangha
              ├── compound: wake_up, ingest, wrap_up
              └── vidhi: implement, review, verify
```

Individual service endpoints remain available for direct access (debugging, admin), but the agent's default path is through manas serve.

## open questions

1. **Proxy latency.** Adding a hop for every sutra/smriti call. Is this measurable? sutra and smriti are stdio today — manas serve would need to manage their processes or switch them to HTTP.

2. **sutra/smriti lifecycle.** These are per-project today. manas serve is per-session. Does serve spawn sutra/smriti per workspace, or expect them to already be running?

3. **Incremental migration.** Can we add compound/vidhi tools to manas serve while keeping direct connections for sutra/smriti? That avoids the proxy question for v0.

4. **Harness adaptation.** Different harnesses configure MCP differently. manas warm already handles this — extend it to point all tool access through serve.

## references

- `docs/manas-hub-design.md` — the starting point for manas serve
- `docs/manas-architecture.md` § manas-cli, § skills two-layer model
- `docs/principles.md` § principle 9 (cross-tier composition in manas-cli)
- `manas-cli/docs/boot-contract.md`
