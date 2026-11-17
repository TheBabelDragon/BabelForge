use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Append-only event journal record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRecord {
    pub schema_version: String,
    pub event_id: Uuid,
    pub node_id: Uuid,
    pub sequence: u64,
    pub kind: EventKind,
    pub payload: serde_json::Value,
    pub content_hash: String,
    pub prev_hash: String,
    pub recorded_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature_hex: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    NodeRegistered,
    CapabilityPublished,
    OperationRequested,
    AuthorizationDecided,
    OperationExecuted,
    OperationRejected,
    StateCheckpoint,
    PolicyUpdated,
    DenialRecorded,
}

impl EventRecord {
    pub fn genesis(node_id: Uuid) -> Self {
        use crate::canonical::canonical_hash;
        let payload = serde_json::json!({ "message": "genesis" });
        let content_hash = canonical_hash(&payload).unwrap_or_default();
        Self {
            schema_version: crate::CONTRACT_VERSION.to_string(),
            event_id: Uuid::new_v4(),
            node_id,
            sequence: 0,
            kind: EventKind::NodeRegistered,
            payload,
            content_hash,
            prev_hash: "0".repeat(64),
            recorded_at: Utc::now(),
            signature_hex: None,
        }
    }
}
