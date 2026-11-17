# BabelForge v0.1

**Universal Execution Runtime** for the TheBabelDragon ecosystem.

Rust-first, language-neutral execution substrate.

```
DISCOVER → INSPECT → AUTHORIZE → EXECUTE → VERIFY → RECORD
```

Local-first. No mandatory AI, blockchain, or cloud.

## Status

v0.1 implements the core execution lifecycle, versioned contracts, SQLite event journal, Ed25519 identity, default-deny policy, deterministic replay support, and a complete two-node experiment.

## Quick start

```bash
cargo build --release
cargo run -p forge-cli -- experiment
cargo run -p forge-cli -- version
cargo test
```

## Workspace

| Crate | Role |
|-------|------|
| forge-contracts | Versioned language-neutral contracts + canonical hashing |
| forge-crypto | Ed25519 node identity & signatures |
| forge-state | SQLite state + append-only event journal |
| forge-policy | Default-deny authorization + limits |
| forge-executor | Typed operation execution |
| forge-registry | Capability / node registry |
| forge-events | Event helpers |
| forge-replay | Deterministic replay support |
| forge-transport | Transport abstraction (local in v0.1) |
| forge-node | Executable node |
| forge-wasm | WASM-compatible surface |
| forge-cli | `forge` operational CLI |
| forge-core | Re-exports / lifecycle |

## Documentation

- [docs/ECOSYSTEM_INVENTORY.md](docs/ECOSYSTEM_INVENTORY.md) — Phase 0 reconnaissance
- [ARCHITECTURE.md](ARCHITECTURE.md)
- [SECURITY.md](SECURITY.md)
- [docs/CONTRACTS.md](docs/CONTRACTS.md)
- [docs/INTEGRATION_GUIDE.md](docs/INTEGRATION_GUIDE.md)
- [docs/QUICKSTART.md](docs/QUICKSTART.md)

## License

MIT
