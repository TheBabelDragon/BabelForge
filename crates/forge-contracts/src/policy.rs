use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionLimits {
    pub schema_version: String,
    pub max_cost_per_request: u64,
    pub max_concurrent: u32,
    pub default_timeout_ms: u64,
    pub allow_side_effects: bool,
}

impl Default for ExecutionLimits {
    fn default() -> Self {
        Self {
            schema_version: crate::CONTRACT_VERSION.to_string(),
            max_cost_per_request: 100,
            max_concurrent: 8,
            default_timeout_ms: 5_000,
            allow_side_effects: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionGrant {
    pub principal_id: Uuid,
    pub capability_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_id: Option<Uuid>,
    pub allow: bool,
}
