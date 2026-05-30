use crate::checkpoint::{CheckpointData, CheckpointSummary, RestoredSession};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Manages checkpoint persistence to disk
pub struct CheckpointManager {
    session_dir: PathBuf,
    max_checkpoints: usize,
}

impl CheckpointManager {
    /// Create a new checkpoint manager for a session
    pub fn new(base_dir: &Path, session_id: &str, max_checkpoints: usize) -> Result<Self> {
        let session_dir = base_dir.join(session_id);
        std::fs::create_dir_all(&session_dir).context("failed to create checkpoint directory")?;
        Ok(Self {
            session_dir,
            max_checkpoints,
        })
    }

    /// Save a checkpoint to disk
    pub fn save(&self, checkpoint: &CheckpointData) -> Result<()> {
        let filename = format!("{}.json", checkpoint.id);
        let path = self.session_dir.join(&filename);
        let json =
            serde_json::to_string_pretty(checkpoint).context("failed to serialize checkpoint")?;
        std::fs::write(&path, json).context("failed to write checkpoint file")?;

        tracing::info!(
            checkpoint_id = %checkpoint.id,
            path = %path.display(),
            "checkpoint saved"
        );

        // Enforce max checkpoints
        self.enforce_limit()?;

        Ok(())
    }

    /// Load a checkpoint by ID
    pub fn load(&self, checkpoint_id: &str) -> Result<CheckpointData> {
        let filename = format!("{}.json", checkpoint_id);
        let path = self.session_dir.join(&filename);
        let json = std::fs::read_to_string(&path).context("failed to read checkpoint file")?;
        let checkpoint: CheckpointData =
            serde_json::from_str(&json).context("failed to deserialize checkpoint")?;

        tracing::info!(checkpoint_id = %checkpoint_id, "checkpoint loaded");
        Ok(checkpoint)
    }

    /// Restore a session from a checkpoint
    pub fn restore(&self, checkpoint_id: &str) -> Result<RestoredSession> {
        let checkpoint = self.load(checkpoint_id)?;
        Ok(RestoredSession {
            seed_messages: checkpoint.seed_messages,
            completed_goals: checkpoint.completed_goals,
            anchor_context: checkpoint.anchor_context,
            state_snapshot: checkpoint.state_snapshot,
            iteration_count: checkpoint.iteration_count,
            total_cost_usd: checkpoint.total_cost_usd,
        })
    }

    /// List all checkpoints for this session
    pub fn list(&self) -> Result<Vec<CheckpointSummary>> {
        let mut summaries = Vec::new();

        if !self.session_dir.exists() {
            return Ok(summaries);
        }

        let entries =
            std::fs::read_dir(&self.session_dir).context("failed to read checkpoint directory")?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "json")
                && let Ok(json) = std::fs::read_to_string(&path)
                && let Ok(checkpoint) = serde_json::from_str::<CheckpointData>(&json)
            {
                summaries.push(CheckpointSummary::from(&checkpoint));
            }
        }

        // Sort by creation time, newest first
        summaries.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(summaries)
    }

    /// Delete a checkpoint
    pub fn delete(&self, checkpoint_id: &str) -> Result<()> {
        let filename = format!("{}.json", checkpoint_id);
        let path = self.session_dir.join(&filename);
        if path.exists() {
            std::fs::remove_file(&path).context("failed to delete checkpoint file")?;
            tracing::info!(checkpoint_id = %checkpoint_id, "checkpoint deleted");
        }
        Ok(())
    }

    /// Enforce the maximum checkpoint limit by removing oldest
    fn enforce_limit(&self) -> Result<()> {
        let summaries = self.list()?;
        if summaries.len() > self.max_checkpoints {
            // Remove oldest (end of sorted list)
            let to_remove = summaries.len() - self.max_checkpoints;
            for summary in summaries.iter().rev().take(to_remove) {
                self.delete(&summary.id)?;
                tracing::info!(
                    checkpoint_id = %summary.id,
                    "removed old checkpoint to enforce limit"
                );
            }
        }
        Ok(())
    }

    /// Get the session directory path
    pub fn session_dir(&self) -> &Path {
        &self.session_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checkpoint::{CheckpointData, CompletedGoal};
    use chrono::Utc;

    fn make_checkpoint(id: &str, desc: &str) -> CheckpointData {
        CheckpointData {
            id: id.to_string(),
            session_id: "test-session".to_string(),
            seed_messages: vec![orca_utils::message::Message::system("test")],
            completed_goals: vec![CompletedGoal {
                description: "do something".to_string(),
                completed_at: Utc::now(),
                result_summary: "done".to_string(),
            }],
            active_goals: vec!["another goal".to_string()],
            anchor_context: "user wants X".to_string(),
            state_snapshot: serde_json::json!({}),
            iteration_count: 42,
            total_cost_usd: 0.5,
            created_at: Utc::now(),
            description: desc.to_string(),
        }
    }

    #[test]
    fn test_save_and_load() {
        let tmp = tempfile::tempdir().unwrap();
        let mgr = CheckpointManager::new(tmp.path(), "sess-1", 10).unwrap();

        let cp = make_checkpoint("cp-1", "first checkpoint");
        mgr.save(&cp).unwrap();

        let loaded = mgr.load("cp-1").unwrap();
        assert_eq!(loaded.description, "first checkpoint");
        assert_eq!(loaded.iteration_count, 42);
    }

    #[test]
    fn test_list_checkpoints() {
        let tmp = tempfile::tempdir().unwrap();
        let mgr = CheckpointManager::new(tmp.path(), "sess-1", 10).unwrap();

        mgr.save(&make_checkpoint("cp-1", "first")).unwrap();
        mgr.save(&make_checkpoint("cp-2", "second")).unwrap();

        let list = mgr.list().unwrap();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn test_restore_session() {
        let tmp = tempfile::tempdir().unwrap();
        let mgr = CheckpointManager::new(tmp.path(), "sess-1", 10).unwrap();

        mgr.save(&make_checkpoint("cp-1", "test")).unwrap();
        let restored = mgr.restore("cp-1").unwrap();
        assert_eq!(restored.completed_goals.len(), 1);
        assert_eq!(restored.iteration_count, 42);
    }

    #[test]
    fn test_max_checkpoints_enforced() {
        let tmp = tempfile::tempdir().unwrap();
        let mgr = CheckpointManager::new(tmp.path(), "sess-1", 2).unwrap();

        mgr.save(&make_checkpoint("cp-1", "first")).unwrap();
        mgr.save(&make_checkpoint("cp-2", "second")).unwrap();
        mgr.save(&make_checkpoint("cp-3", "third")).unwrap();

        let list = mgr.list().unwrap();
        assert_eq!(list.len(), 2);
        // Oldest should have been removed
        assert!(list.iter().all(|s| s.id != "cp-1"));
    }

    #[test]
    fn test_delete_checkpoint() {
        let tmp = tempfile::tempdir().unwrap();
        let mgr = CheckpointManager::new(tmp.path(), "sess-1", 10).unwrap();

        mgr.save(&make_checkpoint("cp-1", "test")).unwrap();
        mgr.delete("cp-1").unwrap();
        assert!(mgr.list().unwrap().is_empty());
    }
}
