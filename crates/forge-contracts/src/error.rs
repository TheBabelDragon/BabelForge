use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Error, Serialize, Deserialize)]
#[error("{code}: {message}")]
pub struct ForgeError {
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl ForgeError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }

    pub fn unauthorized(msg: impl Into<String>) -> Self {
        Self::new("unauthorized", msg)
    }

    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::new("not_found", msg)
    }

    pub fn invalid_input(msg: impl Into<String>) -> Self {
        Self::new("invalid_input", msg)
    }

    pub fn limit_exceeded(msg: impl Into<String>) -> Self {
        Self::new("limit_exceeded", msg)
    }

    pub fn execution_failed(msg: impl Into<String>) -> Self {
        Self::new("execution_failed", msg)
    }

    pub fn conflict(msg: impl Into<String>) -> Self {
        Self::new("conflict", msg)
    }
}
