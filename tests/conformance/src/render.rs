// §5 — hj-render → hj-core adapter contracts

use hj_core::{Handoff, HandoffItem, HandoffState, LogEntry};
use hj_render::{render_handover_markdown, render_markdown};

// §5.1 — render_markdown structure

#[test]
fn s5_1_starts_with_handoff_header() {
    let handoff = Handoff {
        project: Some("hj".into()),
        updated: Some("2026-04-18".into()),
        ..Handoff::default()
    };
    let rendered = render_markdown(&handoff, None);
    assert!(
        rendered.starts_with("# Handoff — hj (2026-04-18)\n"),
        "must start with '# Handoff — <project> (<updated>)\\n', got: {rendered:?}"
    );
}

#[test]
fn s5_1_contains_branch_build_tests_line() {
    let state = HandoffState {
        branch: Some("main".into()),
        build: Some("clean".into()),
        tests: Some("passing".into()),
        ..HandoffState::default()
    };
    let rendered = render_markdown(&Handoff::default(), Some(&state));
    assert!(
        rendered.contains("**Branch:** main | **Build:** clean | **Tests:** passing"),
        "must contain branch/build/tests line"
    );
}

#[test]
fn s5_1_contains_items_and_log_sections() {
    let rendered = render_markdown(&Handoff::default(), None);
    assert!(rendered.contains("## Items"));
    assert!(rendered.contains("## Log"));
}

#[test]
fn s5_1_unknown_fallback_when_state_is_none() {
    let rendered = render_markdown(&Handoff::default(), None);
    assert!(
        rendered.contains("unknown"),
        "missing state must render 'unknown'"
    );
}

// §5.2 — item sort order

#[test]
fn s5_2_items_sorted_p0_before_p1_before_p2() {
    let handoff = Handoff {
        project: Some("hj".into()),
        updated: Some("2026-04-18".into()),
        items: vec![
            item("hj-3", "P2", "open", "P2 task"),
            item("hj-1", "P0", "open", "P0 task"),
            item("hj-2", "P1", "open", "P1 task"),
        ],
        ..Handoff::default()
    };
    let rendered = render_markdown(&handoff, None);
    let p0_pos = rendered.find("P0 task").unwrap();
    let p1_pos = rendered.find("P1 task").unwrap();
    let p2_pos = rendered.find("P2 task").unwrap();
    assert!(p0_pos < p1_pos, "P0 must appear before P1");
    assert!(p1_pos < p2_pos, "P1 must appear before P2");
}

#[test]
fn s5_2_open_before_blocked_within_same_priority() {
    let handoff = Handoff {
        project: Some("hj".into()),
        updated: Some("2026-04-18".into()),
        items: vec![
            item("hj-2", "P1", "blocked", "Blocked task"),
            item("hj-1", "P1", "open", "Open task"),
        ],
        ..Handoff::default()
    };
    let rendered = render_markdown(&handoff, None);
    let open_pos = rendered.find("Open task").unwrap();
    let blocked_pos = rendered.find("Blocked task").unwrap();
    assert!(open_pos < blocked_pos, "open must appear before blocked");
}

#[test]
fn s5_2_done_items_excluded() {
    let handoff = Handoff {
        project: Some("hj".into()),
        updated: Some("2026-04-18".into()),
        items: vec![
            item("hj-1", "P1", "open", "Visible"),
            item("hj-2", "P1", "done", "Hidden"),
        ],
        ..Handoff::default()
    };
    let rendered = render_markdown(&handoff, None);
    assert!(rendered.contains("Visible"));
    assert!(
        !rendered.contains("Hidden"),
        "done items must not appear in rendered output"
    );
}

// §5.3 — render_handover_markdown structure

#[test]
fn s5_3_starts_with_state_header() {
    let rendered = render_handover_markdown(&Handoff::default(), None);
    assert!(
        rendered.starts_with("## State\n"),
        "handover must start with '## State\\n', got: {rendered:?}"
    );
}

#[test]
fn s5_3_status_line_has_no_bold_markers() {
    let state = HandoffState {
        branch: Some("main".into()),
        build: Some("clean".into()),
        tests: Some("passing".into()),
        ..HandoffState::default()
    };
    let rendered = render_handover_markdown(&Handoff::default(), Some(&state));
    assert!(
        rendered.contains("Branch: main | Build: clean | Tests: passing"),
        "handover status line must not use bold markers"
    );
    assert!(
        !rendered.contains("**Branch:**"),
        "handover must not use bold markers"
    );
}

#[test]
fn s5_3_notes_appear_between_status_and_items() {
    let state = HandoffState {
        branch: Some("main".into()),
        build: Some("clean".into()),
        tests: Some("passing".into()),
        notes: Some("Ready for review.".into()),
        ..HandoffState::default()
    };
    let rendered = render_handover_markdown(&Handoff::default(), Some(&state));
    let notes_pos = rendered.find("Ready for review.").unwrap();
    let items_pos = rendered.find("## Items").unwrap();
    assert!(notes_pos < items_pos, "notes must appear before ## Items");
}

// §5.4 — commit formatting in log

#[test]
fn s5_4_log_entry_with_commits_includes_sha() {
    let handoff = Handoff {
        project: Some("hj".into()),
        updated: Some("2026-04-18".into()),
        log: vec![LogEntry {
            date: Some("2026-04-18".into()),
            summary: "Did the thing".into(),
            commits: vec!["abc1234".into()],
            ..LogEntry::default()
        }],
        ..Handoff::default()
    };
    let rendered = render_markdown(&handoff, None);
    assert!(
        rendered.contains("- 2026-04-18: Did the thing [abc1234]"),
        "log entry with commit must include sha in brackets"
    );
}

#[test]
fn s5_4_log_entry_without_commits_has_no_brackets() {
    let handoff = Handoff {
        project: Some("hj".into()),
        updated: Some("2026-04-18".into()),
        log: vec![LogEntry {
            date: Some("2026-04-18".into()),
            summary: "No commit".into(),
            commits: vec![],
            ..LogEntry::default()
        }],
        ..Handoff::default()
    };
    let rendered = render_markdown(&handoff, None);
    assert!(
        rendered.contains("- 2026-04-18: No commit\n"),
        "log entry without commits must not have brackets"
    );
    assert!(!rendered.contains("- 2026-04-18: No commit ["));
}

#[test]
fn s5_4_log_capped_at_five_entries() {
    let log = (1u8..=7)
        .map(|i| LogEntry {
            date: Some("2026-04-18".into()),
            summary: format!("Entry {i}"),
            ..LogEntry::default()
        })
        .collect();
    let handoff = Handoff {
        project: Some("hj".into()),
        updated: Some("2026-04-18".into()),
        log,
        ..Handoff::default()
    };
    let rendered = render_markdown(&handoff, None);
    assert!(
        !rendered.contains("Entry 6"),
        "log must be capped at 5 entries"
    );
    assert!(!rendered.contains("Entry 7"));
}

// ─── helpers ────────────────────────────────────────────────────────────────

fn item(id: &str, priority: &str, status: &str, title: &str) -> HandoffItem {
    HandoffItem {
        id: id.into(),
        priority: Some(priority.into()),
        status: Some(status.into()),
        title: title.into(),
        ..HandoffItem::default()
    }
}
