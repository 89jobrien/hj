use hj_core::{Handoff, HandoffItem, HandoffState};

pub fn render_markdown(handoff: &Handoff, state: Option<&HandoffState>) -> String {
    let project = handoff.project.as_deref().unwrap_or("unknown");
    let updated = handoff.updated.as_deref().unwrap_or("unknown");
    let branch = state
        .and_then(|value| value.branch.as_deref())
        .unwrap_or("unknown");
    let build = state
        .and_then(|value| value.build.as_deref())
        .unwrap_or("unknown");
    let tests = state
        .and_then(|value| value.tests.as_deref())
        .unwrap_or("unknown");

    let mut out = String::new();
    out.push_str(&format!("# Handoff — {project} ({updated})\n\n"));
    out.push_str(&format!(
        "**Branch:** {branch} | **Build:** {build} | **Tests:** {tests}\n"
    ));
    if let Some(notes) = state
        .and_then(|value| value.notes.as_deref())
        .filter(|notes| !notes.is_empty())
    {
        out.push_str(&format!("{notes}\n"));
    }

    out.push_str("\n## Items\n\n");
    out.push_str("| ID | P | Status | Title |\n");
    out.push_str("|---|---|---|---|\n");

    for item in sorted_active_items(handoff) {
        out.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            item.id,
            item.priority.as_deref().unwrap_or("-"),
            item.status.as_deref().unwrap_or("-"),
            item.title
        ));
    }

    out.push_str("\n## Log\n\n");
    for entry in handoff.log.iter().take(5) {
        let date = entry.date.as_deref().unwrap_or("unknown");
        if entry.commits.is_empty() {
            out.push_str(&format!("- {date}: {}\n", entry.summary));
        } else {
            out.push_str(&format!(
                "- {date}: {} [{}]\n",
                entry.summary,
                entry.commits.join(", ")
            ));
        }
    }

    out
}

pub fn render_handover_markdown(handoff: &Handoff, state: Option<&HandoffState>) -> String {
    let branch = state
        .and_then(|value| value.branch.as_deref())
        .unwrap_or("unknown");
    let build = state
        .and_then(|value| value.build.as_deref())
        .unwrap_or("unknown");
    let tests = state
        .and_then(|value| value.tests.as_deref())
        .unwrap_or("unknown");

    let mut out = String::new();
    out.push_str("## State\n\n");
    out.push_str(&format!(
        "Branch: {branch} | Build: {build} | Tests: {tests}\n"
    ));
    if let Some(notes) = state
        .and_then(|value| value.notes.as_deref())
        .filter(|notes| !notes.is_empty())
    {
        out.push_str(&format!("{notes}\n"));
    }

    out.push_str("\n## Items\n\n");
    out.push_str("| ID | Priority | Status | Title |\n");
    out.push_str("|---|---|---|---|\n");

    for item in sorted_active_items(handoff) {
        out.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            item.id,
            item.priority.as_deref().unwrap_or("-"),
            item.status.as_deref().unwrap_or("-"),
            item.title
        ));
    }

    out.push_str("\n## Log\n\n");
    for entry in handoff.log.iter().take(5) {
        let date = entry.date.as_deref().unwrap_or("unknown");
        if entry.commits.is_empty() {
            out.push_str(&format!("- {date}: {}\n", entry.summary));
        } else {
            out.push_str(&format!(
                "- {date}: {} [{}]\n",
                entry.summary,
                entry.commits.join(", ")
            ));
        }
    }

    out
}

fn sorted_active_items(handoff: &Handoff) -> Vec<&HandoffItem> {
    let mut items: Vec<&HandoffItem> = handoff.active_items().collect();
    items.sort_by_key(|item| {
        (
            priority_rank(item.priority.as_deref()),
            status_rank(item.status.as_deref()),
            item.id.as_str(),
        )
    });
    items
}

fn priority_rank(priority: Option<&str>) -> u8 {
    match priority {
        Some("P0") => 0,
        Some("P1") => 1,
        Some("P2") => 2,
        _ => 9,
    }
}

fn status_rank(status: Option<&str>) -> u8 {
    match status {
        Some("open") => 0,
        Some("blocked") => 1,
        _ => 9,
    }
}

#[cfg(test)]
mod tests {
    use hj_core::{Handoff, HandoffItem, HandoffState, LogEntry};

    use super::{render_handover_markdown, render_markdown};

    #[test]
    fn renders_summary_markdown() {
        let handoff = Handoff {
            project: Some("hj".into()),
            updated: Some("2026-04-15".into()),
            items: vec![HandoffItem {
                id: "hj-1".into(),
                priority: Some("P1".into()),
                status: Some("open".into()),
                title: "Ship reconcile".into(),
                ..HandoffItem::default()
            }],
            log: vec![LogEntry {
                date: Some("2026-04-15".into()),
                summary: "Scaffolded workspace".into(),
                commits: vec!["abc1234".into()],
                ..LogEntry::default()
            }],
            ..Handoff::default()
        };
        let state = HandoffState {
            branch: Some("main".into()),
            build: Some("clean".into()),
            tests: Some("passing".into()),
            ..HandoffState::default()
        };

        let rendered = render_markdown(&handoff, Some(&state));
        assert!(rendered.contains("# Handoff — hj (2026-04-15)"));
        assert!(rendered.contains("| hj-1 | P1 | open | Ship reconcile |"));
        assert!(rendered.contains("- 2026-04-15: Scaffolded workspace [abc1234]"));
    }

    #[test]
    fn renders_handover_markdown() {
        let handoff = Handoff {
            items: vec![HandoffItem {
                id: "hj-1".into(),
                priority: Some("P1".into()),
                status: Some("open".into()),
                title: "Ship reconcile".into(),
                ..HandoffItem::default()
            }],
            log: vec![LogEntry {
                date: Some("2026-04-15".into()),
                summary: "Scaffolded workspace".into(),
                commits: vec!["abc1234".into()],
                ..LogEntry::default()
            }],
            ..Handoff::default()
        };
        let state = HandoffState {
            branch: Some("main".into()),
            build: Some("clean".into()),
            tests: Some("passing".into()),
            notes: Some("Ready for follow-up.".into()),
            ..HandoffState::default()
        };

        let rendered = render_handover_markdown(&handoff, Some(&state));
        assert!(rendered.contains("## State"));
        assert!(rendered.contains("Branch: main | Build: clean | Tests: passing"));
        assert!(rendered.contains("Ready for follow-up."));
        assert!(rendered.contains("| hj-1 | P1 | open | Ship reconcile |"));
    }
}
