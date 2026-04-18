mod alias;
mod cli;
mod handoff;
mod handup;
mod install;

use std::process;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};
use hj_core::ReconcileMode;

pub fn main_entry() {
    if let Err(error) = run() {
        eprintln!("error: {error:#}");
        process::exit(1);
    }
}

pub fn run() -> Result<()> {
    let cli = Cli::parse_from(alias::rewrite_args_for_alias(std::env::args_os()));
    match cli.command {
        Commands::Detect(args) => handoff::detect(args),
        Commands::Handoff(args) => handoff::close(args),
        Commands::Handon(args) => handoff::handon(args),
        Commands::Handover(args) => handoff::handover(args),
        Commands::HandoffDb(args) => handoff::handoff_db(args),
        Commands::Handup(args) => handup::handup(args),
        Commands::Install(args) => install::install(args),
        Commands::Update(args) | Commands::UpdateAll(args) => install::update(args),
        Commands::Refresh(args) => handoff::refresh(args),
        Commands::Reconcile(args) => handoff::reconcile(args, ReconcileMode::Sync),
        Commands::Audit(args) => handoff::reconcile(args, ReconcileMode::Audit),
        Commands::Close(args) => handoff::close(args),
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::{OsStr, OsString};

    use clap::Parser;

    use crate::{
        alias::rewrite_args_for_alias,
        cli::{Cli, Commands, InstallArgs, TargetArgs, UpdateArgs},
    };

    // §6.1 — alias rewriting

    #[test]
    fn s6_1_handoff_detect_alias_rewrites_to_detect() {
        let rewritten = rewrite_args_for_alias([
            OsString::from("handoff-detect"),
            OsString::from("--project"),
        ]);
        assert_eq!(
            rewritten,
            vec![
                OsString::from("hj"),
                OsString::from("detect"),
                OsString::from("--project")
            ]
        );
    }

    #[test]
    fn s6_1_handoff_alias_rewrites_to_handoff() {
        let rewritten = rewrite_args_for_alias([OsString::from("handoff")]);
        assert_eq!(
            rewritten,
            vec![OsString::from("hj"), OsString::from("handoff")]
        );
    }

    #[test]
    fn s6_1_handon_alias_rewrites_to_handon() {
        let rewritten = rewrite_args_for_alias([OsString::from("handon")]);
        assert_eq!(
            rewritten,
            vec![OsString::from("hj"), OsString::from("handon")]
        );
    }

    #[test]
    fn s6_1_handover_alias_rewrites_to_handover() {
        let rewritten = rewrite_args_for_alias([OsString::from("handover")]);
        assert_eq!(
            rewritten,
            vec![OsString::from("hj"), OsString::from("handover")]
        );
    }

    #[test]
    fn s6_1_handup_alias_rewrites_to_handup() {
        let rewritten = rewrite_args_for_alias([OsString::from("handup"), OsString::from("--max-depth"), OsString::from("3")]);
        assert_eq!(
            rewritten,
            vec![
                OsString::from("hj"),
                OsString::from("handup"),
                OsString::from("--max-depth"),
                OsString::from("3"),
            ]
        );
    }

    #[test]
    fn s6_1_handoff_db_alias_rewrites_to_handoff_db() {
        let rewritten = rewrite_args_for_alias([OsString::from("handoff-db"), OsString::from("query")]);
        assert_eq!(
            rewritten,
            vec![
                OsString::from("hj"),
                OsString::from("handoff-db"),
                OsString::from("query"),
            ]
        );
    }

    #[test]
    fn s6_1_non_alias_passthrough() {
        let rewritten = rewrite_args_for_alias([OsString::from("hj"), OsString::from("detect")]);
        assert_eq!(rewritten, vec![OsString::from("hj"), OsString::from("detect")]);
    }

    // §6.2 — CLI parsing

    #[test]
    fn s6_2_install_command_uses_default_root() {
        let cli = Cli::parse_from([OsStr::new("hj"), OsStr::new("install")]);
        match cli.command {
            Commands::Install(InstallArgs { root }) => assert_eq!(root, "~/.local"),
            other => panic!("expected install command, got {other:?}"),
        }
    }

    #[test]
    fn s6_2_update_command_uses_default_root() {
        let cli = Cli::parse_from([OsStr::new("hj"), OsStr::new("update")]);
        match cli.command {
            Commands::Update(UpdateArgs { root }) => assert_eq!(root, "~/.local"),
            other => panic!("expected update command, got {other:?}"),
        }
    }

    #[test]
    fn s6_2_update_all_command_parses_separately() {
        let cli = Cli::parse_from([OsStr::new("hj"), OsStr::new("update-all")]);
        match cli.command {
            Commands::UpdateAll(UpdateArgs { root }) => assert_eq!(root, "~/.local"),
            other => panic!("expected update-all command, got {other:?}"),
        }
    }

    #[test]
    fn s6_2_handon_command_parses_target_args() {
        let cli = Cli::parse_from([
            OsStr::new("hj"),
            OsStr::new("handon"),
            OsStr::new("--project"),
            OsStr::new("hj"),
        ]);
        match cli.command {
            Commands::Handon(TargetArgs { project, .. }) => {
                assert_eq!(project.as_deref(), Some("hj"))
            }
            other => panic!("expected handon command, got {other:?}"),
        }
    }
}
