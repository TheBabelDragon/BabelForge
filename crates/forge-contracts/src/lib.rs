//! BabelForge Contracts v0.1
//!
//! Language-neutral, versioned contracts. Rust owns the types; JSON Schema
//! and examples are the interop surface for Python, JS, C/C++, etc.
//!
//! Canonical encoding for hashing is defined in `canonical`.

pub mod canonical;
pub mod capability;
pub mod error;
pub mod event;
pub mod identity;
pub mod operation;
pub mod policy;
pub mod state;

pub use capability::*;
pub use error::*;
pub use event::*;
pub use identity::*;
pub use operation::*;
pub use policy::*;
pub use state::*;

/// Contract schema version for this crate.
pub const CONTRACT_VERSION: &str = "0.1.0";

/// Schema identifier prefix.
pub const SCHEMA_PREFIX: &str = "https://thebabeldragon.github.io/BabelForge/schemas/v0.1";
