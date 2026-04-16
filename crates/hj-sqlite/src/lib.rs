use std::{fs, path::PathBuf};

use anyhow::{Context, Result, anyhow};
use hj_core::Handoff;
use rusqlite::{Connection, params, params_from_iter};

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

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct HandupCheckpoint {
    pub project: String,
    pub cwd: String,
    pub generated: String,
    pub recommendation: String,
    pub json_path: String,
}

pub struct HandupDb {
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
        let mut connection = self.open()?;
        Self::init_schema(&connection)?;
        let transaction = connection.transaction()?;

        let mut synced = 0usize;
        for item in &handoff.items {
            transaction.execute(
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
        Self::prune_missing_items(&transaction, project, handoff)?;
        transaction.commit()?;

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

    fn prune_missing_items(
        connection: &Connection,
        project: &str,
        handoff: &Handoff,
    ) -> Result<()> {
        if handoff.items.is_empty() {
            connection.execute("DELETE FROM items WHERE project = ?1", params![project])?;
            return Ok(());
        }

        let placeholders = std::iter::repeat_n("?", handoff.items.len())
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!("DELETE FROM items WHERE project = ? AND id NOT IN ({placeholders})");
        let params = std::iter::once(project.to_string())
            .chain(handoff.items.iter().map(|item| item.id.clone()))
            .collect::<Vec<_>>();
        connection.execute(&sql, params_from_iter(params))?;
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

impl HandupDb {
    pub fn new() -> Result<Self> {
        let home = dirs::home_dir().ok_or_else(|| anyhow!("could not determine home directory"))?;
        Ok(Self {
            db_path: home.join(".ctx/handoffs/handup.db"),
        })
    }

    #[cfg(test)]
    pub fn with_path(db_path: PathBuf) -> Self {
        Self { db_path }
    }

    pub fn checkpoint(&self, checkpoint: &HandupCheckpoint) -> Result<PathBuf> {
        let connection = self.open()?;
        Self::init_schema(&connection)?;
        connection.execute(
            "INSERT INTO checkpoints (project, cwd, generated, recommendation, json_path)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                checkpoint.project,
                checkpoint.cwd,
                checkpoint.generated,
                checkpoint.recommendation,
                checkpoint.json_path
            ],
        )?;
        Ok(self.db_path.clone())
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
            "CREATE TABLE IF NOT EXISTS checkpoints (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                project TEXT NOT NULL,
                cwd TEXT NOT NULL,
                generated TEXT NOT NULL,
                recommendation TEXT,
                json_path TEXT NOT NULL,
                created_at TEXT DEFAULT (datetime('now'))
            );",
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use hj_core::{Handoff, HandoffItem};
    use rusqlite::Connection;
    use tempfile::tempdir;

    use super::{HandoffDb, HandoffRow, HandupCheckpoint, HandupDb};

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

    #[test]
    fn upsert_prunes_rows_removed_from_handoff() {
        let tmp = tempdir().expect("tempdir");
        let db = HandoffDb::with_path(tmp.path().join("handoff.db"));
        let initial = Handoff {
            items: vec![
                HandoffItem {
                    id: "hj-1".into(),
                    priority: Some("P1".into()),
                    status: Some("open".into()),
                    ..HandoffItem::default()
                },
                HandoffItem {
                    id: "hj-2".into(),
                    priority: Some("P2".into()),
                    status: Some("open".into()),
                    ..HandoffItem::default()
                },
            ],
            ..Handoff::default()
        };
        let updated = Handoff {
            items: vec![HandoffItem {
                id: "hj-2".into(),
                priority: Some("P2".into()),
                status: Some("blocked".into()),
                ..HandoffItem::default()
            }],
            ..Handoff::default()
        };

        db.upsert("hj", &initial, "2026-04-16")
            .expect("initial upsert");
        db.upsert("hj", &updated, "2026-04-17")
            .expect("updated upsert");

        let rows = db.query("hj").expect("query");
        assert_eq!(
            rows,
            vec![HandoffRow {
                id: "hj-2".into(),
                priority: "P2".into(),
                status: "blocked".into(),
                completed: String::new(),
                updated: "2026-04-17".into(),
            }]
        );
    }

    #[test]
    fn upsert_empty_handoff_prunes_only_target_project() {
        let tmp = tempdir().expect("tempdir");
        let db = HandoffDb::with_path(tmp.path().join("handoff.db"));
        let initial = Handoff {
            items: vec![HandoffItem {
                id: "hj-1".into(),
                priority: Some("P1".into()),
                status: Some("open".into()),
                ..HandoffItem::default()
            }],
            ..Handoff::default()
        };
        let other_project = Handoff {
            items: vec![HandoffItem {
                id: "other-1".into(),
                priority: Some("P2".into()),
                status: Some("open".into()),
                ..HandoffItem::default()
            }],
            ..Handoff::default()
        };

        db.upsert("hj", &initial, "2026-04-16").expect("hj upsert");
        db.upsert("other", &other_project, "2026-04-16")
            .expect("other upsert");
        db.upsert("hj", &Handoff::default(), "2026-04-17")
            .expect("empty upsert");

        assert!(db.query("hj").expect("hj query").is_empty());
        assert_eq!(
            db.query("other").expect("other query"),
            vec![HandoffRow {
                id: "other-1".into(),
                priority: "P2".into(),
                status: "open".into(),
                completed: String::new(),
                updated: "2026-04-16".into(),
            }]
        );
    }

    #[test]
    fn handup_checkpoint_persists_rows() {
        let tmp = tempdir().expect("tempdir");
        let db = HandupDb::with_path(tmp.path().join("handup.db"));
        let checkpoint = HandupCheckpoint {
            project: "hj".into(),
            cwd: "/Users/joe/dev/hj".into(),
            generated: "2026-04-16".into(),
            recommendation: "Clean state".into(),
            json_path: "/Users/joe/.ctx/handoffs/hj/HANDUP.json".into(),
        };

        let db_path = db.checkpoint(&checkpoint).expect("checkpoint");
        assert!(db_path.ends_with("handup.db"));

        let connection = Connection::open(db_path).expect("open db");
        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM checkpoints", [], |row| row.get(0))
            .expect("count");
        assert_eq!(count, 1);
    }
}
