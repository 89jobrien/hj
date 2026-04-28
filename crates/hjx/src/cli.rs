use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "hj")]
#[command(about = "Rust implementation for handoff state, reconciliation, rendering, and closeout")]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Commands,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Commands {
    Detect(DetectArgs),
    Handoff(CloseArgs),
    Handon(TargetArgs),
    Handover(TargetArgs),
    HandoffDb(DbArgs),
    Handup(HandupArgs),
    Install(InstallArgs),
    Update(UpdateArgs),
    UpdateAll(UpdateArgs),
    Refresh(RefreshArgs),
    Reconcile(TargetArgs),
    Audit(TargetArgs),
    Close(CloseArgs),
}

#[derive(Debug, Args)]
pub(crate) struct DetectArgs {
    #[arg(long)]
    pub(crate) name: bool,
    #[arg(long)]
    pub(crate) root: bool,
    #[arg(long)]
    pub(crate) project: bool,
    #[arg(long)]
    pub(crate) init: bool,
}

#[derive(Debug, Args)]
pub(crate) struct RefreshArgs {
    #[arg(long)]
    pub(crate) force: bool,
}

#[derive(Debug, Args)]
pub(crate) struct InstallArgs {
    #[arg(long, default_value = "~/.local")]
    pub(crate) root: String,
}

#[derive(Debug, Args, Clone)]
pub(crate) struct UpdateArgs {
    #[arg(long, default_value = "~/.local")]
    pub(crate) root: String,
}

#[derive(Debug, Args)]
pub(crate) struct HandupArgs {
    #[arg(long, default_value_t = 5)]
    pub(crate) max_depth: usize,
}

#[derive(Debug, Args)]
pub(crate) struct DbArgs {
    #[command(subcommand)]
    pub(crate) command: DbCommand,
}

#[derive(Debug, Subcommand)]
pub(crate) enum DbCommand {
    Init,
    Upsert(DbUpsertArgs),
    Query(DbProjectArgs),
    Complete(DbItemArgs),
    Status(DbStatusArgs),
}

#[derive(Debug, Args)]
pub(crate) struct DbProjectArgs {
    #[arg(long)]
    pub(crate) project: String,
}

#[derive(Debug, Args)]
pub(crate) struct DbUpsertArgs {
    #[arg(long)]
    pub(crate) project: String,
    #[arg(long)]
    pub(crate) handoff: PathBuf,
}

#[derive(Debug, Args)]
pub(crate) struct DbItemArgs {
    #[arg(long)]
    pub(crate) project: String,
    #[arg(long)]
    pub(crate) id: String,
}

#[derive(Debug, Args)]
pub(crate) struct DbStatusArgs {
    #[arg(long)]
    pub(crate) project: String,
    #[arg(long)]
    pub(crate) id: String,
    #[arg(long)]
    pub(crate) status: String,
}

#[derive(Debug, Args, Clone)]
pub(crate) struct TargetArgs {
    #[arg(long)]
    pub(crate) handoff: Option<PathBuf>,
    #[arg(long)]
    pub(crate) project: Option<String>,
}

#[derive(Debug, Args)]
pub(crate) struct CloseArgs {
    #[command(flatten)]
    pub(crate) target: TargetArgs,
    #[arg(long)]
    pub(crate) force_refresh: bool,
    #[arg(long)]
    pub(crate) allow_create: bool,
    #[arg(long)]
    pub(crate) build: Option<String>,
    #[arg(long)]
    pub(crate) tests: Option<String>,
    #[arg(long)]
    pub(crate) notes: Option<String>,
    #[arg(long)]
    pub(crate) log_summary: Option<String>,
    #[arg(long = "commit")]
    pub(crate) commits: Vec<String>,
}
