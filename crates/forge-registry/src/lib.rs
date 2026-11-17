//! In-memory capability and node registry for v0.1.

use forge_contracts::{CapabilityDescriptor, DiscoveredCapability, NodeId, NodeRegistration};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Default)]
pub struct Registry {
    nodes: HashMap<Uuid, NodeRegistration>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, reg: NodeRegistration) {
        self.nodes.insert(reg.node.id, reg);
    }

    pub fn get(&self, id: &Uuid) -> Option<&NodeRegistration> {
        self.nodes.get(id)
    }

    pub fn list_nodes(&self) -> Vec<&NodeRegistration> {
        self.nodes.values().collect()
    }

    pub fn discover(&self, capability_id: Option<&str>) -> Vec<DiscoveredCapability> {
        let mut out = Vec::new();
        for reg in self.nodes.values() {
            for cap in &reg.capabilities {
                if capability_id.map(|id| id == cap.id).unwrap_or(true) {
                    out.push(DiscoveredCapability {
                        node_id: reg.node.id,
                        node_name: reg.node.name.clone(),
                        capability: cap.clone(),
                    });
                }
            }
        }
        out
    }
}
