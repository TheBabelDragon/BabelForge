use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Describes a single capability a node can offer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapabilityDescriptor {
    pub id: String,
    pub description: String,
    pub version: String,
    pub input_schema: String,
    pub output_schema: String,
    pub deterministic: bool,
    pub has_side_effects: bool,
    #[serde(default)]
    pub cost: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, serde_json::Value>,
}

impl CapabilityDescriptor {
    pub fn new(
        id: impl Into<String>,
        description: impl Into<String>,
        input_schema: impl Into<String>,
        output_schema: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            description: description.into(),
            version: "0.1.0".into(),
            input_schema: input_schema.into(),
            output_schema: output_schema.into(),
            deterministic: true,
            has_side_effects: false,
            cost: 1,
            tags: vec![],
            metadata: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityDiscovery {
    pub schema_version: String,
    pub capabilities: Vec<DiscoveredCapability>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredCapability {
    pub node_id: uuid::Uuid,
    pub node_name: String,
    pub capability: CapabilityDescriptor,
}
