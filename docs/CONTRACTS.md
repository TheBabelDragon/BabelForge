# Contracts — BabelForge v0.1

All contracts carry `schema_version: "0.1.0"`.

## Core types

- `NodeId` / `NodeRegistration`
- `CapabilityDescriptor` / `DiscoveredCapability`
- `OperationRequest` / `AuthorizationDecision` / `OperationResult`
- `EventRecord` / `EventKind`
- `StateSnapshot` / `Checkpoint`
- `ExecutionLimits` / `PermissionGrant`
- `ForgeError`

## Canonical hashing

`forge_contracts::canonical::canonical_hash` produces a stable SHA-256 hex digest.
Object keys are sorted; whitespace is removed; control characters are escaped.
Never hash raw `serde_json::to_string` output for integrity.

## Language neutrality

Rust owns the execution semantics. Other languages interact via JSON that conforms to these types. JSON Schema files will be added under `schemas/` as the surface stabilizes.
