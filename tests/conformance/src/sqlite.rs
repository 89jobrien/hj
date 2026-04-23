// §2 — hj-sqlite → hj-core adapter contracts

use hj_core::{Handoff, HandoffItem};
use hj_sqlite::{HandoffDb, HandupCheckpoint, HandupDb};
use rusqlite::Connection;
use tempfile::tempdir;

// §2.1 — upsert/query round-trip and ordering

#[test]
fn s2_1_upsert_then_query_round_trip() {
    let tmp = tempdir().unwrap();
    let db = HandoffDb::with_path(tmp.path().join("handoff.db"));
    let handoff = handoff_with(vec![HandoffItem {
        id: "hj-1".into(),
        priority: Some("P1".into()),
        status: Some("open".into()),
        ..HandoffItem::default()
    }]);

    db.upsert("hj", &handoff, "2026-04-18").unwrap();
    let rows = db.query("hj").unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, "hj-1");
    assert_eq!(rows[0].status, "open");
    assert_eq!(rows[0].updated, "2026-04-18");
}

#[test]
fn s2_1_query_orders_by_priority_then_id() {
    let tmp = tempdir().unwrap();
    let db = HandoffDb::with_path(tmp.path().join("handoff.db"));
    let handoff = handoff_with(vec![
        HandoffItem {
            id: "hj-2".into(),
            priority: Some("P2".into()),
            status: Some("open".into()),
            ..HandoffItem::default()
        },
        HandoffItem {
            id: "hj-1".into(),
            priority: Some("P0".into()),
            status: Some("open".into()),
            ..HandoffItem::default()
        },
    ]);

    db.upsert("hj", &handoff, "2026-04-18").unwrap();
    let rows = db.query("hj").unwrap();

    assert_eq!(rows[0].id, "hj-1", "P0 item must come first");
    assert_eq!(rows[1].id, "hj-2");
}

#[test]
fn s2_1_upsert_updates_existing_row_no_duplicate() {
    let tmp = tempdir().unwrap();
    let db = HandoffDb::with_path(tmp.path().join("handoff.db"));
    let handoff = handoff_with(vec![item("hj-1", "P1", "open")]);

    db.upsert("hj", &handoff, "2026-04-17").unwrap();

    let updated = handoff_with(vec![item("hj-1", "P1", "blocked")]);
    db.upsert("hj", &updated, "2026-04-18").unwrap();

    let rows = db.query("hj").unwrap();
    assert_eq!(rows.len(), 1, "upsert must not create duplicate rows");
    assert_eq!(rows[0].status, "blocked");
    assert_eq!(rows[0].updated, "2026-04-18");
}

// §2.2 — pruning contract

#[test]
fn s2_2_upsert_prunes_removed_ids() {
    let tmp = tempdir().unwrap();
    let db = HandoffDb::with_path(tmp.path().join("handoff.db"));
    let initial = handoff_with(vec![item("hj-1", "P1", "open"), item("hj-2", "P2", "open")]);
    db.upsert("hj", &initial, "2026-04-17").unwrap();

    let reduced = handoff_with(vec![item("hj-2", "P2", "open")]);
    db.upsert("hj", &reduced, "2026-04-18").unwrap();

    let rows = db.query("hj").unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, "hj-2");
}

#[test]
fn s2_2_pruning_scoped_to_project() {
    let tmp = tempdir().unwrap();
    let db = HandoffDb::with_path(tmp.path().join("handoff.db"));
    db.upsert(
        "hj",
        &handoff_with(vec![item("hj-1", "P1", "open")]),
        "2026-04-17",
    )
    .unwrap();
    db.upsert(
        "other",
        &handoff_with(vec![item("other-1", "P2", "open")]),
        "2026-04-17",
    )
    .unwrap();

    db.upsert("hj", &Handoff::default(), "2026-04-18").unwrap();

    assert!(db.query("hj").unwrap().is_empty(), "hj rows must be pruned");
    assert_eq!(
        db.query("other").unwrap().len(),
        1,
        "other project rows must survive"
    );
}

#[test]
fn s2_2_empty_handoff_clears_all_project_rows() {
    let tmp = tempdir().unwrap();
    let db = HandoffDb::with_path(tmp.path().join("handoff.db"));
    db.upsert(
        "hj",
        &handoff_with(vec![item("hj-1", "P1", "open")]),
        "2026-04-17",
    )
    .unwrap();

    db.upsert("hj", &Handoff::default(), "2026-04-18").unwrap();

    assert!(db.query("hj").unwrap().is_empty());
}

// §2.3 — complete and set_status

#[test]
fn s2_3_complete_sets_done_and_stamps_date() {
    let tmp = tempdir().unwrap();
    let db = HandoffDb::with_path(tmp.path().join("handoff.db"));
    db.upsert(
        "hj",
        &handoff_with(vec![item("hj-1", "P1", "open")]),
        "2026-04-17",
    )
    .unwrap();

    let changed = db.complete("hj", "hj-1", "2026-04-18").unwrap();
    assert!(changed);

    let rows = db.query("hj").unwrap();
    assert_eq!(rows[0].status, "done");
    assert_eq!(rows[0].completed, "2026-04-18");
}

#[test]
fn s2_3_set_status_does_not_touch_completed() {
    let tmp = tempdir().unwrap();
    let db = HandoffDb::with_path(tmp.path().join("handoff.db"));
    db.upsert(
        "hj",
        &handoff_with(vec![item("hj-1", "P1", "open")]),
        "2026-04-17",
    )
    .unwrap();

    db.set_status("hj", "hj-1", "blocked", "2026-04-18")
        .unwrap();

    let rows = db.query("hj").unwrap();
    assert_eq!(rows[0].status, "blocked");
    assert_eq!(rows[0].completed, "", "set_status must not stamp completed");
}

#[test]
fn s2_3_complete_returns_false_for_missing_row() {
    let tmp = tempdir().unwrap();
    let db = HandoffDb::with_path(tmp.path().join("handoff.db"));
    db.init().unwrap();

    let changed = db.complete("hj", "nonexistent", "2026-04-18").unwrap();
    assert!(!changed);
}

// §2.4 — HandupDb::checkpoint

#[test]
fn s2_4_checkpoint_inserts_row() {
    let tmp = tempdir().unwrap();
    let db = HandupDb::with_path(tmp.path().join("handup.db"));
    let cp = HandupCheckpoint {
        project: "hj".into(),
        cwd: "/Users/joe/dev/hj".into(),
        generated: "2026-04-18".into(),
        recommendation: "clean state".into(),
        json_path: "/tmp/HANDUP.json".into(),
    };

    let db_path = db.checkpoint(&cp).unwrap();
    assert!(db_path.ends_with("handup.db"));

    let conn = Connection::open(&db_path).unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM checkpoints", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1);
}

#[test]
fn s2_4_checkpoint_appends_not_replaces() {
    let tmp = tempdir().unwrap();
    let db = HandupDb::with_path(tmp.path().join("handup.db"));
    let cp = HandupCheckpoint {
        project: "hj".into(),
        cwd: "/tmp".into(),
        generated: "2026-04-18".into(),
        recommendation: "ok".into(),
        json_path: "/tmp/a.json".into(),
    };

    db.checkpoint(&cp).unwrap();
    db.checkpoint(&cp).unwrap();

    let conn = Connection::open(tmp.path().join("handup.db")).unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM checkpoints", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 2, "each checkpoint call must insert a new row");
}

// ─── helpers ────────────────────────────────────────────────────────────────

fn item(id: &str, priority: &str, status: &str) -> HandoffItem {
    HandoffItem {
        id: id.into(),
        priority: Some(priority.into()),
        status: Some(status.into()),
        ..HandoffItem::default()
    }
}

fn handoff_with(items: Vec<HandoffItem>) -> Handoff {
    Handoff {
        items,
        ..Handoff::default()
    }
}
