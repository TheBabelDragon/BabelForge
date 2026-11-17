# Implemented vs Deferred — BabelForge v0.1

## Implemented

- 13-crate Rust workspace
- Versioned contracts + canonical hashing
- Ed25519 node identity
- SQLite state + append-only event journal
- Default-deny policy enforced on the execution path
- Typed executor with counter + sensor capabilities
- Idempotency keys
- Event chain verification
- Process restart persistence
- Phase 7 two-node experiment CLI
- WASM-compatible pure surface (forge-wasm)
- CI workflow (fmt, clippy, test, experiment, wasm32 check)
- Architecture, security, contracts, integration, quickstart docs

## Explicitly deferred

- Multi-node network transport beyond local/in-process
- Production adapters to MetaField / CANtheon / Dragon Gate / etc.
- Automatic side-effect replay
- Blockchain / wallet / token systems
- AI scheduling or agent orchestration
- Hardware-backed key storage
- Full JSON Schema publication under schemas/
- Distributed consensus
- Unrestricted plugin loading
