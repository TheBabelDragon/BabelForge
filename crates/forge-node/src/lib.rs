//! A BabelForge node: identity + capabilities + policy + state + executor.

use forge_contracts::{
    CapabilityDescriptor, EventKind, ExecutionLimits, NodeId, NodeRegistration, OperationRequest,
    OperationResult, PermissionGrant,
};
use forge_crypto::NodeKeyPair;
use forge_executor::{register_counter, register_sensor, Executor};
use forge_policy::PolicyEngine;
use forge_registry::Registry;
use forge_state::StateStore;
use std::path::Path;
use uuid::Uuid;

pub struct Node {
    pub id: NodeId,
    pub keypair: NodeKeyPair,
    pub store: StateStore,
    pub policy: PolicyEngine,
    pub executor: Executor,
    pub registry: Registry,
}

impl Node {
    /// Create a new node. If `db_path` exists and already has an identity, that identity is restored.
    pub fn create(name: &str, db_path: Option<&Path>) -> Result<Self, forge_contracts::ForgeError> {
        let keypair = NodeKeyPair::generate();
        let provisional = NodeId::new(name).with_key(keypair.public_key_hex());

        let store = match db_path {
            Some(p) if p.exists() => {
                // Reopen existing DB — identity must come from the store.
                StateStore::open(p, None)?
            }
            Some(p) => {
                // New DB path — seed with provisional identity.
                let store = StateStore::open(p, Some(provisional.id))?;
                store.persist_identity(
                    provisional.id,
                    Some((&keypair.public_key_hex(), "")), // private key not persisted as hex in v0.1 demo
                )?;
                store
            }
            None => StateStore::open_in_memory(provisional.id)?,
        };

        let node_uuid = store.node_id();
        let id = if node_uuid == provisional.id {
            provisional
        } else {
            // Restored identity — keep stored UUID, attach current public key for this process.
            NodeId {
                id: node_uuid,
                name: name.to_string(),
                public_key_hex: Some(keypair.public_key_hex()),
            }
        };

        let policy = PolicyEngine::new(id.id, ExecutionLimits::default());
        let executor = Executor::new();
        let registry = Registry::new();

        let mut node = Self {
            id: id.clone(),
            keypair,
            store,
            policy,
            executor,
            registry,
        };

        // Only emit NodeRegistered on first creation (sequence 0 after genesis would already exist).
        let events = node.store.list_events(Some(1))?;
        if events.is_empty() {
            node.store.append_event(
                EventKind::NodeRegistered,
                serde_json::json!({
                    "node_id": id.id,
                    "name": id.name,
                    "public_key": node.keypair.public_key_hex(),
                }),
                None,
            )?;
        }

        Ok(node)
    }

    pub fn register_capability(
        &mut self,
        desc: CapabilityDescriptor,
        handler: forge_executor::Handler,
    ) {
        self.executor.register(desc, handler);
    }

    pub fn publish(&mut self) {
        let caps = self.executor.list_capabilities();
        let reg = NodeRegistration::new(self.id.clone(), caps);
        self.registry.register(reg);
    }

    pub fn grant(&mut self, principal: Uuid, capability_id: &str) {
        self.policy.grant(PermissionGrant {
            principal_id: principal,
            capability_id: capability_id.to_string(),
            node_id: Some(self.id.id),
            allow: true,
        });
    }

    pub fn submit(
        &mut self,
        req: OperationRequest,
    ) -> Result<OperationResult, forge_contracts::ForgeError> {
        self.store.append_event(
            EventKind::OperationRequested,
            serde_json::to_value(&req).unwrap_or_default(),
            None,
        )?;
        self.executor.execute(req, &self.policy, &mut self.store)
    }

    pub fn inspect_state(
        &self,
        capability_id: Option<&str>,
    ) -> Result<forge_contracts::StateSnapshot, forge_contracts::ForgeError> {
        self.store.snapshot(capability_id)
    }

    pub fn events(
        &self,
        limit: Option<usize>,
    ) -> Result<Vec<forge_contracts::EventRecord>, forge_contracts::ForgeError> {
        self.store.list_events(limit)
    }

    pub fn verify_integrity(&self) -> Result<bool, forge_contracts::ForgeError> {
        self.store.verify_event_chain()
    }

    pub fn replay_summary(&self) -> Result<Vec<String>, forge_contracts::ForgeError> {
        forge_replay::describe_replayability(&self.store)
    }
}

pub fn build_experiment_nodes(
    counter_db: Option<&Path>,
    sensor_db: Option<&Path>,
) -> Result<(Node, Node), forge_contracts::ForgeError> {
    let mut node_a = Node::create("counter-node", counter_db)?;
    register_counter(&mut node_a.executor);
    node_a.publish();

    let mut node_b = Node::create("sensor-node", sensor_db)?;
    register_sensor(&mut node_b.executor);
    node_b.publish();

    node_a.grant(node_a.id.id, "counter.v1");
    node_b.grant(node_b.id.id, "sensor.threshold.v1");

    Ok((node_a, node_b))
}
