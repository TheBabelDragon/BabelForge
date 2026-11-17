//! WASM-compatible surface.
//!
//! Exposes pure contract types and canonical hashing.
//! No filesystem, network, or unrestricted host capabilities.

pub use forge_contracts::{
    canonical::{canonical_hash, canonical_json},
    CapabilityDescriptor, ForgeError, NodeId, OperationRequest, OperationResult, OperationStatus,
    CONTRACT_VERSION,
};

pub fn version() -> String {
    CONTRACT_VERSION.to_string()
}

pub fn hash_json(json_str: &str) -> Result<String, String> {
    let v: serde_json::Value = serde_json::from_str(json_str).map_err(|e| e.to_string())?;
    canonical_hash(&v)
}
