use std::{
    collections::BTreeSet,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process,
};

use anyhow::{Context, Result, anyhow, bail};
use clap::{Args, Parser, Subcommand};
use hj_core::{Handoff, HandoffItem, HandoffState, LogEntry};
use hj_doob::{
    DoobClient, ReconcileReport, TodoStatus, ensure_doob_on_path, map_priority, unique_titles,
};
use hj_git::{RepoContext, branch_name, current_short_head, discover, today};
use hj_render::render_markdown;
use hj_sqlite::HandoffDb;

#[derive(Debug, Parser)]
#[command(name = "hj")]
#[command(about = "Rust implementation for handoff state, reconciliation, rendering, and closeout")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Detect(DetectArgs),
    HandoffDb(DbArgs),
    Refresh(RefreshArgs),
    Reconcile(TargetArgs),
    Audit(TargetArgs),
    Close(CloseArgs),
}

#[derive(Debug, Args)]
struct DetectArgs {
    #[arg(long)]
    name: bool,
    #[arg(long)]
    root: bool,
    #[arg(long)]
    project: bool,
    #[arg(long)]
    init: bool,
}

#[derive(Debug, Args)]
struct RefreshArgs {
    #[arg(long)]
    force: bool,
}

#[derive(Debug, Args)]
struct DbArgs {
    #[command(subcommand)]
    command: DbCommand,
}

#[derive(Debug, Subcommand)]
enum DbCommand {
    Init,
    Upsert(DbUpsertArgs),
    Query(DbProjectArgs),
    Complete(DbItemArgs),
    Status(DbStatusArgs),
}

#[derive(Debug, Args)]
struct DbProjectArgs {
    #[arg(long)]
    project: String,
}

#[derive(Debug, Args)]
struct DbUpsertArgs {
    #[arg(long)]
    project: String,
    #[arg(long)]
    handoff: PathBuf,
}

#[derive(Debug, Args)]
struct DbItemArgs {
    #[arg(long)]
    project: String,
    #[arg(long)]
    id: String,
}

#[derive(Debug, Args)]
struct DbStatusArgs {
    #[arg(long)]
    project: String,
    #[arg(long)]
    id: String,
    #[arg(long)]
    status: String,
}

#[derive(Debug, Args, Clone)]
struct TargetArgs {
    #[arg(long)]
    handoff: Option<PathBuf>,
    #[arg(long)]
    project: Option<String>,
}

#[derive(Debug, Args)]
struct CloseArgs {
    #[command(flatten)]
    target: TargetArgs,
    #[arg(long)]
    force_refresh: bool,
    #[arg(long)]
    allow_create: bool,
    #[arg(long)]
    build: Option<String>,
    #[arg(long)]
    tests: Option<String>,
    #[arg(long)]
    notes: Option<String>,
    #[arg(long)]
    log_summary: Option<String>,
    #[arg(long = "commit")]
    commits: Vec<String>,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error:#}");
        process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse_from(rewrite_args_for_alias(std::env::args_os()));
    match cli.command {
        Commands::Detect(args) => detect(args),
        Commands::HandoffDb(args) => handoff_db(args),
        Commands::Refresh(args) => refresh(args),
        Commands::Reconcile(args) => reconcile(args, Mode::Sync),
        Commands::Audit(args) => reconcile(args, Mode::Audit),
        Commands::Close(args) => close(args),
    }
}

fn rewrite_args_for_alias(args: impl IntoIterator<Item = OsString>) -> Vec<OsString> {
    let args = args.into_iter().collect::<Vec<_>>();
    let Some(program) = args.first() else {
        return vec![OsString::from("hj")];
    };
    let Some(name) = Path::new(program)
        .file_name()
        .and_then(|value| value.to_str())
    else {
        return args;
    };

    let subcommand = match name {
        "handoff-detect" => Some("detect"),
        "handoff-db" => Some("handoff-db"),
        _ => None,
    };

    let Some(subcommand) = subcommand else {
        return args;
    };

    let mut rewritten = vec![OsString::from("hj"), OsString::from(subcommand)];
    rewritten.extend(args.into_iter().skip(1));
    rewritten
}

fn detect(args: DetectArgs) -> Result<()> {
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

fn refresh(args: RefreshArgs) -> Result<()> {
    let context = discover(Path::new("."))?;
    let report = context.refresh(args.force)?;
    println!(
        "refreshed {} (packages: {})",
        report.ctx_dir.display(),
        report.packages.join(", ")
    );
    Ok(())
}

fn handoff_db(args: DbArgs) -> Result<()> {
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
                "status updated: {}/{} → {}",
                args.project, args.id, args.status
            );
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum Mode {
    Sync,
    Audit,
}

fn reconcile(args: TargetArgs, mode: Mode) -> Result<()> {
    let context = discover(Path::new("."))?;
    let doob = DoobClient::new(&context.repo_root);
    ensure_doob_on_path(&context.repo_root)?;
    let (paths, handoff) = load_target_handoff(&context, args.clone(), false)?;
    let report = reconcile_handoff(&doob, &paths.project, &handoff, mode)?;
    print_reconcile_report(mode, &report);

    if mode == Mode::Audit
        && (!report.not_captured.is_empty() || !report.closed_upstream.is_empty())
    {
        process::exit(1);
    }

    Ok(())
}

fn close(args: CloseArgs) -> Result<()> {
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
    let reconcile_report = reconcile_handoff(&doob, &paths.project, &handoff, Mode::Sync)?;
    let rendered = render_markdown(&handoff, Some(&state));
    fs::write(&paths.rendered_path, rendered)
        .with_context(|| format!("failed to write {}", paths.rendered_path.display()))?;

    println!("Closed {}", paths.project);
    println!("Handoff: {}", paths.handoff_path.display());
    println!("State:   {}", paths.state_path.display());
    println!("Render:  {}", paths.rendered_path.display());
    println!(
        "SQLite:  synced {} item(s) into {}",
        upsert.synced,
        upsert.db_path.display()
    );
    print_reconcile_report(Mode::Sync, &reconcile_report);
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
    doob: &DoobClient,
    project: &str,
    handoff: &Handoff,
    mode: Mode,
) -> Result<ReconcileReport> {
    let pending = doob.list_titles(project, TodoStatus::Pending)?;
    let in_progress = doob.list_titles(project, TodoStatus::InProgress)?;
    let completed = doob.list_titles(project, TodoStatus::Completed)?;
    let cancelled = doob.list_titles(project, TodoStatus::Cancelled)?;

    let mut active_titles = unique_titles(
        pending
            .iter()
            .chain(in_progress.iter())
            .cloned()
            .collect::<Vec<_>>(),
    );
    let closed_titles = unique_titles(
        completed
            .iter()
            .chain(cancelled.iter())
            .cloned()
            .collect::<Vec<_>>(),
    );

    let mut captured_count = 0usize;
    let mut created_count = 0usize;
    let mut not_captured = Vec::new();
    let mut closed_upstream = Vec::new();
    let mut handoff_titles = BTreeSet::new();

    for item in handoff.active_items() {
        for variant in item.title_variants() {
            handoff_titles.insert(variant);
        }

        if contains_any(&active_titles, item) {
            captured_count += 1;
            continue;
        }

        if contains_any(&closed_titles, item) {
            closed_upstream.push(item.doob_title());
            continue;
        }

        match mode {
            Mode::Sync => {
                let title = item.doob_title();
                let tags = vec!["handoff".to_string(), project.to_string()];
                doob.add(
                    project,
                    &title,
                    map_priority(item.priority.as_deref()),
                    &tags,
                )?;
                active_titles.push(title);
                active_titles = unique_titles(active_titles);
                captured_count += 1;
                created_count += 1;
            }
            Mode::Audit => not_captured.push(item.doob_title()),
        }
    }

    let orphaned = active_titles
        .into_iter()
        .filter(|title| !handoff_titles.contains(title))
        .collect::<Vec<_>>();

    Ok(ReconcileReport {
        project: project.to_string(),
        captured_count,
        created_count,
        not_captured,
        orphaned,
        closed_upstream,
    })
}

fn contains_any(existing: &[String], item: &HandoffItem) -> bool {
    item.title_variants()
        .into_iter()
        .any(|variant| existing.iter().any(|title| title == &variant))
}

fn print_reconcile_report(mode: Mode, report: &ReconcileReport) {
    println!("Reconciliation — {}", report.project);
    println!("===========================");
    println!("Captured (HANDOFF→doob):  {} items", report.captured_count);
    if mode == Mode::Sync {
        println!("Created this run:         {} items", report.created_count);
    }
    println!(
        "Not captured:             {} items",
        report.not_captured.len()
    );
    println!("Orphaned todos:           {} items", report.orphaned.len());
    println!(
        "Closed upstream:          {} items",
        report.closed_upstream.len()
    );

    print_list("Missing items:", &report.not_captured);
    print_list("Orphaned todos:", &report.orphaned);
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
