# manas-cli — admin credential

Status: decided
Date: 2026-05-05

## what this is

manas-cli calls mcpjungle's admin API to mint and revoke per-session tokens. That API requires authentication in enterprise mode. This doc specifies how manas-cli finds and uses the admin credential.

---

## resolution order

1. **Env var `MANAS_ADMIN_TOKEN`** — if set, used directly. For CI, scripts, ephemeral contexts.
2. **File `~/.manas/admin-token`** — single-line file, mode `0600`. For interactive use.

First match wins. If neither exists, manas-cli refuses to boot (except in dev mode — see below).

---

## bootstrapping

`manas init` (new subcommand, added after scaffold):

1. Check if mcpjungle is reachable at `MANAS_MCPJUNGLE_URL` (default `http://127.0.0.1:8080`).
2. If mcpjungle is in dev mode (no auth required): warn loudly, skip token storage, exit 0.
3. If mcpjungle requires auth: prompt the operator for the admin token (or accept `--token <value>`).
4. Write to `~/.manas/admin-token` with mode `0600`. Create `~/.manas/` if absent.
5. Validate: call `GET /api/v0/health` with the token. If 401/403, reject and ask again.

The operator gets the admin token from mcpjungle's own setup (its config file or initial bootstrap output). manas-cli never generates this token; it only stores and uses it.

---

## dev-mode detection

mcpjungle dev mode = no authentication required. Detected by:

1. `GET /api/v0/health` without auth headers.
2. If response is 200 with full status: **dev mode**.
3. If response is 401: enterprise mode.

When dev mode is detected, `manas health` prints:

```
  ⚠ WARNING: mcpjungle is in dev mode (no auth)
  ⚠ Tool Group ACL is NOT enforced — any client can reach any subsystem
  ⚠ This is acceptable for local development only
```

`manas warm` / `manas done` / `manas reflect` still work in dev mode (they skip token minting since it's not needed), but the warning is repeated at boot.

---

## what we're not doing

- **Keyring integration.** Over-engineering for a local-only single-user tool. The file is `0600` in a user-owned directory. If stolen, the attacker already has shell access.
- **Token rotation.** The admin token is long-lived and operator-managed. manas-cli's per-session tokens are ephemeral and auto-revoked; the admin token is not.
- **Multiple mcpjungle instances.** One admin token, one mcpjungle URL. If multi-instance becomes real, config grows a `[jungles]` section.

---

## file layout

```
~/.manas/
├── admin-token          # admin credential (0600)
├── bindings.log         # jsonl of session bindings
└── sessions/
    └── <session-id>/    # per-session scratch (config files, adapter state)
```
