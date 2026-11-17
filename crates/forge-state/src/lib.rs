//! Persistent state and event journal (SQLite).

use forge_contracts::{
    canonical::canonical_hash, Checkpoint, EventKind, EventRecord, ForgeError, StateSnapshot,
};
use rusqlite::{params, Connection};
use serde_json::Value;
use std::path::Path;
use uuid::Uuid;

pub struct StateStore {
    conn: Connection,
    node_id: Uuid,
    next_sequence: u64,
}

impl StateStore {
    pub fn open(path: impl AsRef<Path>, node_id: Uuid) -> Result<Self, ForgeError> {
        let conn = Connection::open(path)
            .map_err(|e| ForgeError::new("db_open", e.to_string()))?;
        let mut store = Self {
            conn,
            node_id,
            next_sequence: 0,
        };
        store.init_schema()?;
        store.next_sequence = store.load_next_sequence()?;
        Ok(store)
    }

    pub fn open_in_memory(node_id: Uuid) -> Result<Self, ForgeError> {
        let conn = Connection::open_in_memory()
            .map_err(|e| ForgeError::new("db_open", e.to_string()))?;
        let mut store = Self {
            conn,
            node_id,
            next_sequence: 0,
        };
        store.init_schema()?;
        Ok(store)
    }

    fn init_schema(&self) -> Result<(), ForgeError> {
        self.conn
            .execute_batch(
                r#"
                CREATE TABLE IF NOT EXISTS kv (
                    key TEXT PRIMARY KEY,
                    value TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS events (
                    sequence INTEGER PRIMARY KEY,
                    event_id TEXT NOT NULL UNIQUE,
                    kind TEXT NOT NULL,
                    payload TEXT NOT NULL,
                    content_hash TEXT NOT NULL,
                    prev_hash TEXT NOT NULL,
                    recorded_at TEXT NOT NULL,
                    signature_hex TEXT
                );
                CREATE TABLE IF NOT EXISTS idempotency (
                    key TEXT PRIMARY KEY,
                    request_id TEXT NOT NULL,
                    result_json TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS checkpoints (
                    sequence INTEGER PRIMARY KEY,
                    state_hash TEXT NOT NULL,
                    event_id TEXT NOT NULL,
                    created_at TEXT NOT NULL
                );
                "#,
            )
            .map_err(|e| ForgeError::new("db_schema", e.to_string()))?;
        Ok(())
    }

    fn load_next_sequence(&self) -> Result<u64, ForgeError> {
        let max: Option<i64> = self
            .conn
            .query_row("SELECT MAX(sequence) FROM events", [], |r| r.get(0))
            .unwrap_or(None);
        Ok(max.map(|m| (m as u64) + 1).unwrap_or(0))
    }

    pub fn get_state(&self, key: &str) -> Result<Option<Value>, ForgeError> {
        let mut stmt = self
            .conn
            .prepare("SELECT value FROM kv WHERE key = ?1")
            .map_err(|e| ForgeError::new("db", e.to_string()))?;
        let mut rows = stmt
            .query(params![key])
            .map_err(|e| ForgeError::new("db", e.to_string()))?;
        if let Some(row) = rows.next().map_err(|e| ForgeError::new("db", e.to_string()))? {
            let s: String = row.get(0).map_err(|e| ForgeError::new("db", e.to_string()))?;
            let v: Value = serde_json::from_str(&s)
                .map_err(|e| ForgeError::new("db_json", e.to_string()))?;
            Ok(Some(v))
        } else {
            Ok(None)
        }
    }

    pub fn set_state(&self, key: &str, value: &Value) -> Result<(), ForgeError> {
        let s = serde_json::to_string(value)
            .map_err(|e| ForgeError::new("json", e.to_string()))?;
        self.conn
            .execute(
                "INSERT OR REPLACE INTO kv (key, value) VALUES (?1, ?2)",
                params![key, s],
            )
            .map_err(|e| ForgeError::new("db", e.to_string()))?;
        Ok(())
    }

    pub fn snapshot(&self, capability_id: Option<&str>) -> Result<StateSnapshot, ForgeError> {
        let key = capability_id.unwrap_or("__root__");
        let state = self.get_state(key)?.unwrap_or(Value::Null);
        let content_hash = canonical_hash(&state).map_err(|e| ForgeError::new("hash", e))?;
        Ok(StateSnapshot {
            schema_version: forge_contracts::CONTRACT_VERSION.to_string(),
            node_id: self.node_id,
            capability_id: capability_id.map(|s| s.to_string()),
            state,
            content_hash,
            sequence: self.next_sequence.saturating_sub(1),
            captured_at: chrono::Utc::now(),
        })
    }

    pub fn append_event(
        &mut self,
        kind: EventKind,
        payload: Value,
        signature_hex: Option<String>,
    ) -> Result<EventRecord, ForgeError> {
        let prev_hash = if self.next_sequence == 0 {
            "0".repeat(64)
        } else {
            self.conn
                .query_row(
                    "SELECT content_hash FROM events WHERE sequence = ?1",
                    params![self.next_sequence as i64 - 1],
                    |r| r.get::<_, String>(0),
                )
                .map_err(|e| ForgeError::new("db", e.to_string()))?
        };

        let content_hash = canonical_hash(&payload).map_err(|e| ForgeError::new("hash", e))?;
        let event_id = Uuid::new_v4();
        let recorded_at = chrono::Utc::now();
        let sequence = self.next_sequence;

        let rec = EventRecord {
            schema_version: forge_contracts::CONTRACT_VERSION.to_string(),
            event_id,
            node_id: self.node_id,
            sequence,
            kind: kind.clone(),
            payload: payload.clone(),
            content_hash: content_hash.clone(),
            prev_hash: prev_hash.clone(),
            recorded_at,
            signature_hex: signature_hex.clone(),
        };

        let kind_str = serde_json::to_string(&kind).unwrap_or_default();
        let payload_str = serde_json::to_string(&payload)
            .map_err(|e| ForgeError::new("json", e.to_string()))?;

        self.conn
            .execute(
                "INSERT INTO events (sequence, event_id, kind, payload, content_hash, prev_hash, recorded_at, signature_hex)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    sequence as i64,
                    event_id.to_string(),
                    kind_str,
                    payload_str,
                    content_hash,
                    prev_hash,
                    recorded_at.to_rfc3339(),
                    signature_hex,
                ],
            )
            .map_err(|e| ForgeError::new("db", e.to_string()))?;

        self.next_sequence += 1;
        Ok(rec)
    }

    pub fn list_events(&self, limit: usize) -> Result<Vec<EventRecord>, ForgeError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT sequence, event_id, kind, payload, content_hash, prev_hash, recorded_at, signature_hex
                 FROM events ORDER BY sequence ASC LIMIT ?1",
            )
            .map_err(|e| ForgeError::new("db", e.to_string()))?;
        let rows = stmt
            .query_map(params![limit as i64], |row| {
                let sequence: i64 = row.get(0)?;
                let event_id: String = row.get(1)?;
                let kind_str: String = row.get(2)?;
                let payload_str: String = row.get(3)?;
                let content_hash: String = row.get(4)?;
                let prev_hash: String = row.get(5)?;
                let recorded_at: String = row.get(6)?;
                let signature_hex: Option<String> = row.get(7)?;
                Ok((
                    sequence,
                    event_id,
                    kind_str,
                    payload_str,
                    content_hash,
                    prev_hash,
                    recorded_at,
                    signature_hex,
                ))
            })
            .map_err(|e| ForgeError::new("db", e.to_string()))?;

        let mut out = Vec::new();
        for r in rows {
            let (seq, eid, kind_str, payload_str, ch, ph, ra, sig) =
                r.map_err(|e| ForgeError::new("db", e.to_string()))?;
            let kind: EventKind = serde_json::from_str(&kind_str)
                .unwrap_or(EventKind::OperationExecuted);
            let payload: Value = serde_json::from_str(&payload_str)
                .map_err(|e| ForgeError::new("json", e.to_string()))?;
            let recorded_at = chrono::DateTime::parse_from_rfc3339(&ra)
                .map(|d| d.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now());
            out.push(EventRecord {
                schema_version: forge_contracts::CONTRACT_VERSION.to_string(),
                event_id: Uuid::parse_str(&eid).unwrap_or_default(),
                node_id: self.node_id,
                sequence: seq as u64,
                kind,
                payload,
                content_hash: ch,
                prev_hash: ph,
                recorded_at,
                signature_hex: sig,
            });
        }
        Ok(out)
    }

    pub fn check_idempotency(&self, key: &str) -> Result<Option<Value>, ForgeError> {
        let mut stmt = self
            .conn
            .prepare("SELECT result_json FROM idempotency WHERE key = ?1")
            .map_err(|e| ForgeError::new("db", e.to_string()))?;
        let mut rows = stmt
            .query(params![key])
            .map_err(|e| ForgeError::new("db", e.to_string()))?;
        if let Some(row) = rows.next().map_err(|e| ForgeError::new("db", e.to_string()))? {
            let s: String = row.get(0).map_err(|e| ForgeError::new("db", e.to_string()))?;
            let v: Value = serde_json::from_str(&s)
                .map_err(|e| ForgeError::new("json", e.to_string()))?;
            Ok(Some(v))
        } else {
            Ok(None)
        }
    }

    pub fn store_idempotency(
        &self,
        key: &str,
        request_id: Uuid,
        result: &Value,
    ) -> Result<(), ForgeError> {
        let s = serde_json::to_string(result)
            .map_err(|e| ForgeError::new("json", e.to_string()))?;
        self.conn
            .execute(
                "INSERT OR REPLACE INTO idempotency (key, request_id, result_json) VALUES (?1, ?2, ?3)",
                params![key, request_id.to_string(), s],
            )
            .map_err(|e| ForgeError::new("db", e.to_string()))?;
        Ok(())
    }

    pub fn create_checkpoint(&mut self, state_hash: &str) -> Result<Checkpoint, ForgeError> {
        let event = self.append_event(
            EventKind::StateCheckpoint,
            serde_json::json!({ "state_hash": state_hash }),
            None,
        )?;
        let cp = Checkpoint {
            schema_version: forge_contracts::CONTRACT_VERSION.to_string(),
            node_id: self.node_id,
            sequence: event.sequence,
            state_hash: state_hash.to_string(),
            event_id: event.event_id,
            created_at: event.recorded_at,
            metadata: Default::default(),
        };
        self.conn
            .execute(
                "INSERT OR REPLACE INTO checkpoints (sequence, state_hash, event_id, created_at) VALUES (?1, ?2, ?3, ?4)",
                params![
                    cp.sequence as i64,
                    &cp.state_hash,
                    cp.event_id.to_string(),
                    cp.created_at.to_rfc3339()
                ],
            )
            .map_err(|e| ForgeError::new("db", e.to_string()))?;
        Ok(cp)
    }

    pub fn verify_event_chain(&self) -> Result<bool, ForgeError> {
        let events = self.list_events(10_000)?;
        let mut prev = "0".repeat(64);
        for e in &events {
            if e.prev_hash != prev {
                return Ok(false);
            }
            let expected = canonical_hash(&e.payload).map_err(|e| ForgeError::new("hash", e))?;
            if e.content_hash != expected {
                return Ok(false);
            }
            prev = e.content_hash.clone();
        }
        Ok(true)
    }
}
