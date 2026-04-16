use std::{fs, path::PathBuf};

use anyhow::{Context, Result, anyhow};
use hj_core::Handoff;
use rusqlite::{Connection, params};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct HandoffRow {
    pub id: String,
    pub priority: String,
    pub status: String,
    pub completed: String,
    pub updated: String,
}

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

    #[cfg(test)]
    pub fn with_path(db_path: PathBuf) -> Self {
        Self { db_path }
    }

    pub fn init(&self) -> Result<PathBuf> {
        let connection = self.open()?;
        Self::init_schema(&connection)?;
        Ok(self.db_path.clone())
    }

    pub fn upsert(&self, project: &str, handoff: &Handoff, today: &str) -> Result<UpsertReport> {
        let connection = self.open()?;
        Self::init_schema(&connection)?;

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

    pub fn query(&self, project: &str) -> Result<Vec<HandoffRow>> {
        let connection = self.open()?;
        Self::init_schema(&connection)?;

        let mut statement = connection.prepare(
            "SELECT id, coalesce(priority, ''), coalesce(status, ''), coalesce(completed, ''),
                    coalesce(updated, '')
             FROM items
             WHERE project = ?1
             ORDER BY priority, id",
        )?;
        let rows = statement.query_map(params![project], |row| {
            Ok(HandoffRow {
                id: row.get(0)?,
                priority: row.get(1)?,
                status: row.get(2)?,
                completed: row.get(3)?,
                updated: row.get(4)?,
            })
        })?;

        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }
        Ok(items)
    }

    pub fn complete(&self, project: &str, id: &str, today: &str) -> Result<bool> {
        self.update_status(project, id, "done", Some(today), today)
    }

    pub fn set_status(&self, project: &str, id: &str, status: &str, today: &str) -> Result<bool> {
        self.update_status(project, id, status, None, today)
    }

    fn open(&self) -> Result<Connection> {
        let parent = self
            .db_path
            .parent()
            .ok_or_else(|| anyhow!("database path has no parent directory"))?;
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;

        Connection::open(&self.db_path)
            .with_context(|| format!("failed to open {}", self.db_path.display()))
    }

    fn init_schema(connection: &Connection) -> Result<()> {
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
        Ok(())
    }

    fn update_status(
        &self,
        project: &str,
        id: &str,
        status: &str,
        completed: Option<&str>,
        today: &str,
    ) -> Result<bool> {
        let connection = self.open()?;
        Self::init_schema(&connection)?;
        let changed = connection.execute(
            "UPDATE items
             SET status = ?3,
                 completed = COALESCE(?4, completed),
                 updated = ?5
             WHERE project = ?1 AND id = ?2",
            params![project, id, status, completed, today],
        )?;
        Ok(changed > 0)
    }
}

#[cfg(test)]
mod tests {
    use hj_core::{Handoff, HandoffItem};
    use tempfile::tempdir;

    use super::{HandoffDb, HandoffRow};

    #[test]
    fn query_returns_rows_in_priority_order() {
        let tmp = tempdir().expect("tempdir");
        let db = HandoffDb::with_path(tmp.path().join("handoff.db"));
        let handoff = Handoff {
            items: vec![
                HandoffItem {
                    id: "hj-2".into(),
                    priority: Some("P2".into()),
                    status: Some("open".into()),
                    ..HandoffItem::default()
                },
                HandoffItem {
                    id: "hj-1".into(),
                    priority: Some("P1".into()),
                    status: Some("blocked".into()),
                    ..HandoffItem::default()
                },
            ],
            ..Handoff::default()
        };

        db.upsert("hj", &handoff, "2026-04-16").expect("upsert");

        let rows = db.query("hj").expect("query");
        assert_eq!(
            rows,
            vec![
                HandoffRow {
                    id: "hj-1".into(),
                    priority: "P1".into(),
                    status: "blocked".into(),
                    completed: String::new(),
                    updated: "2026-04-16".into(),
                },
                HandoffRow {
                    id: "hj-2".into(),
                    priority: "P2".into(),
                    status: "open".into(),
                    completed: String::new(),
                    updated: "2026-04-16".into(),
                },
            ]
        );
    }

    #[test]
    fn complete_and_status_update_existing_rows() {
        let tmp = tempdir().expect("tempdir");
        let db = HandoffDb::with_path(tmp.path().join("handoff.db"));
        let handoff = Handoff {
            items: vec![HandoffItem {
                id: "hj-1".into(),
                priority: Some("P1".into()),
                status: Some("open".into()),
                ..HandoffItem::default()
            }],
            ..Handoff::default()
        };

        db.upsert("hj", &handoff, "2026-04-16").expect("upsert");
        assert!(
            db.set_status("hj", "hj-1", "blocked", "2026-04-17")
                .expect("status")
        );
        assert!(db.complete("hj", "hj-1", "2026-04-18").expect("complete"));

        let rows = db.query("hj").expect("query");
        assert_eq!(rows[0].status, "done");
        assert_eq!(rows[0].completed, "2026-04-18");
        assert_eq!(rows[0].updated, "2026-04-18");
    }
}
