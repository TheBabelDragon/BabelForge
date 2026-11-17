# Architecture — BabelForge v0.1

## Purpose

BabelForge is a local-first, language-neutral execution runtime. It lets independent nodes publish capabilities, authorize callers, execute typed operations, record events with integrity, and recover state after restart.

It is **not** an agent framework, a blockchain, or a cloud service mesh.

## Execution lifecycle

```
DISCOVER → INSPECT → AUTHORIZE → EXECUTE → VERIFY → RECORD
```

1. **Discover** — Registry lists nodes and capabilities.
2. **Inspect** — Caller may read state snapshots.
3. **Authorize** — PolicyEngine evaluates the request (default-deny).
4. **Execute** — Handler runs only after authorization and limit checks.
5. **Verify** — Result includes state hash; event chain is hash-linked.
6. **Record** — Append-only journal + optional signature.

## Crate map

| Crate | Responsibility |
|-------|----------------|
| forge-contracts | Versioned types + canonical hashing |
| forge-crypto | Ed25519 identity |
| forge-state | SQLite KV + event journal + idempotency |
| forge-policy | Default-deny + resource budgets |
| forge-executor | Lifecycle + handlers |
| forge-registry | In-memory discovery |
| forge-node | Assembled runtime node |
| forge-cli | `forge` binary + Phase 7 experiment |
| forge-wasm | Pure contract surface for WASM |
| forge-replay | Replay notes (no automatic side-effect replay) |
| forge-transport | Local abstraction |
| forge-events | Thin re-exports |
| forge-core | Version + re-exports |

## Persistence

- SQLite (bundled) for state and the append-only event log.
- Content hashes use canonical JSON (key-sorted, no whitespace) then SHA-256.
- Idempotency keys prevent double execution.
- Checkpoints mark recovery points.

## Security posture (v0.1)

- Default-deny policy enforced inside the executor path.
- Owner self-operations are allowed; all other callers need explicit grants.
- Side effects require the capability to declare them and policy to allow them (default false).
- No unrestricted plugin loading.
- WASM surface exposes only pure contract functions.

## Determinism

- Canonical hashing is deterministic across runs.
- Pure capabilities can be replayed from the journal.
- Physical / network side effects are never automatically replayed.
