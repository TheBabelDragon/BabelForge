# Security — BabelForge v0.1

## Threat model (summary)

| Threat | Mitigation in v0.1 |
|--------|--------------------|
| Unauthorized execution | Default-deny PolicyEngine; executor refuses without allow decision |
| Replay / double-spend of requests | Idempotency keys stored in SQLite |
| Tampered event history | Hash-chained event journal + verify_event_chain |
| Silent numerical / state drift | Canonical content hashes on state snapshots |
| Unrestricted WASM host access | forge-wasm exposes only pure contract functions; no FS/network |
| Privilege escalation via side effects | Capabilities declare has_side_effects; policy default denies them |

## Cryptography

- Node identity: Ed25519 (ed25519-dalek).
- Event integrity: SHA-256 over canonical JSON.
- Optional event signatures: node signs the content hash.

Blockchain, wallets, and token economics are **out of scope**.

## Explicit non-goals for v0.1

- Distributed consensus
- Multi-node authentication beyond local grants
- Sandbox enforcement beyond “no host APIs in WASM surface”
- Hardware-backed key storage

## Audit points

- Denied operations are recorded as `OperationRejected` / `DenialRecorded` events.
- Every execution produces an `OperationExecuted` or rejection event.
