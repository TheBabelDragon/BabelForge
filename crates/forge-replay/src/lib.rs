//! Deterministic replay of eligible operations.
//!
//! Physical / network side effects are NEVER automatically replayed.

use forge_contracts::{EventKind, ForgeError};
use forge_state::StateStore;

pub fn verify_and_summarize(store: &StateStore) -> Result<(bool, u64, usize), ForgeError> {
    let ok = store.verify_event_chain()?;
    let events = store.list_events(None)?;
    let seq = events.last().map(|e| e.sequence).unwrap_or(0);
    Ok((ok, seq, events.len()))
}

pub fn describe_replayability(store: &StateStore) -> Result<Vec<String>, ForgeError> {
    let events = store.list_events(None)?;
    let mut notes = Vec::new();
    for e in events {
        match e.kind {
            EventKind::OperationExecuted => {
                notes.push(format!(
                    "seq {}: OperationExecuted — eligible for deterministic state replay if capability is pure",
                    e.sequence
                ));
            }
            EventKind::StateCheckpoint => {
                notes.push(format!("seq {}: Checkpoint — recovery point", e.sequence));
            }
            other => {
                notes.push(format!("seq {}: {:?} — recorded", e.sequence, other));
            }
        }
    }
    Ok(notes)
}
