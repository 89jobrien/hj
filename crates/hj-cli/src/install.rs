use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, anyhow, bail};
use hj_git::discover;

use crate::cli::{InstallArgs, UpdateArgs};

pub(crate) fn install(args: InstallArgs) -> Result<()> {
    let context = discover(Path::new("."))?;
    let root = expand_home(&args.root)?;
    run_cargo_install(
        &[
            "install",
            "--path",
            "crates/hj-cli",
            "--bins",
            "--force",
            "--root",
        ],
        &[root.as_os_str()],
        Some(&context.repo_root),
    )?;
    print_install_summary("Installed", &root);
    Ok(())
}

pub(crate) fn update(args: UpdateArgs) -> Result<()> {
    let root = expand_home(&args.root)?;
    run_cargo_install(
        &[
            "install", "hj-cli", "--bins", "--locked", "--force", "--root",
        ],
        &[root.as_os_str()],
        None,
    )?;
    print_install_summary("Updated", &root);
    Ok(())
}

fn expand_home(value: &str) -> Result<PathBuf> {
    if value == "~/.local" {
        let home = dirs::home_dir().ok_or_else(|| anyhow!("could not determine home directory"))?;
        return Ok(home.join(".local"));
    }
    Ok(PathBuf::from(value))
}

fn run_cargo_install(
    prefix_args: &[&str],
    extra_args: &[&OsStr],
    cwd: Option<&Path>,
) -> Result<()> {
    let mut command = Command::new("cargo");
    command.env_remove("RUSTC_WRAPPER");
    command.args(prefix_args);
    command.args(extra_args);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }

    let status = command
        .status()
        .context("failed to run cargo install/update")?;
    if !status.success() {
        bail!("cargo install failed with status {status}");
    }
    Ok(())
}

fn print_install_summary(action: &str, root: &Path) {
    println!(
        "{action} hj, handoff, handon, handover, handup, handoff-db, and handoff-detect into {}",
        root.join("bin").display()
    );
}
