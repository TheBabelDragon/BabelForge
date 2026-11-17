//! Transport abstractions (v0.1 is in-process / local only).

pub trait Transport: Send + Sync {
    fn name(&self) -> &str;
}

pub struct LocalTransport;

impl Transport for LocalTransport {
    fn name(&self) -> &str {
        "local"
    }
}
