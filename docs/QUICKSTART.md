# Quickstart — BabelForge v0.1

## Requirements

- Rust 1.82+ (see `rust-toolchain.toml`)

## Build & experiment

```bash
git clone https://github.com/TheBabelDragon/BabelForge.git
cd BabelForge
git checkout feat/babelforge-runtime-v0.1

cargo build --release -p forge-cli
cargo run -p forge-cli --release -- experiment
cargo test --workspace
```

The experiment demonstrates:

1. Two nodes register and publish capabilities
2. State inspection
3. Authorized execution
4. Unauthorized rejection (default-deny)
5. Resource limit enforcement
6. Event recording
7. Persistence across process restart
8. Event chain integrity verification
9. Replay notes
10. Idempotent duplicate detection
