use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationRequest {
    pub schema_version: String,
    pub request_id: Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
    pub target_node_id: Uuid,
    pub capability_id: String,
    pub input: serde_json::Value,
    pub caller_id: Uuid,
    pub requested_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
}

impl OperationRequest {
    pub fn new(
        target_node_id: Uuid,
        capability_id: impl Into<String>,
        input: serde_json::Value,
        caller_id: Uuid,
    ) -> Self {
        Self {
            schema_version: crate::CONTRACT_VERSION.to_string(),
            request_id: Uuid::new_v4(),
            idempotency_key: None,
            target_node_id,
            capability_id: capability_id.into(),
            input,
            caller_id,
            requested_at: Utc::now(),
            budget: None,
            timeout_ms: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationDecision {
    pub schema_version: String,
    pub request_id: Uuid,
    pub allowed: bool,
    pub reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authorized_capability_id: Option<String>,
    pub decided_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationResult {
    pub schema_version: String,
    pub request_id: Uuid,
    pub executed_capability_id: String,
    pub status: OperationStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<crate::ForgeError>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resulting_state_hash: Option<String>,
    pub completed_at: DateTime<Utc>,
    #[serde(default)]
    pub from_replay: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationStatus {
    Success,
    Rejected,
    Failed,
    Timeout,
    IdempotentReplay,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecord {
    pub schema_version: String,
    pub request: OperationRequest,
    pub decision: AuthorizationDecision,
    pub result: OperationResult,
}
