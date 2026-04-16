use std::{fs, path::Path, process};

use anyhow::{Context, Result, anyhow, bail};
use hj_core::{
    ExtraEntry, Handoff, HandoffItem, HandoffState, LogEntry, ReconcileMode, ReconcileReport,
    build_reconcile_plan,
};
use hj_doob::{DoobClient, ensure_doob_on_path, map_priority};
use hj_git::{RepoContext, branch_name, current_short_head, discover, today};
use hj_render::{render_handover_markdown, render_markdown};
use hj_sqlite::{HandoffDb, HandoffRow};

use crate::cli::{CloseArgs, DbArgs, DbCommand, DetectArgs, RefreshArgs, TargetArgs};

trait TodoMemoryBackend {
    fn snapshot(&self, project: &str) -> Result<hj_core::TodoSnapshot>;
    fn create(&self, project: &str, item: &hj_core::ReconcileCreate) -> Result<()>;
}

impl TodoMemoryBackend for DoobClient {
    fn snapshot(&self, project: &str) -> Result<hj_core::TodoSnapshot> {
        DoobClient::snapshot(self, project)
    }

    fn create(&self, project: &str, item: &hj_core::ReconcileCreate) -> Result<()> {
        let tags = vec!["handoff".to_string(), project.to_string()];
        self.add(
            project,
            &item.title,
            map_priority(item.priority.as_deref()),
            &tags,
        )
    }
}

pub(crate) fn detect(args: DetectArgs) -> Result<()> {
    let context = discover(Path::new("."))?;
    if args.init {
        context.refresh(false)?;
    }
    let paths = context.paths(None)?;

    if args.root {
        println!("{}", paths.repo_root.display());
        return Ok(());
    }
    if args.project {
        println!("{}", paths.project);
        return Ok(());
    }
    if args.name {
        let file_name = paths
            .handoff_path
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| anyhow!("handoff path has no filename"))?;
        println!("{file_name}");
        return Ok(());
    }

    if paths.handoff_path.exists() {
        println!("{}", paths.handoff_path.display());
        return Ok(());
    }

    if let Some(migrated) = context.migrate_root_handoff(&paths.handoff_path)? {
        println!("{}", migrated.display());
        return Ok(());
    }

    println!("{}", paths.handoff_path.display());
    process::exit(2);
}

pub(crate) fn refresh(args: RefreshArgs) -> Result<()> {
    let context = discover(Path::new("."))?;
    let report = context.refresh(args.force)?;
    println!(
        "refreshed {} (packages: {})",
        report.ctx_dir.display(),
        report.packages.join(", ")
    );
    Ok(())
}

pub(crate) fn handon(args: TargetArgs) -> Result<()> {
    let context = discover(Path::new("."))?;
    let (paths, mut handoff) = load_target_handoff(&context, args, false)?;
    apply_sqlite_overrides(&paths.project, &mut handoff)?;
    let state = load_state(&paths.state_path)?;
    let review = collect_review_on_wake(&handoff);
    let triage = classify_items(&handoff);

    println!("## Handoff Triage — {}", paths.repo_root.display());
    if let Some(state) = state.as_ref() {
        println!(
            "Branch: {} | Build: {} | Tests: {}",
            state.branch.as_deref().unwrap_or("unknown"),
            state.build.as_deref().unwrap_or("unknown"),
            state.tests.as_deref().unwrap_or("unknown")
        );
        if let Some(notes) = state.notes.as_deref().filter(|notes| !notes.is_empty()) {
            println!("{notes}");
        }
        if !state.touched_files.is_empty() {
            println!("Recently touched: {}", state.touched_files.join(", "));
        }
    }
    println!();

    if !review.is_empty() {
        println!("## Review on Wake");
        for line in review {
            println!("{line}");
        }
        println!();
    }

    print_triage_bucket("P0", &triage.p0);
    print_triage_bucket("P1", &triage.p1);
    print_triage_bucket("P2", &triage.p2);

    if triage.p0.is_empty() && triage.p1.is_empty() && triage.p2.is_empty() {
        println!("No open handoff items.");
    }

    Ok(())
}

pub(crate) fn handover(args: TargetArgs) -> Result<()> {
    let context = discover(Path::new("."))?;
    let (paths, mut handoff) = load_target_handoff(&context, args, false)?;
    apply_sqlite_overrides(&paths.project, &mut handoff)?;
    let state = load_state(&paths.state_path)?;
    let rendered = render_handover_markdown(&handoff, state.as_ref());
    fs::write(&paths.handover_path, rendered)
        .with_context(|| format!("failed to write {}", paths.handover_path.display()))?;
    println!("{}", paths.handover_path.display());
    Ok(())
}

pub(crate) fn handoff_db(args: DbArgs) -> Result<()> {
    let db = HandoffDb::new()?;
    match args.command {
        DbCommand::Init => {
            let db_path = db.init()?;
            println!("db initialized: {}", db_path.display());
        }
        DbCommand::Upsert(args) => {
            let contents = fs::read_to_string(&args.handoff)
                .with_context(|| format!("failed to read {}", args.handoff.display()))?;
            let handoff: Handoff = serde_yaml::from_str(&contents)
                .with_context(|| format!("failed to parse {}", args.handoff.display()))?;
            let today = today(Path::new("."))?;
            let report = db.upsert(&args.project, &handoff, &today)?;
            println!(
                "synced {} item(s) for project '{}'",
                report.synced, args.project
            );
        }
        DbCommand::Query(args) => {
            for row in db.query(&args.project)? {
                println!(
                    "{}|{}|{}|{}|{}",
                    row.id, row.priority, row.status, row.completed, row.updated
                );
            }
        }
        DbCommand::Complete(args) => {
            let today = today(Path::new("."))?;
            db.complete(&args.project, &args.id, &today)?;
            println!("marked done: {}/{}", args.project, args.id);
        }
        DbCommand::Status(args) => {
            let today = today(Path::new("."))?;
            db.set_status(&args.project, &args.id, &args.status, &today)?;
            println!(
                "status updated: {}/{} -> {}",
                args.project, args.id, args.status
            );
        }
    }
    Ok(())
}

pub(crate) fn reconcile(args: TargetArgs, mode: ReconcileMode) -> Result<()> {
    let context = discover(Path::new("."))?;
    let doob = DoobClient::new(&context.repo_root);
    ensure_doob_on_path(&context.repo_root)?;
    let (paths, handoff) = load_target_handoff(&context, args, false)?;
    let report = reconcile_handoff(&doob, &paths.project, &handoff, mode)?;
    print_reconcile_report(mode, &report);

    if mode == ReconcileMode::Audit
        && (!report.not_captured.is_empty() || !report.closed_upstream.is_empty())
    {
        process::exit(1);
    }

    Ok(())
}

pub(crate) fn close(args: CloseArgs) -> Result<()> {
    let context = discover(Path::new("."))?;
    context.refresh(args.force_refresh)?;

    let (paths, mut handoff) =
        load_target_handoff(&context, args.target.clone(), args.allow_create)?;
    let today = today(&context.repo_root)?;
    handoff.ensure_project(&paths.project);
    handoff.ensure_id_prefix(&paths.project);
    handoff.updated = Some(today.clone());

    if let Some(summary) = args.log_summary {
        let commits = if args.commits.is_empty() {
            current_short_head(&context.repo_root)
                .map(|hash| vec![hash])
                .unwrap_or_default()
        } else {
            args.commits.clone()
        };

        handoff.log.insert(
            0,
            LogEntry {
                date: Some(today.clone()),
                summary,
                commits,
                ..LogEntry::default()
            },
        );
    }

    let state = build_state(&context, &paths, args.build, args.tests, args.notes)?;
    fs::create_dir_all(&paths.ctx_dir)
        .with_context(|| format!("failed to create {}", paths.ctx_dir.display()))?;
    fs::write(&paths.handoff_path, serde_yaml::to_string(&handoff)?)
        .with_context(|| format!("failed to write {}", paths.handoff_path.display()))?;
    fs::write(&paths.state_path, serde_yaml::to_string(&state)?)
        .with_context(|| format!("failed to write {}", paths.state_path.display()))?;

    let db = HandoffDb::new()?;
    let upsert = db.upsert(&paths.project, &handoff, &today)?;
    let doob = DoobClient::new(&context.repo_root);
    ensure_doob_on_path(&context.repo_root)?;
    let reconcile_report = reconcile_handoff(&doob, &paths.project, &handoff, ReconcileMode::Sync)?;
    let rendered = render_markdown(&handoff, Some(&state));
    fs::write(&paths.rendered_path, rendered)
        .with_context(|| format!("failed to write {}", paths.rendered_path.display()))?;
    let handover = render_handover_markdown(&handoff, Some(&state));
    fs::write(&paths.handover_path, handover)
        .with_context(|| format!("failed to write {}", paths.handover_path.display()))?;

    println!("Closed {}", paths.project);
    println!("Handoff: {}", paths.handoff_path.display());
    println!("State:   {}", paths.state_path.display());
    println!("Render:  {}", paths.rendered_path.display());
    println!("Handover: {}", paths.handover_path.display());
    println!(
        "SQLite:  synced {} item(s) into {}",
        upsert.synced,
        upsert.db_path.display()
    );
    print_reconcile_report(ReconcileMode::Sync, &reconcile_report);
    Ok(())
}

fn build_state(
    context: &RepoContext,
    paths: &hj_git::HandoffPaths,
    build: Option<String>,
    tests: Option<String>,
    notes: Option<String>,
) -> Result<HandoffState> {
    let existing = if paths.state_path.exists() {
        let contents = fs::read_to_string(&paths.state_path)
            .with_context(|| format!("failed to read {}", paths.state_path.display()))?;
        serde_yaml::from_str::<HandoffState>(&contents)
            .with_context(|| format!("failed to parse {}", paths.state_path.display()))?
    } else {
        HandoffState::default()
    };

    Ok(HandoffState {
        updated: Some(today(&context.repo_root)?),
        branch: Some(branch_name(&context.repo_root).unwrap_or_else(|_| "unknown".into())),
        build: build.or(existing.build).or(Some("unknown".into())),
        tests: tests.or(existing.tests).or(Some("unknown".into())),
        notes: notes.or(existing.notes),
        touched_files: context.working_tree_files()?,
        extra: existing.extra,
    })
}

fn load_state(path: &Path) -> Result<Option<HandoffState>> {
    if !path.exists() {
        return Ok(None);
    }

    let contents =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let state = serde_yaml::from_str::<HandoffState>(&contents)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(Some(state))
}

fn load_target_handoff(
    context: &RepoContext,
    args: TargetArgs,
    allow_create: bool,
) -> Result<(hj_git::HandoffPaths, Handoff)> {
    let explicit_project = args.project.as_deref();
    let paths = context.paths(explicit_project)?;
    let handoff_path = args.handoff.unwrap_or_else(|| paths.handoff_path.clone());

    if handoff_path.exists() {
        let contents = fs::read_to_string(&handoff_path)
            .with_context(|| format!("failed to read {}", handoff_path.display()))?;
        let handoff: Handoff = serde_yaml::from_str(&contents)
            .with_context(|| format!("failed to parse {}", handoff_path.display()))?;
        let mut resolved = paths;
        resolved.handoff_path = handoff_path;
        return Ok((resolved, handoff));
    }

    if let Some(migrated) = context.migrate_root_handoff(&handoff_path)? {
        let contents = fs::read_to_string(&migrated)
            .with_context(|| format!("failed to read {}", migrated.display()))?;
        let handoff: Handoff = serde_yaml::from_str(&contents)
            .with_context(|| format!("failed to parse {}", migrated.display()))?;
        let mut resolved = paths;
        resolved.handoff_path = migrated;
        return Ok((resolved, handoff));
    }

    if allow_create {
        let handoff = Handoff {
            project: Some(paths.project.clone()),
            id: Some(hj_core::default_id_prefix(&paths.project)),
            updated: Some(today(&context.repo_root)?),
            ..Handoff::default()
        };
        return Ok((paths, handoff));
    }

    bail!("handoff file not found: {}", handoff_path.display())
}

fn reconcile_handoff(
    backend: &impl TodoMemoryBackend,
    project: &str,
    handoff: &Handoff,
    mode: ReconcileMode,
) -> Result<ReconcileReport> {
    let snapshot = backend.snapshot(project)?;
    let plan = build_reconcile_plan(project, handoff, &snapshot, mode);

    if mode == ReconcileMode::Sync {
        for create in &plan.creates {
            backend.create(project, create)?;
        }
    }

    Ok(plan.report)
}

fn apply_sqlite_overrides(project: &str, handoff: &mut Handoff) -> Result<()> {
    let db = match HandoffDb::new() {
        Ok(db) => db,
        Err(_) => return Ok(()),
    };
    let rows = match db.query(project) {
        Ok(rows) => rows,
        Err(_) => return Ok(()),
    };
    apply_handoff_rows(handoff, &rows);
    Ok(())
}

fn apply_handoff_rows(handoff: &mut Handoff, rows: &[HandoffRow]) {
    for row in rows {
        let Some(item) = handoff.items.iter_mut().find(|item| item.id == row.id) else {
            continue;
        };

        if !row.priority.is_empty() {
            item.priority = Some(row.priority.clone());
        }
        if !row.status.is_empty() && item.status.as_deref() != Some(row.status.as_str()) {
            item.status = Some(row.status.clone());
        }
        if !row.completed.is_empty() {
            item.completed = Some(row.completed.clone());
        }
    }
}

#[derive(Debug, Default)]
struct TriageBuckets {
    p0: Vec<String>,
    p1: Vec<String>,
    p2: Vec<String>,
}

fn collect_review_on_wake(handoff: &Handoff) -> Vec<String> {
    let mut lines = Vec::new();
    for item in &handoff.items {
        for entry in item.extra.iter().filter(is_unreviewed_human_edit) {
            let date = entry.date.as_deref().unwrap_or("unknown");
            let field = entry.field.as_deref().unwrap_or("unknown");
            let value = entry.value.as_deref().unwrap_or("unknown");
            lines.push(format!(
                "- [{}] \"{}\" — human edited `{field}` -> `{value}` on {date}",
                item.id, item.title
            ));
            if let Some(note) = entry.note.as_deref().filter(|note| !note.is_empty()) {
                lines.push(format!("  {note}"));
            }
        }
    }
    lines
}

fn is_unreviewed_human_edit(entry: &&ExtraEntry) -> bool {
    entry.r#type.as_deref() == Some("human-edit")
        && entry
            .reviewed
            .as_deref()
            .is_none_or(|reviewed| reviewed.is_empty())
}

fn classify_items(handoff: &Handoff) -> TriageBuckets {
    let mut buckets = TriageBuckets::default();
    let mut items = handoff.active_items().collect::<Vec<_>>();
    items.sort_by(|left, right| {
        left.inferred_priority()
            .cmp(&right.inferred_priority())
            .then(left.id.cmp(&right.id))
    });

    for item in items {
        let line = format_triage_item(item);
        match item.inferred_priority().as_str() {
            "P0" => buckets.p0.push(line),
            "P1" => buckets.p1.push(line),
            _ => buckets.p2.push(line),
        }
    }

    buckets
}

fn format_triage_item(item: &HandoffItem) -> String {
    let name = item.name.as_deref().unwrap_or("-");
    let mut line = format!(
        "- [{}] [{}] \"{}\" -> {}",
        item.id,
        name,
        item.title,
        item.status.as_deref().unwrap_or("open")
    );
    if let Some(description) = item
        .description
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        line.push_str(&format!(": {description}"));
    }
    line
}

fn print_triage_bucket(label: &str, items: &[String]) {
    println!("{label}:");
    if items.is_empty() {
        println!("  - none");
    } else {
        for item in items {
            println!("  {item}");
        }
    }
}

fn print_reconcile_report(mode: ReconcileMode, report: &ReconcileReport) {
    println!("Reconciliation — {}", report.project);
    println!("===========================");
    println!("Captured in backend:      {} items", report.captured_count);
    if mode == ReconcileMode::Sync {
        println!("Created this run:         {} items", report.created_count);
    }
    println!(
        "Not captured:             {} items",
        report.not_captured.len()
    );
    println!("Orphaned backend items:   {} items", report.orphaned.len());
    println!(
        "Closed upstream:          {} items",
        report.closed_upstream.len()
    );

    print_list("Missing items:", &report.not_captured);
    print_list("Orphaned backend items:", &report.orphaned);
    print_list("Closed upstream:", &report.closed_upstream);
}

fn print_list(label: &str, items: &[String]) {
    if items.is_empty() {
        return;
    }
    println!("{label}");
    for item in items {
        println!("- {item}");
    }
}

#[cfg(test)]
mod tests {
    use hj_core::{ExtraEntry, Handoff, HandoffItem};
    use hj_sqlite::HandoffRow;

    use super::{apply_handoff_rows, collect_review_on_wake};

    #[test]
    fn sqlite_rows_override_handoff_status() {
        let mut handoff = Handoff {
            items: vec![HandoffItem {
                id: "hj-1".into(),
                priority: Some("P1".into()),
                status: Some("open".into()),
                ..HandoffItem::default()
            }],
            ..Handoff::default()
        };
        let rows = vec![HandoffRow {
            id: "hj-1".into(),
            priority: "P0".into(),
            status: "blocked".into(),
            completed: "2026-04-16".into(),
            updated: "2026-04-16".into(),
        }];

        apply_handoff_rows(&mut handoff, &rows);
        let item = &handoff.items[0];
        assert_eq!(item.priority.as_deref(), Some("P0"));
        assert_eq!(item.status.as_deref(), Some("blocked"));
        assert_eq!(item.completed.as_deref(), Some("2026-04-16"));
    }

    #[test]
    fn review_on_wake_finds_unreviewed_human_edits() {
        let handoff = Handoff {
            items: vec![HandoffItem {
                id: "hj-1".into(),
                title: "Ship handon".into(),
                extra: vec![ExtraEntry {
                    date: Some("2026-04-16".into()),
                    r#type: Some("human-edit".into()),
                    field: Some("title".into()),
                    value: Some("Ship handon".into()),
                    reviewed: None,
                    note: Some("Needs confirmation.".into()),
                    ..ExtraEntry::default()
                }],
                ..HandoffItem::default()
            }],
            ..Handoff::default()
        };

        let review = collect_review_on_wake(&handoff);
        assert_eq!(review.len(), 2);
        assert!(review[0].contains("[hj-1]"));
        assert!(review[1].contains("Needs confirmation."));
    }
}
