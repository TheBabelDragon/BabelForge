//! Persistent state and event journal (SQLite) — stabilized v0.1.
//!
//! Guarantees:
//! - Event content_hash commits to the full event envelope (incl. prev_hash).
//! - Idempotency keys are bound to a canonical request fingerprint.
//! - Node identity (UUID + key material) is persisted and restored on reopen.
//! - Schema migration from v1 → v2.

use forge_contracts::{
    canonical::canonical_hash, Checkpoint, EventKind, EventRecord, ForgeError, StateSnapshot,
};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};
use std::path::Path;
use uuid::Uuid;

const SCHEMA_VERSION: i32 = 2;

pub struct StateStore {
    conn: Connection,
    node_id: Uuid,
    next_sequence: u64,
}

#[derive(Debug)]
pub enum IdempotencyLookup {
    Miss,
    Hit(Value),
    Conflict,
}

impl StateStore {
    pub fn open(path: impl AsRef<Path>, expected_node_id: Option<Uuid>) -> Result<Self, ForgeError> {
        let conn =
            Connection::open(path).map_err(|e| ForgeError::new("db_open", e.to_string()))?;
        Self::from_conn(conn, expected_node_id)
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
        store.persist_identity(node_id, None)?;
        Ok(store)
    }

    fn from_conn(conn: Connection, expected_node_id: Option<Uuid>) -> Result<Self, ForgeError> {
        let mut store = Self {
            conn,
            node_id: Uuid::nil(),
            next_sequence: 0,
        };
        store.init_schema()?;
        store.migrate()?;
        let (stored_id, _) = store.load_identity()?;
        match (stored_id, expected_node_id) {
            (Some(id), Some(exp)) if id != exp => {
                return Err(ForgeError::conflict(format!(
                    "database identity {} does not match expected {}",
                    id, exp
                )));
            }
            (Some(id), _) => store.node_id = id,
            (None, Some(exp)) => {
                store.persist_identity(exp, None)?;
                store.node_id = exp;
            }
            (None, None) => {
                return Err(ForgeError::new(
                    "identity_missing",
                    "database has no node identity; create a new node instead of opening empty store",
                ));
            }
        }
        store.next_sequence = store.load_next_sequence()?;
        Ok(store)
    }

    fn init_schema(&self) -> Result<(), ForgeError> {
        self.conn
            .execute_batch(
                r#"
                CREATE TABLE IF NOT EXISTS meta (
                    key TEXT PRIMARY KEY,
                    value TEXT NOT NULL
                );
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
                    signature_hex TEXT,
                    node_id TEXT NOT NULL,
                    schema_version TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS idempotency (
                    key TEXT PRIMARY KEY,
                    request_fingerprint TEXT NOT NULL DEFAULT '',
                    request_id TEXT NOT NULL,
                    result_json TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS checkpoints (
                    sequence INTEGER PRIMARY KEY,
                    state_hash TEXT NOT NULL,
                    event_id TEXT NOT NULL,
                    created_at TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS identity (
                    id INTEGER PRIMARY KEY CHECK (id = 1),
                    node_id TEXT NOT NULL,
                    public_key_hex TEXT,
                    private_key_hex TEXT
                );
                "#,
            )
            .map_err(|e| ForgeError::new("db_schema", e.to_string()))?;
        Ok(())
    }

    fn migrate(&self) -> Result<(), ForgeError> {
        let ver: i32 = self
            .conn
            .query_row(
                "SELECT value FROM meta WHERE key = 'schema_version'",
                [],
                |r| r.get::<_, String>(0),
            )
            .optional()
            .map_err(|e| ForgeError::new("db", e.to_string()))?
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);

        if ver < 2 {
            let _ = self.conn.execute(
                "ALTER TABLE idempotency ADD COLUMN request_fingerprint TEXT NOT NULL DEFAULT ''",
                [],
            );
            self.conn
                .execute(
                    "INSERT OR REPLACE INTO meta (key, value) VALUES ('schema_version', ?1)",
                    params![SCHEMA_VERSION.to_string()],
                )
                .map_err(|e| ForgeError::new("db", e.to_string()))?;
        }
        Ok(())
    }

    pub fn persist_identity(
        &self,
        node_id: Uuid,
        key_material: Option<(&str, &str)>,
    ) -> Result<(), ForgeError> {
        let (pk, sk) = match key_material {
            Some((p, s)) => (Some(p.to_string()), Some(s.to_string())),
            None => (None, None),
        };
        self.conn
            .execute(
                "INSERT OR REPLACE INTO identity (id, node_id, public_key_hex, private_key_hex) VALUES (1, ?1, ?2, ?3)",
                params![node_id.to_string(), pk, sk],
            )
            .map_err(|e| ForgeError::new("db", e.to_string()))?;
        Ok(())
    }

    pub fn load_identity(&self) -> Result<(Option<Uuid>, Option<(String, String)>), ForgeError> {
        let row = self
            .conn
            .query_row(
                "SELECT node_id, public_key_hex, private_key_hex FROM identity WHERE id = 1",
                [],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, Option<String>>(1)?,
                        r.get::<_, Option<String>>(2)?,
                    ))
                },
            )
            .optional()
            .map_err(|e| ForgeError::new("db", e.to_string()))?;

        match row {
            Some((id_str, pk, sk)) => {
                let id = Uuid::parse_str(&id_str)
                    .map_err(|e| ForgeError::new("identity_corrupt", e.to_string()))?;
                let keys = match (pk, sk) {
                    (Some(p), Some(s)) if !p.is_empty() && !s.is_empty() => Some((p, s)),
                    (Some(_), None) | (None, Some(_)) => {
                        return Err(ForgeError::new(
                            "identity_corrupt",
                            "incomplete key material in identity store",
                        ));
                    }
                    _ => None,
                };
                Ok((Some(id), keys))
            }
            None => Ok((None, None)),
        }
    }

    pub fn node_id(&self) -> Uuid {
        self.node_id
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
            let v: Value =
                serde_json::from_str(&s).map_err(|e| ForgeError::new("db_json", e.to_string()))?;
            Ok(Some(v))
        } else {
            Ok(None)
        }
    }

    pub fn set_state(&self, key: &str, value: &Value) -> Result<(), ForgeError> {
        let s = serde_json::to_string(value).map_err(|e| ForgeError::new("json", e.to_string()))?;
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

    pub fn event_envelope(
        schema_version: &str,
        event_id: Uuid,
        node_id: Uuid,
        sequence: u64,
        kind: &EventKind,
        payload: &Value,
        prev_hash: &str,
        recorded_at: &str,
    ) -> Value {
        json!({
            "schema_version": schema_version,
            "event_id": event_id.to_string(),
            "node_id": node_id.to_string(),
            "sequence": sequence,
            "kind": kind,
            "payload": payload,
            "prev_hash": prev_hash,
            "recorded_at": recorded_at,
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

        let event_id = Uuid::new_v4();
        let recorded_at = chrono::Utc::now();
        let recorded_at_str = recorded_at.to_rfc3339();
        let schema_version = forge_contracts::CONTRACT_VERSION.to_string();
        let sequence = self.next_sequence;

        let envelope = Self::event_envelope(
            &schema_version,
            event_id,
            self.node_id,
            sequence,
            &kind,
            &payload,
            &prev_hash,
            &recorded_at_str,
        );
        let content_hash = canonical_hash(&envelope).map_err(|e| ForgeError::new("hash", e))?;

        let rec = EventRecord {
            schema_version: schema_version.clone(),
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
        let payload_str =
            serde_json::to_string(&payload).map_err(|e| ForgeError::new("json", e.to_string()))?;

        self.conn
            .execute(
                "INSERT INTO events (sequence, event_id, kind, payload, content_hash, prev_hash, recorded_at, signature_hex, node_id, schema_version)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    sequence as i64,
                    event_id.to_string(),
                    kind_str,
                    payload_str,
                    content_hash,
                    prev_hash,
                    recorded_at_str,
                    signature_hex,
                    self.node_id.to_string(),
                    schema_version,
                ],
            )
            .map_err(|e| ForgeError::new("db", e.to_string()))?;

        self.next_sequence += 1;
        Ok(rec)
    }

    pub fn list_events(&self, limit: Option<usize>) -> Result<Vec<EventRecord>, ForgeError> {
        let lim = limit.map(|l| l as i64).unwrap_or(i64::MAX);
        let mut stmt = self
            .conn
            .prepare(
                "SELECT sequence, event_id, kind, payload, content_hash, prev_hash, recorded_at, signature_hex, node_id, schema_version
                 FROM events ORDER BY sequence ASC LIMIT ?1",
            )
            .map_err(|e| ForgeError::new("db", e.to_string()))?;
        let rows = stmt
            .query_map(params![lim], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, String>(9)?,
                ))
            })
            .map_err(|e| ForgeError::new("db", e.to_string()))?;

        let mut out = Vec::new();
        for r in rows {
            let (seq, eid, kind_str, payload_str, ch, ph, ra, sig, nid, sv) =
                r.map_err(|e| ForgeError::new("db", e.to_string()))?;
            let kind: EventKind =
                serde_json::from_str(&kind_str).unwrap_or(EventKind::OperationExecuted);
            let payload: Value = serde_json::from_str(&payload_str)
                .map_err(|e| ForgeError::new("json", e.to_string()))?;
            let recorded_at = chrono::DateTime::parse_from_rfc3339(&ra)
                .map(|d| d.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now());
            out.push(EventRecord {
                schema_version: sv,
                event_id: Uuid::parse_str(&eid).unwrap_or_default(),
                node_id: Uuid::parse_str(&nid).unwrap_or(self.node_id),
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

    pub fn verify_event_chain(&self) -> Result<bool, ForgeError> {
        let events = self.list_events(None)?;
        let mut prev = "0".repeat(64);
        let mut expected_seq = 0u64;
        for e in &events {
            if e.sequence != expected_seq {
                return Ok(false);
            }
            if e.prev_hash != prev {
                return Ok(false);
            }
            let recorded_at_str = e.recorded_at.to_rfc3339();
            let envelope = Self::event_envelope(
                &e.schema_version,
                e.event_id,
                e.node_id,
                e.sequence,
                &e.kind,
                &e.payload,
                &e.prev_hash,
                &recorded_at_str,
            );
            let expected = canonical_hash(&envelope).map_err(|e| ForgeError::new("hash", e))?;
            if e.content_hash != expected {
                return Ok(false);
            }
            if e.node_id != self.node_id {
                return Ok(false);
            }
            prev = e.content_hash.clone();
            expected_seq += 1;
        }
        Ok(true)
    }

    pub fn request_fingerprint(
        caller_id: Uuid,
        target_node_id: Uuid,
        capability_id: &str,
        input: &Value,
        budget: Option<u64>,
        timeout_ms: Option<u64>,
    ) -> Result<String, ForgeError> {
        let v = json!({
            "caller_id": caller_id.to_string(),
            "target_node_id": target_node_id.to_string(),
            "capability_id": capability_id,
            "input": input,
            "budget": budget,
            "timeout_ms": timeout_ms,
        });
        canonical_hash(&v).map_err(|e| ForgeError::new("hash", e))
    }

    pub fn check_idempotency(
        &self,
        key: &str,
        fingerprint: &str,
    ) -> Result<IdempotencyLookup, ForgeError> {
        let row = self
            .conn
            .query_row(
                "SELECT request_fingerprint, result_json FROM idempotency WHERE key = ?1",
                params![key],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(|e| ForgeError::new("db", e.to_string()))?;

        match row {
            None => Ok(IdempotencyLookup::Miss),
            Some((stored_fp, result_json)) => {
                if !stored_fp.is_empty() && stored_fp != fingerprint {
                    return Ok(IdempotencyLookup::Conflict);
                }
                let v: Value = serde_json::from_str(&result_json)
                    .map_err(|e| ForgeError::new("json", e.to_string()))?;
                Ok(IdempotencyLookup::Hit(v))
            }
        }
    }

    pub fn store_idempotency(
        &self,
        key: &str,
        fingerprint: &str,
        request_id: Uuid,
        result: &Value,
    ) -> Result<(), ForgeError> {
        let s = serde_json::to_string(result).map_err(|e| ForgeError::new("json", e.to_string()))?;
        self.conn
            .execute(
                "INSERT OR REPLACE INTO idempotency (key, request_fingerprint, request_id, result_json) VALUES (?1, ?2, ?3, ?4)",
                params![key, fingerprint, request_id.to_string(), s],
            )
            .map_err(|e| ForgeError::new("db", e.to_string()))?;
        Ok(())
    }

    pub fn create_checkpoint(&mut self, state_hash: &str) -> Result<Checkpoint, ForgeError> {
        let event = self.append_event(
            EventKind::StateCheckpoint,
            json!({ "state_hash": state_hash }),
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
}
