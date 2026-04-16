use std::{fs, path::PathBuf};

use anyhow::{Context, Result, anyhow};
use hj_core::Handoff;
use rusqlite::{Connection, params};

#[derive(Debug, Clone)]
pub struct UpsertReport {
    pub db_path: PathBuf,
    pub synced: usize,
}

pub struct HandoffDb {
    db_path: PathBuf,
}

impl HandoffDb {
    pub fn new() -> Result<Self> {
        let home = dirs::home_dir().ok_or_else(|| anyhow!("could not determine home directory"))?;
        Ok(Self {
            db_path: home.join(".local/share/atelier/handoff.db"),
        })
    }

    pub fn upsert(&self, project: &str, handoff: &Handoff, today: &str) -> Result<UpsertReport> {
        let parent = self
            .db_path
            .parent()
            .ok_or_else(|| anyhow!("database path has no parent directory"))?;
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;

        let connection = Connection::open(&self.db_path)
            .with_context(|| format!("failed to open {}", self.db_path.display()))?;
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS items (
                project   TEXT NOT NULL,
                id        TEXT NOT NULL,
                name      TEXT,
                priority  TEXT,
                status    TEXT,
                completed TEXT,
                updated   TEXT,
                PRIMARY KEY (project, id)
            );",
        )?;

        let mut synced = 0usize;
        for item in &handoff.items {
            connection.execute(
                "INSERT INTO items (project, id, name, priority, status, completed, updated)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(project, id) DO UPDATE SET
                    status = excluded.status,
                    completed = excluded.completed,
                    updated = excluded.updated",
                params![
                    project,
                    item.id,
                    item.name.as_deref().unwrap_or_default(),
                    item.priority.as_deref().unwrap_or_default(),
                    item.status.as_deref().unwrap_or_default(),
                    item.completed.as_deref().unwrap_or_default(),
                    today,
                ],
            )?;
            synced += 1;
        }

        Ok(UpsertReport {
            db_path: self.db_path.clone(),
            synced,
        })
    }
}
