//! Authorization policy — default-deny.

use forge_contracts::{
    AuthorizationDecision, ExecutionLimits, ForgeError, OperationRequest, PermissionGrant,
};
use std::collections::HashMap;
use uuid::Uuid;

pub struct PolicyEngine {
    owner_id: Uuid,
    /// (principal, capability_id) -> allow
    grants: HashMap<(Uuid, String), bool>,
    limits: ExecutionLimits,
}

impl PolicyEngine {
    pub fn new(owner_id: Uuid, limits: ExecutionLimits) -> Self {
        Self {
            owner_id,
            grants: HashMap::new(),
            limits,
        }
    }

    pub fn grant(&mut self, g: PermissionGrant) {
        self.grants
            .insert((g.principal_id, g.capability_id), g.allow);
    }

    pub fn authorize(&self, req: &OperationRequest) -> AuthorizationDecision {
        // Owner may always operate on own node.
        if req.caller_id == self.owner_id && req.target_node_id == self.owner_id {
            return AuthorizationDecision {
                schema_version: forge_contracts::CONTRACT_VERSION.to_string(),
                request_id: req.request_id,
                allowed: true,
                reason: "owner self-operation".into(),
                authorized_capability_id: Some(req.capability_id.clone()),
                decided_at: chrono::Utc::now(),
            };
        }

        let key = (req.caller_id, req.capability_id.clone());
        match self.grants.get(&key) {
            Some(true) => AuthorizationDecision {
                schema_version: forge_contracts::CONTRACT_VERSION.to_string(),
                request_id: req.request_id,
                allowed: true,
                reason: "explicit grant".into(),
                authorized_capability_id: Some(req.capability_id.clone()),
                decided_at: chrono::Utc::now(),
            },
            Some(false) => AuthorizationDecision {
                schema_version: forge_contracts::CONTRACT_VERSION.to_string(),
                request_id: req.request_id,
                allowed: false,
                reason: "explicit denial".into(),
                authorized_capability_id: None,
                decided_at: chrono::Utc::now(),
            },
            None => AuthorizationDecision {
                schema_version: forge_contracts::CONTRACT_VERSION.to_string(),
                request_id: req.request_id,
                allowed: false,
                reason: "default deny: no matching grant".into(),
                authorized_capability_id: None,
                decided_at: chrono::Utc::now(),
            },
        }
    }

    pub fn check_limits(
        &self,
        req: &OperationRequest,
        capability_cost: u64,
    ) -> Result<(), ForgeError> {
        if capability_cost > self.limits.max_cost_per_request {
            return Err(ForgeError::limit_exceeded(format!(
                "cost {} exceeds max {}",
                capability_cost, self.limits.max_cost_per_request
            )));
        }
        if let Some(budget) = req.budget {
            if capability_cost > budget {
                return Err(ForgeError::limit_exceeded(format!(
                    "cost {} exceeds request budget {}",
                    capability_cost, budget
                )));
            }
        }
        Ok(())
    }

    pub fn limits(&self) -> &ExecutionLimits {
        &self.limits
    }
}
