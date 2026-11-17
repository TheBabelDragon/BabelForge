//! Typed operation execution.
//!
//! Lifecycle: DISCOVER → INSPECT → AUTHORIZE → EXECUTE → VERIFY → RECORD

use forge_contracts::{
    CapabilityDescriptor, EventKind, ForgeError, OperationRequest, OperationResult,
    OperationStatus,
};
use forge_policy::PolicyEngine;
use forge_state::{IdempotencyLookup, StateStore};
use serde_json::{json, Value};
use std::collections::HashMap;

pub type Handler =
    Box<dyn Fn(&Value, &mut StateStore) -> Result<Value, ForgeError> + Send + Sync>;

pub struct Executor {
    capabilities: HashMap<String, CapabilityDescriptor>,
    handlers: HashMap<String, Handler>,
}

impl Executor {
    pub fn new() -> Self {
        Self {
            capabilities: HashMap::new(),
            handlers: HashMap::new(),
        }
    }

    pub fn register(&mut self, desc: CapabilityDescriptor, handler: Handler) {
        self.handlers.insert(desc.id.clone(), handler);
        self.capabilities.insert(desc.id.clone(), desc);
    }

    pub fn list_capabilities(&self) -> Vec<CapabilityDescriptor> {
        self.capabilities.values().cloned().collect()
    }

    pub fn get_capability(&self, id: &str) -> Option<&CapabilityDescriptor> {
        self.capabilities.get(id)
    }

    pub fn execute(
        &self,
        req: OperationRequest,
        policy: &PolicyEngine,
        store: &mut StateStore,
    ) -> Result<OperationResult, ForgeError> {
        let idem_key = req
            .idempotency_key
            .clone()
            .unwrap_or_else(|| req.request_id.to_string());

        let fingerprint = StateStore::request_fingerprint(
            req.caller_id,
            req.target_node_id,
            &req.capability_id,
            &req.input,
            req.budget,
            req.timeout_ms,
        )?;

        match store.check_idempotency(&idem_key, &fingerprint)? {
            IdempotencyLookup::Hit(prev) => {
                let mut result: OperationResult = serde_json::from_value(prev)
                    .map_err(|e| ForgeError::new("json", e.to_string()))?;
                result.from_replay = true;
                result.status = OperationStatus::IdempotentReplay;
                return Ok(result);
            }
            IdempotencyLookup::Conflict => {
                return Err(ForgeError::conflict(
                    "idempotency key already used with a different request fingerprint",
                ));
            }
            IdempotencyLookup::Miss => {}
        }

        let decision = policy.authorize(&req);
        store.append_event(
            EventKind::AuthorizationDecided,
            serde_json::to_value(&decision).unwrap_or(Value::Null),
            None,
        )?;

        if !decision.allowed {
            let result = OperationResult {
                schema_version: forge_contracts::CONTRACT_VERSION.to_string(),
                request_id: req.request_id,
                executed_capability_id: req.capability_id.clone(),
                status: OperationStatus::Rejected,
                output: None,
                error: Some(ForgeError::unauthorized(decision.reason.clone())),
                resulting_state_hash: None,
                completed_at: chrono::Utc::now(),
                from_replay: false,
            };
            store.append_event(
                EventKind::OperationRejected,
                serde_json::to_value(&result).unwrap_or(Value::Null),
                None,
            )?;
            let v = serde_json::to_value(&result).unwrap();
            store.store_idempotency(&idem_key, &fingerprint, req.request_id, &v)?;
            return Ok(result);
        }

        let cap = self
            .get_capability(&req.capability_id)
            .ok_or_else(|| ForgeError::not_found(format!("capability {}", req.capability_id)))?;
        policy.check_limits(&req, cap.cost)?;

        let handler = self
            .handlers
            .get(&req.capability_id)
            .ok_or_else(|| ForgeError::not_found(format!("handler for {}", req.capability_id)))?;

        let exec_result = handler(&req.input, store);

        let result = match exec_result {
            Ok(output) => {
                let snap = store.snapshot(Some(&req.capability_id))?;
                OperationResult {
                    schema_version: forge_contracts::CONTRACT_VERSION.to_string(),
                    request_id: req.request_id,
                    executed_capability_id: req.capability_id.clone(),
                    status: OperationStatus::Success,
                    output: Some(output),
                    error: None,
                    resulting_state_hash: Some(snap.content_hash),
                    completed_at: chrono::Utc::now(),
                    from_replay: false,
                }
            }
            Err(e) => OperationResult {
                schema_version: forge_contracts::CONTRACT_VERSION.to_string(),
                request_id: req.request_id,
                executed_capability_id: req.capability_id.clone(),
                status: OperationStatus::Failed,
                output: None,
                error: Some(e),
                resulting_state_hash: None,
                completed_at: chrono::Utc::now(),
                from_replay: false,
            },
        };

        store.append_event(
            EventKind::OperationExecuted,
            serde_json::to_value(&result).unwrap_or(Value::Null),
            None,
        )?;

        let v = serde_json::to_value(&result).unwrap();
        store.store_idempotency(&idem_key, &fingerprint, req.request_id, &v)?;
        Ok(result)
    }
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}

pub fn counter_capability() -> CapabilityDescriptor {
    CapabilityDescriptor::new(
        "counter.v1",
        "Bounded integer counter with read and increment",
        "counter.input.v1",
        "counter.output.v1",
    )
}

pub fn register_counter(executor: &mut Executor) {
    let desc = counter_capability();
    executor.register(
        desc,
        Box::new(|input, store| {
            let op = input.get("op").and_then(|v| v.as_str()).unwrap_or("read");
            let mut value = store
                .get_state("counter.v1")?
                .and_then(|v| v.as_i64())
                .unwrap_or(0);

            match op {
                "read" => Ok(json!({ "value": value })),
                "increment" => {
                    let delta = input.get("delta").and_then(|v| v.as_i64()).unwrap_or(1);
                    if !(0..=10).contains(&delta) {
                        return Err(ForgeError::invalid_input(
                            "delta must be between 0 and 10 inclusive",
                        ));
                    }
                    value = value.saturating_add(delta);
                    if value > 1000 {
                        return Err(ForgeError::limit_exceeded("counter exceeded 1000"));
                    }
                    store.set_state("counter.v1", &json!(value))?;
                    Ok(json!({ "value": value }))
                }
                _ => Err(ForgeError::invalid_input(format!("unknown op: {}", op))),
            }
        }),
    );
}

pub fn sensor_capability() -> CapabilityDescriptor {
    let mut d = CapabilityDescriptor::new(
        "sensor.threshold.v1",
        "Sensor-like value with read and threshold check",
        "sensor.input.v1",
        "sensor.output.v1",
    );
    d.cost = 2;
    d
}

pub fn register_sensor(executor: &mut Executor) {
    let desc = sensor_capability();
    executor.register(
        desc,
        Box::new(|input, store| {
            let op = input.get("op").and_then(|v| v.as_str()).unwrap_or("read");
            let mut value = store
                .get_state("sensor.threshold.v1")?
                .and_then(|v| v.as_f64())
                .unwrap_or(42.0);

            match op {
                "read" => Ok(json!({ "value": value })),
                "set" => {
                    let v = input
                        .get("value")
                        .and_then(|v| v.as_f64())
                        .ok_or_else(|| ForgeError::invalid_input("missing value"))?;
                    if !(0.0..=100.0).contains(&v) {
                        return Err(ForgeError::invalid_input("value must be 0..100"));
                    }
                    value = v;
                    store.set_state("sensor.threshold.v1", &json!(value))?;
                    Ok(json!({ "value": value }))
                }
                "threshold" => {
                    let threshold = input
                        .get("threshold")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(50.0);
                    Ok(json!({
                        "value": value,
                        "threshold": threshold,
                        "above": value > threshold
                    }))
                }
                _ => Err(ForgeError::invalid_input(format!("unknown op: {}", op))),
            }
        }),
    );
}
