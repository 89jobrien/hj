// §1 — hj-core pure domain contracts

use hj_core::{
    Handoff, HandoffItem, HandoffState, ReconcileMode, TodoSnapshot, build_reconcile_plan,
    default_id_prefix, infer_priority, sanitize_name, titleize_slug,
};

// §1.1 — build_reconcile_plan: audit mode never creates

#[test]
fn s1_1_reconcile_audit_no_creates() {
    let handoff = handoff_with_items(vec![open_item("hj-1", "Missing task")]);
    let snapshot = TodoSnapshot {
        active_titles: vec![],
        closed_titles: vec![],
    };
    let plan = build_reconcile_plan("hj", &handoff, &snapshot, ReconcileMode::Audit);
    assert!(
        plan.creates.is_empty(),
        "audit mode must never populate creates"
    );
    assert_eq!(plan.report.not_captured, vec!["Missing task".to_string()]);
}

#[test]
fn s1_1_reconcile_sync_creates_missing() {
    let handoff = handoff_with_items(vec![open_item("hj-1", "Missing task")]);
    let snapshot = TodoSnapshot {
        active_titles: vec![],
        closed_titles: vec![],
    };
    let plan = build_reconcile_plan("hj", &handoff, &snapshot, ReconcileMode::Sync);
    assert_eq!(plan.creates.len(), 1);
    assert_eq!(plan.creates[0].title, "Missing task");
    assert_eq!(plan.report.created_count, 1);
    assert_eq!(plan.report.captured_count, 1);
}

#[test]
fn s1_1_reconcile_active_title_increments_captured() {
    let handoff = handoff_with_items(vec![open_item("hj-1", "Already tracked")]);
    let snapshot = TodoSnapshot {
        active_titles: vec!["Already tracked".into()],
        closed_titles: vec![],
    };
    let plan = build_reconcile_plan("hj", &handoff, &snapshot, ReconcileMode::Sync);
    assert!(plan.creates.is_empty());
    assert_eq!(plan.report.captured_count, 1);
    assert_eq!(plan.report.created_count, 0);
}

#[test]
fn s1_1_reconcile_closed_upstream_not_recaptured() {
    let handoff = handoff_with_items(vec![open_item("hj-1", "Done task")]);
    let snapshot = TodoSnapshot {
        active_titles: vec![],
        closed_titles: vec!["Done task".into()],
    };
    let plan = build_reconcile_plan("hj", &handoff, &snapshot, ReconcileMode::Sync);
    assert!(plan.creates.is_empty());
    assert!(plan.report.not_captured.is_empty());
    assert_eq!(plan.report.closed_upstream, vec!["Done task".to_string()]);
}

#[test]
fn s1_1_reconcile_orphaned_active_title_has_no_handoff_item() {
    let handoff = Handoff::default();
    let snapshot = TodoSnapshot {
        active_titles: vec!["Orphaned".into()],
        closed_titles: vec![],
    };
    let plan = build_reconcile_plan("hj", &handoff, &snapshot, ReconcileMode::Sync);
    assert_eq!(plan.report.orphaned, vec!["Orphaned".to_string()]);
}

#[test]
fn s1_1_reconcile_blocked_variant_matches_closed_upstream() {
    let mut item = open_item("hj-1", "Blocked task");
    item.status = Some("blocked".into());
    let handoff = handoff_with_items(vec![item]);
    let snapshot = TodoSnapshot {
        active_titles: vec![],
        closed_titles: vec!["Blocked task [BLOCKED]".into()],
    };
    let plan = build_reconcile_plan("hj", &handoff, &snapshot, ReconcileMode::Sync);
    assert!(plan.creates.is_empty());
    assert_eq!(plan.report.closed_upstream.len(), 1);
}

// §1.2 — todo_title and title_variants

#[test]
fn s1_2_todo_title_prefers_name_slug() {
    let item = HandoffItem {
        id: "x-1".into(),
        name: Some("wire-render-pass".into()),
        status: Some("open".into()),
        title: "ignored".into(),
        ..HandoffItem::default()
    };
    assert_eq!(item.todo_title(), "Wire Render Pass");
}

#[test]
fn s1_2_todo_title_falls_back_to_title() {
    let item = HandoffItem {
        id: "x-1".into(),
        name: None,
        status: Some("open".into()),
        title: "Raw title".into(),
        ..HandoffItem::default()
    };
    assert_eq!(item.todo_title(), "Raw title");
}

#[test]
fn s1_2_todo_title_appends_blocked_suffix() {
    let item = HandoffItem {
        id: "x-1".into(),
        name: None,
        status: Some("blocked".into()),
        title: "Some task".into(),
        ..HandoffItem::default()
    };
    assert_eq!(item.todo_title(), "Some task [BLOCKED]");
}

#[test]
fn s1_2_title_variants_no_duplicates() {
    let item = HandoffItem {
        id: "x-1".into(),
        name: Some("some-task".into()),
        status: Some("open".into()),
        title: "Some Task".into(),
        ..HandoffItem::default()
    };
    let variants = item.title_variants();
    let deduped: std::collections::HashSet<_> = variants.iter().collect();
    assert_eq!(
        variants.len(),
        deduped.len(),
        "title_variants must not contain duplicates"
    );
}

#[test]
fn s1_2_title_variants_no_empty_strings() {
    let item = open_item("x-1", "Task");
    for v in item.title_variants() {
        assert!(
            !v.is_empty(),
            "title_variants must not contain empty strings"
        );
    }
}

// §1.3 — infer_priority

#[test]
fn s1_3_p0_keywords() {
    for kw in &[
        "broken", "fails", "security", "blocked", "urgent", "panic", "segfault",
    ] {
        assert_eq!(
            infer_priority(&format!("CI {kw}"), None),
            "P0",
            "keyword '{kw}' should map to P0"
        );
    }
}

#[test]
fn s1_3_p1_keywords() {
    for kw in &["fix", "implement", "refactor", "wire"] {
        assert_eq!(
            infer_priority(&format!("{kw} this thing"), None),
            "P1",
            "keyword '{kw}' should map to P1"
        );
    }
}

#[test]
fn s1_3_p2_fallback() {
    assert_eq!(infer_priority("Explore someday", None), "P2");
}

#[test]
fn s1_3_description_contributes_to_priority() {
    assert_eq!(
        infer_priority("Someday idea", Some("This is urgent!")),
        "P0",
        "description field must contribute to priority inference"
    );
}

// §1.4 — active_items filter

#[test]
fn s1_4_active_items_includes_open_and_blocked() {
    let handoff = handoff_with_items(vec![
        item_with_status("hj-1", "open"),
        item_with_status("hj-2", "blocked"),
        item_with_status("hj-3", "done"),
        item_with_status("hj-4", "closed"),
    ]);
    let active: Vec<_> = handoff.active_items().collect();
    assert_eq!(active.len(), 2);
    assert!(
        active
            .iter()
            .all(|i| matches!(i.status.as_deref(), Some("open" | "blocked")))
    );
}

#[test]
fn s1_4_active_items_excludes_none_status() {
    let mut item = open_item("hj-1", "Task");
    item.status = None;
    let handoff = handoff_with_items(vec![item]);
    assert_eq!(handoff.active_items().count(), 0);
}

// §1.5 — HandoffState serialization

#[test]
fn s1_5_state_omits_empty_touched_files() {
    let state = HandoffState {
        branch: Some("main".into()),
        build: Some("clean".into()),
        tests: Some("passing".into()),
        ..HandoffState::default()
    };
    let yaml = serde_yaml::to_string(&state).expect("serialize");
    assert!(
        !yaml.contains("touched_files"),
        "empty touched_files must be omitted"
    );
}

#[test]
fn s1_5_state_round_trips() {
    let state = HandoffState {
        updated: Some("2026-04-18".into()),
        branch: Some("main".into()),
        build: Some("clean".into()),
        tests: Some("passing".into()),
        notes: Some("Some notes.".into()),
        touched_files: vec!["src/lib.rs".into()],
        last_log: None,
        extra: Default::default(),
    };
    let yaml = serde_yaml::to_string(&state).expect("serialize");
    let back: HandoffState = serde_yaml::from_str(&yaml).expect("deserialize");
    assert_eq!(back.branch, state.branch);
    assert_eq!(back.build, state.build);
    assert_eq!(back.tests, state.tests);
    assert_eq!(back.notes, state.notes);
    assert_eq!(back.touched_files, state.touched_files);
}

// §1.6 — titleize_slug

#[test]
fn s1_6_titleize_slug_basic() {
    assert_eq!(titleize_slug("wire-render-pass"), "Wire Render Pass");
}

#[test]
fn s1_6_titleize_slug_single_word() {
    assert_eq!(titleize_slug("implement"), "Implement");
}

#[test]
fn s1_6_titleize_slug_skips_empty_segments() {
    // double-dash produces empty segment that must be skipped
    let result = titleize_slug("a--b");
    assert!(
        !result.contains("  "),
        "consecutive spaces indicate empty segment not skipped"
    );
}

// §1.7 — sanitize_name and default_id_prefix

#[test]
fn s1_7_sanitize_name_lowercases_and_replaces() {
    assert_eq!(sanitize_name("My Project/CLI"), "my-project-cli");
    assert_eq!(sanitize_name("  leading "), "leading");
}

#[test]
fn s1_7_default_id_prefix_max_seven_chars() {
    let prefix = default_id_prefix("very-long-project-name");
    assert!(
        prefix.len() <= 7,
        "prefix must be at most 7 characters, got {}",
        prefix.len()
    );
}

#[test]
fn s1_7_default_id_prefix_exact_seven() {
    assert_eq!(default_id_prefix("atelier"), "atelier");
}

// ─── helpers ────────────────────────────────────────────────────────────────

fn open_item(id: &str, title: &str) -> HandoffItem {
    HandoffItem {
        id: id.into(),
        status: Some("open".into()),
        title: title.into(),
        ..HandoffItem::default()
    }
}

fn item_with_status(id: &str, status: &str) -> HandoffItem {
    HandoffItem {
        id: id.into(),
        status: Some(status.into()),
        title: format!("Task {id}"),
        ..HandoffItem::default()
    }
}

fn handoff_with_items(items: Vec<HandoffItem>) -> Handoff {
    Handoff {
        items,
        ..Handoff::default()
    }
}
