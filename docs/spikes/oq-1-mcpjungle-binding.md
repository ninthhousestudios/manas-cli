# OQ-1 spike: mcpjungle per-session Tool Group binding

Status: resolved
Date: 2026-05-04
Author: spike agent (sonnet-4-6)
Yojana: manas-harness/2

---

## The question

The boot contract (§8 OQ-1) asks: how does mcpjungle enforce per-session Tool Group scoping? `McpClient` has an `AllowList` of *server names*, not Tool Group names. Three candidate answers were on the table: (a) a hidden client→ToolGroup binding field already exists; (b) mcpjungle needs a new binding column/join; (c) Tool Groups are separate MCP endpoints and scoping is endpoint selection rather than client binding.

---

## What I found

### Model layer

**`internal/model/mcp_client.go`** — `McpClient` has three fields that matter: `Name`, `AccessToken`, and `AllowList` (a JSON array of server names). The comment on `AllowList` says explicitly: *"In the future, this will be removed in favor of a separate table for ACLs."* There is no `ToolGroupID`, no foreign key to `ToolGroup`, no join table. The only enforcement method is `CheckHasServerAccess(serverName string) bool`, which is an allow-list check on server names only.

**`internal/model/tool_group.go`** — `ToolGroup` has `IncludedTools`, `IncludedServers`, and `ExcludedTools` (all JSON). No reference to `McpClient`. No join table. The two models are entirely decoupled in the DB schema.

### Service layer

**`internal/service/toolgroup/toolgroup.go`** — At startup `initToolGroupMCPServers()` (line 355) reads every `ToolGroup` from the DB and builds a dedicated in-memory `*server.MCPServer` for each one, pre-populated with exactly the tools that group resolves to. These are stored in `s.mcpServers[groupName]`. When a tool is added or removed from mcpjungle, callbacks (`handleToolAddition`, `handleToolDeletion`) keep each group's server in sync. The group's MCP server instance is the enforcement point — it literally only knows about the tools it was initialized with.

### Route/middleware layer

**`internal/api/server.go`** — Two distinct MCP endpoint families are registered:

| Route | Handler |
|---|---|
| `POST /mcp` | Global proxy — all registered tools |
| `GET/POST /v0/groups/:name/mcp` | `toolGroupMCPServerCallHandler()` — group-specific proxy |
| `GET/POST /v0/groups/:name/sse` | SSE variant of the same |

Both families run `checkAuthForMcpProxyAccess()` middleware first (line 185, 205). That middleware validates the bearer token against `McpClient.AccessToken` and injects the client into the request context — but **it does not filter tools based on that client**. After auth, the group route calls `toolGroupService.GetToolGroupMCPServer(groupName)` and hands off to *that group's dedicated MCP server instance* (line 293–306 of `tool_groups.go`). The scoping is structural: the group's MCP server object only contains the group's tools, so tool enumeration and invocation are inherently limited.

**`internal/api/middleware.go`** — `checkAuthForMcpProxyAccess()` does token validation and client injection only. It does not consult `AllowList` in the context of tool group endpoints. The `AllowList`/`CheckHasServerAccess` path is a separate mechanism used by the global `/mcp` endpoint (confirmed by the field's comment and absence of any call to `CheckHasServerAccess` in the tool group handler path).

### The `AllowList` field's actual role

`McpClient.AllowList` and `CheckHasServerAccess` are used by the **global** `/mcp` proxy endpoint (not the group endpoints). In enterprise mode, a client connecting to `/mcp` is filtered to only the servers on its allow list. This is the *existing* per-client ACL mechanism — it operates at the server-name level and is orthogonal to Tool Groups.

---

## The answer: (c)

**Tool Groups are separate MCP endpoints. Scoping is endpoint selection, not client binding.**

Each Tool Group gets its own endpoint (`/v0/groups/<name>/mcp`). The group's MCP server instance is pre-populated at creation time with exactly the tools in that group and nothing else. A client connecting to `/v0/groups/code/mcp` sees only `code` tools because the server object it's talking to only contains those tools — there is no runtime filter, no ACL check against the client, no join table lookup. The scoping is structural and enforced at the MCP protocol layer.

**There is no per-client Tool Group binding field and none is needed.** The boot contract assumption that "the Tool Group is determined by the access token" is incorrect as stated. The Tool Group is determined by the **URL the client connects to**, not the token.

---

## Implications for manas-cli

### What changes in the boot contract

§3 contains this line:
> "The Tool Group is determined by the access token, not the URL."

This is backwards. The correct statement is:
> "The Tool Group is determined by the URL (`/v0/groups/<name>/mcp`), not the access token."

`MANAS_MCP_ENDPOINT` must be the group-specific URL, e.g. `http://127.0.0.1:8080/v0/groups/code/mcp` for minimal mode or `http://127.0.0.1:8080/v0/groups/full/mcp` for rich mode. The endpoint is **mode-specific**, not mode-neutral.

### What does the token do?

In mcpjungle enterprise mode, the bearer token identifies a `McpClient` and is required for auth. It does not restrict which group endpoint the client can hit — any valid token can connect to any group endpoint. That means:

- **Token-based group restriction does not exist today.** A session token minted for `minimal` mode can connect to `/v0/groups/full/mcp` with no additional enforcement from mcpjungle.
- **manas-cli is the only enforcement point for mode selection** (it writes the correct group URL into `MANAS_MCP_ENDPOINT`). If the harness ignores the env var and constructs its own URL to `/v0/groups/full/mcp`, it will succeed — but that requires the harness to actively subvert the binding, which is outside the threat model (the LLM cannot change its own MCP endpoint config mid-session).

### Is a mcpjungle PR needed?

For v0: **no**. Endpoint selection is sufficient for the stated threat model (an unprivileged session must not accidentally reach chitta/smriti/sangha). The harness receives one URL and cannot enumerate or switch group endpoints on its own.

If the threat model later includes "a compromised harness that can make arbitrary HTTP requests to mcpjungle," then a per-client group restriction (new column `tool_group_name` on `McpClient`, enforced in `checkAuthForMcpProxyAccess`) would be needed. That is a ~50-line Go change + a DB migration (new nullable string column, no FK needed since group names are unique strings). Not a breaking change. No estimated urgency for v0.

### Token minting

Step 3 of the boot lifecycle ("mint a session token") needs revision. The admin API (`POST /api/v0/clients`) creates a `McpClient` with a name and allow list. manas-cli should:

1. `POST /api/v0/clients` with `name: "manas-session-<ID>"` and `allow_list: ["*"]` (wildcard, since group scoping is by URL). mcpjungle returns the access token.
2. Set `MANAS_MCP_ENDPOINT` to the group-specific URL (`/v0/groups/<mode-group>/mcp`).
3. On teardown, `DELETE /api/v0/clients/manas-session-<ID>`.

The `AllowList` field on the minted client is functionally irrelevant for group endpoint access — but setting it to `["*"]` (the wildcard constant at `pkg/types`) is correct and forward-compatible.

---

## Residual uncertainty

- `checkAuthForMcpProxyAccess()` injects the `McpClient` into the request context, but the group handler (`toolGroupMCPServerCallHandler`) does not use it. I did not find any code that reads `client` from the context in the tool group path. This means **the token is validated (required in enterprise mode) but not further used** in group-endpoint calls. This is consistent with the endpoint-scoping model but worth noting — it means the token is auth-only, not authz.
- The global `/mcp` endpoint does use `CheckHasServerAccess` somewhere downstream (implied by the field's existence and the server-injection into context). I did not trace that path fully; it's not relevant to manas-cli since manas-cli will use group endpoints only.
- mcpjungle enterprise mode vs. dev mode: in dev mode, `checkAuthForMcpProxyAccess()` skips token validation entirely. manas-cli should assert it is talking to an enterprise-mode instance (or at minimum warn loudly if in dev mode).

---

## Open follow-ups

1. **Boot contract §3 correction** — update "The Tool Group is determined by the access token, not the URL" to "by the URL". Done in §8 below; propagate to the binding field table (change `MANAS_MCP_ENDPOINT` note from "same endpoint regardless of mode" to "mode-specific group endpoint").
2. **Acceptance test update** — Test 1 step 2 should assert the fixture harness connects to the group-specific URL. The test as written ("connects to `MANAS_MCP_ENDPOINT`") is correct if the env var holds the group URL, but the test setup should verify that connecting to the *other* group URL with the same token also succeeds (to document that token-based group restriction is not enforced by mcpjungle — only the URL matters).
3. **manas-cli admin auth** — manas-cli needs admin credentials (user token in enterprise mode) to call `POST /api/v0/clients`. How that credential is stored/provided to manas-cli is unspecified. This is a separate question from OQ-1 but surfaces from it.
4. **Dev mode warning** — add a check in manas-cli boot that mcpjungle is in enterprise mode; warn if dev mode (ACL is off).
