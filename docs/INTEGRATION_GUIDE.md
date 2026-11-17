# Integration Guide — BabelForge v0.1

## Principles

1. Integrate through versioned contracts and explicit adapters only.
2. Do not introduce cross-repo compile-time dependencies.
3. Existing projects must not regress.

## Adapter pattern

1. Map the external system’s operations to one or more `CapabilityDescriptor`s.
2. Implement a handler that translates input → external call → typed output.
3. Register the capability on a BabelForge node.
4. Grant principals explicitly.
5. Record results via the normal executor path so the event journal stays complete.

## Example boundaries (contract-level only)

- **CANtheon** — capability that ingests a normalized frame and emits a validated observation.
- **signal-processor** — acoustic / session event capability (read-only or recipe application).
- **MetaField** — admit observation / FieldTick operations (hardware required; no invented CSI).
- **DRAGON//GATE** — node identity and evidence events can share the same Ed25519 + NDJSON style.

Physical ECU writes (ECUgate) remain outside the initial execution scope.

## What is deferred

End-to-end production integrations, multi-node mesh transport, and automatic side-effect replay.
