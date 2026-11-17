use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeId {
    pub id: Uuid,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_key_hex: Option<String>,
}

impl NodeId {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            public_key_hex: None,
        }
    }

    pub fn with_key(mut self, public_key_hex: impl Into<String>) -> Self {
        self.public_key_hex = Some(public_key_hex.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeRegistration {
    pub schema_version: String,
    pub node: NodeId,
    pub registered_at: DateTime<Utc>,
    pub capabilities: Vec<crate::CapabilityDescriptor>,
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub metadata: std::collections::BTreeMap<String, serde_json::Value>,
}

impl NodeRegistration {
    pub fn new(node: NodeId, capabilities: Vec<crate::CapabilityDescriptor>) -> Self {
        Self {
            schema_version: crate::CONTRACT_VERSION.to_string(),
            node,
            registered_at: Utc::now(),
            capabilities,
            metadata: Default::default(),
        }
    }
}
