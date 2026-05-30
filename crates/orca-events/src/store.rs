use crate::event::{Event, EventKind};
use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Mutex;

/// SQLite-backed event store
pub struct EventStore {
    conn: Mutex<Connection>,
}

impl EventStore {
    /// Open or create an event store at the given path
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path).context("failed to open event store")?;
        let store = Self {
            conn: Mutex::new(conn),
        };
        store.init_schema()?;
        Ok(store)
    }

    /// Create an in-memory event store (for testing)
    pub fn in_memory() -> Result<Self> {
        let conn =
            Connection::open_in_memory().context("failed to create in-memory event store")?;
        let store = Self {
            conn: Mutex::new(conn),
        };
        store.init_schema()?;
        Ok(store)
    }

    fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS events (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                kind TEXT NOT NULL,
                data TEXT NOT NULL,
                timestamp TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_events_session ON events(session_id);
            CREATE INDEX IF NOT EXISTS idx_events_kind ON events(kind);
            CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp);",
        )
        .context("failed to create events table")?;
        Ok(())
    }

    /// Record a new event
    pub fn record(&self, event: &Event) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO events (id, session_id, kind, data, timestamp) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                event.id,
                event.session_id,
                event.kind.to_string(),
                serde_json::to_string(&event.data).unwrap_or_default(),
                event.timestamp.to_rfc3339(),
            ],
        )
        .context("failed to insert event")?;
        Ok(())
    }

    /// Get all events for a session
    pub fn get_session_events(&self, session_id: &str) -> Result<Vec<Event>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, session_id, kind, data, timestamp \
                 FROM events WHERE session_id = ?1 ORDER BY timestamp",
            )
            .context("failed to prepare query")?;

        let events = stmt
            .query_map(params![session_id], |row| {
                let kind_str: String = row.get(2)?;
                let data_str: String = row.get(3)?;
                let ts_str: String = row.get(4)?;
                Ok(Event {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    kind: parse_event_kind(&kind_str),
                    data: serde_json::from_str(&data_str).unwrap_or(serde_json::Value::Null),
                    timestamp: chrono::DateTime::parse_from_rfc3339(&ts_str)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                })
            })
            .context("failed to query events")?
            .collect::<Result<Vec<_>, _>>()
            .context("failed to collect events")?;

        Ok(events)
    }

    /// Get events by kind, optionally filtered to a session
    pub fn get_events_by_kind(
        &self,
        kind: &EventKind,
        session_id: Option<&str>,
    ) -> Result<Vec<Event>> {
        let conn = self.conn.lock().unwrap();
        let kind_str = kind.to_string();

        let (sql, param_values): (&str, Vec<Box<dyn rusqlite::types::ToSql>>) = match session_id {
            Some(sid) => (
                "SELECT id, session_id, kind, data, timestamp \
                 FROM events WHERE kind = ?1 AND session_id = ?2 ORDER BY timestamp",
                vec![
                    Box::new(kind_str),
                    Box::new(sid.to_string()),
                ],
            ),
            None => (
                "SELECT id, session_id, kind, data, timestamp \
                 FROM events WHERE kind = ?1 ORDER BY timestamp",
                vec![Box::new(kind_str)],
            ),
        };

        let mut stmt = conn.prepare(sql).context("failed to prepare query")?;
        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();

        let events = stmt
            .query_map(param_refs.as_slice(), |row| {
                let kind_str: String = row.get(2)?;
                let data_str: String = row.get(3)?;
                let ts_str: String = row.get(4)?;
                Ok(Event {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    kind: parse_event_kind(&kind_str),
                    data: serde_json::from_str(&data_str).unwrap_or(serde_json::Value::Null),
                    timestamp: chrono::DateTime::parse_from_rfc3339(&ts_str)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                })
            })
            .context("failed to query events by kind")?
            .collect::<Result<Vec<_>, _>>()
            .context("failed to collect events")?;

        Ok(events)
    }

    /// Count events in a session
    pub fn count_session_events(&self, session_id: &str) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let count: usize = conn
            .query_row(
                "SELECT COUNT(*) FROM events WHERE session_id = ?1",
                params![session_id],
                |row| row.get(0),
            )
            .context("failed to count events")?;
        Ok(count)
    }
}

fn parse_event_kind(s: &str) -> EventKind {
    match s {
        "session_start" => EventKind::SessionStart,
        "session_end" => EventKind::SessionEnd,
        "message_sent" => EventKind::MessageSent,
        "message_received" => EventKind::MessageReceived,
        "tool_call_start" => EventKind::ToolCallStart,
        "tool_call_end" => EventKind::ToolCallEnd,
        "tool_call_error" => EventKind::ToolCallError,
        "checkpoint_created" => EventKind::CheckpointCreated,
        "checkpoint_restored" => EventKind::CheckpointRestored,
        "capacity_warning" => EventKind::CapacityWarning,
        "loop_detected" => EventKind::LoopDetected,
        "sandbox_violation" => EventKind::SandboxViolation,
        "budget_warning" => EventKind::BudgetWarning,
        "model_switch" => EventKind::ModelSwitch,
        "error" => EventKind::Error,
        _ => EventKind::Error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_and_retrieve() {
        let store = EventStore::in_memory().unwrap();
        let event = Event::new("session-1", EventKind::SessionStart, serde_json::json!({}));
        store.record(&event).unwrap();

        let events = store.get_session_events("session-1").unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, EventKind::SessionStart);
    }

    #[test]
    fn test_count_events() {
        let store = EventStore::in_memory().unwrap();
        for _ in 0..5 {
            let event = Event::new("s1", EventKind::MessageSent, serde_json::json!({}));
            store.record(&event).unwrap();
        }
        assert_eq!(store.count_session_events("s1").unwrap(), 5);
    }

    #[test]
    fn test_filter_by_kind() {
        let store = EventStore::in_memory().unwrap();
        store
            .record(&Event::new(
                "s1",
                EventKind::ToolCallStart,
                serde_json::json!({}),
            ))
            .unwrap();
        store
            .record(&Event::new(
                "s1",
                EventKind::ToolCallEnd,
                serde_json::json!({}),
            ))
            .unwrap();
        store
            .record(&Event::new(
                "s1",
                EventKind::ToolCallStart,
                serde_json::json!({}),
            ))
            .unwrap();

        let starts = store
            .get_events_by_kind(&EventKind::ToolCallStart, Some("s1"))
            .unwrap();
        assert_eq!(starts.len(), 2);
    }
}
