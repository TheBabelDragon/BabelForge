//! BabelForge core runtime lifecycle.

pub use forge_contracts as contracts;
pub use forge_crypto as crypto;
pub use forge_executor as executor;
pub use forge_policy as policy;
pub use forge_registry as registry;
pub use forge_state as state;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
