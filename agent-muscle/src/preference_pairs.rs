//! DPO-style preference pairs from success vs failed traces (Phase 5.1).

use anyhow::Result;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PreferencePair {
    pub workflow_name: String,
    pub prompt: String,
    pub chosen: String,
    pub rejected: String,
}

pub fn export_preference_pairs(conn: &Connection, limit: usize) -> Result<Vec<PreferencePair>> {
    let mut stmt = conn.prepare(
        "SELECT workflow_name, prompt,
                MAX(CASE WHEN success = 1 THEN completion END) AS chosen,
                MAX(CASE WHEN success = 0 THEN completion END) AS rejected
         FROM traces
         GROUP BY workflow_name, prompt
         HAVING chosen IS NOT NULL AND rejected IS NOT NULL
         LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![limit as i64], |row| {
        Ok(PreferencePair {
            workflow_name: row.get(0)?,
            prompt: row.get(1)?,
            chosen: row.get(2)?,
            rejected: row.get(3)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trace_collector::{ExecutionTraceEvent, TraceStore};

    #[test]
    fn pairs_success_vs_failure() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("traces.db");
        let store = TraceStore::open(&path).unwrap();
        let base = ExecutionTraceEvent {
            msg_id: "a".into(),
            execution_id: "e1".into(),
            workflow_name: "wf".into(),
            prompt: "do x".into(),
            completion: "good".into(),
            reward: 1.0,
            success: true,
        };
        store.insert(&base).unwrap();
        store
            .insert(&ExecutionTraceEvent {
                msg_id: "b".into(),
                completion: "bad".into(),
                success: false,
                ..base.clone()
            })
            .unwrap();
        let pairs = store.export_preference_pairs(10).unwrap();
        assert_eq!(pairs.len(), 1);
    }
}
