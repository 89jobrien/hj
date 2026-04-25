use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, anyhow, bail};
use walkdir::WalkDir;

use crate::{HandoffState, sanitize_name};

#[derive(Debug, Clone)]
pub struct RepoContext {
    pub repo_root: PathBuf,
    pub cwd: PathBuf,
    pub base_name: String,
}

#[derive(Debug, Clone)]
pub struct HandoffPaths {
    pub repo_root: PathBuf,
    pub ctx_dir: PathBuf,
    pub handoff_path: PathBuf,
    pub state_path: PathBuf,
    pub rendered_path: PathBuf,
    pub handover_path: PathBuf,
    pub project: String,
    pub base_name: String,
}

#[derive(Debug, Clone)]
pub struct RefreshReport {
    pub ctx_dir: PathBuf,
    pub packages: Vec<String>,
}

pub fn discover(cwd: &Path) -> Result<RepoContext> {
    let repo_root =
        git_output(cwd, ["rev-parse", "--show-toplevel"]).context("not in a git repository")?;
    let repo_root = PathBuf::from(repo_root.trim());
    let cwd = fs::canonicalize(cwd).context("failed to canonicalize current directory")?;
    let base_name = repo_root
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or_else(|| anyhow!("repo root has no basename"))?
        .to_string();

    Ok(RepoContext {
        repo_root,
        cwd,
        base_name,
    })
}

impl RepoContext {
    pub fn project_name(&self) -> Result<String> {
        derive_project_name(&self.cwd, &self.repo_root)
    }

    pub fn paths(&self, explicit_project: Option<&str>) -> Result<HandoffPaths> {
        let project = explicit_project
            .map(ToOwned::to_owned)
            .unwrap_or(self.project_name()?);
        let project = sanitize_name(&project);
        let ctx_dir = self.repo_root.join(".ctx");
        let handoff_path = ctx_dir.join(format!("HANDOFF.{project}.{}.yaml", self.base_name));
        let state_path = ctx_dir.join(format!("HANDOFF.{project}.{}.state.json", self.base_name));
        let rendered_path = ctx_dir.join("HANDOFF.md");
        let handover_path = ctx_dir.join("HANDOVER.md");

        Ok(HandoffPaths {
            repo_root: self.repo_root.clone(),
            ctx_dir,
            handoff_path,
            state_path,
            rendered_path,
            handover_path,
            project,
            base_name: self.base_name.clone(),
        })
    }

    pub fn refresh(&self, force: bool) -> Result<RefreshReport> {
        let ctx_dir = self.repo_root.join(".ctx");
        let token = ctx_dir.join(".initialized");
        if token.exists() && !force {
            return Ok(RefreshReport {
                ctx_dir,
                packages: scan_package_names(&self.repo_root)?,
            });
        }

        fs::create_dir_all(&ctx_dir).context("failed to create .ctx directory")?;
        let today = today(&self.repo_root)?;
        let branch = branch_name(&self.repo_root).unwrap_or_else(|_| "unknown".to_string());
        let packages = scan_package_names(&self.repo_root)?;

        for pkg in &packages {
            let state_path = ctx_dir.join(format!("HANDOFF.{pkg}.{}.state.json", self.base_name));
            if !state_path.exists() {
                let state = HandoffState {
                    updated: Some(today.clone()),
                    branch: Some(branch.clone()),
                    build: Some("unknown".to_string()),
                    tests: Some("unknown".to_string()),
                    notes: None,
                    touched_files: Vec::new(),
                    last_log: None,
                    extra: Default::default(),
                };
                fs::write(&state_path, serde_json::to_string_pretty(&state)?)
                    .with_context(|| format!("failed to write {}", state_path.display()))?;
            }
        }

        write_gitignore_block(&self.repo_root)?;
        fs::write(&token, format!("{today}\n"))
            .with_context(|| format!("failed to write {}", token.display()))?;

        Ok(RefreshReport { ctx_dir, packages })
    }

    pub fn migrate_root_handoff(&self, target: &Path) -> Result<Option<PathBuf>> {
        let old = find_root_handoff(&self.repo_root)?;
        let Some(old) = old else {
            return Ok(None);
        };

        let parent = target
            .parent()
            .ok_or_else(|| anyhow!("target handoff has no parent directory"))?;
        fs::create_dir_all(parent)?;

        let status = Command::new("git")
            .arg("-C")
            .arg(&self.repo_root)
            .arg("mv")
            .arg(&old)
            .arg(target)
            .status();

        match status {
            Ok(result) if result.success() => Ok(Some(target.to_path_buf())),
            _ => {
                fs::rename(&old, target).with_context(|| {
                    format!(
                        "failed to move legacy handoff {} -> {}",
                        old.display(),
                        target.display()
                    )
                })?;
                Ok(Some(target.to_path_buf()))
            }
        }
    }
}

pub fn branch_name(repo_root: &Path) -> Result<String> {
    Ok(git_output(repo_root, ["branch", "--show-current"])?
        .trim()
        .to_string())
}

pub fn current_short_head(repo_root: &Path) -> Result<String> {
    Ok(git_output(repo_root, ["rev-parse", "--short", "HEAD"])?
        .trim()
        .to_string())
}

pub fn today(cwd: &Path) -> Result<String> {
    Ok(command_output("date", cwd, ["+%Y-%m-%d"])?
        .trim()
        .to_string())
}

pub fn derive_project_name(cwd: &Path, repo_root: &Path) -> Result<String> {
    if let Some(name) = manifest_name(cwd)? {
        return Ok(sanitize_name(&name));
    }
    if let Some(name) = manifest_name(repo_root)? {
        return Ok(sanitize_name(&name));
    }

    let name = cwd
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or_else(|| anyhow!("current directory has no basename"))?;
    Ok(sanitize_name(name))
}

pub fn manifest_name(dir: &Path) -> Result<Option<String>> {
    let cargo = dir.join("Cargo.toml");
    if cargo.exists() {
        let contents = fs::read_to_string(&cargo)
            .with_context(|| format!("failed to read {}", cargo.display()))?;
        let manifest: toml::Value = toml::from_str(&contents)
            .with_context(|| format!("failed to parse {}", cargo.display()))?;
        if let Some(name) = manifest
            .get("package")
            .and_then(|value| value.get("name"))
            .and_then(toml::Value::as_str)
        {
            return Ok(Some(name.to_string()));
        }
    }

    let pyproject = dir.join("pyproject.toml");
    if pyproject.exists() {
        let contents = fs::read_to_string(&pyproject)
            .with_context(|| format!("failed to read {}", pyproject.display()))?;
        let manifest: toml::Value = toml::from_str(&contents)
            .with_context(|| format!("failed to parse {}", pyproject.display()))?;
        let project_name = manifest
            .get("project")
            .and_then(|value| value.get("name"))
            .and_then(toml::Value::as_str)
            .or_else(|| {
                manifest
                    .get("tool")
                    .and_then(|value| value.get("poetry"))
                    .and_then(|value| value.get("name"))
                    .and_then(toml::Value::as_str)
            });
        if let Some(name) = project_name {
            return Ok(Some(name.to_string()));
        }
    }

    let go_mod = dir.join("go.mod");
    if go_mod.exists() {
        let contents = fs::read_to_string(&go_mod)
            .with_context(|| format!("failed to read {}", go_mod.display()))?;
        for line in contents.lines() {
            if let Some(module) = line.strip_prefix("module ") {
                let name = module
                    .split('/')
                    .next_back()
                    .unwrap_or(module)
                    .trim()
                    .to_string();
                if !name.is_empty() {
                    return Ok(Some(name));
                }
            }
        }
    }

    Ok(None)
}

pub fn scan_package_names(repo_root: &Path) -> Result<Vec<String>> {
    let mut packages = Vec::new();

    for entry in WalkDir::new(repo_root)
        .into_iter()
        .filter_entry(|entry| !is_ignored_dir(entry.path()))
    {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }

        let Some(file_name) = entry.file_name().to_str() else {
            continue;
        };

        let manifest_dir = entry.path().parent().unwrap_or(repo_root);
        let maybe_name = match file_name {
            "Cargo.toml" | "pyproject.toml" | "go.mod" => manifest_name(manifest_dir)?,
            _ => None,
        };

        if let Some(name) = maybe_name {
            let name = sanitize_name(&name);
            if !packages.iter().any(|existing: &String| existing == &name) {
                packages.push(name);
            }
        }
    }

    if packages.is_empty() {
        packages.push(
            repo_root
                .file_name()
                .and_then(OsStr::to_str)
                .map(sanitize_name)
                .ok_or_else(|| anyhow!("repo root has no basename"))?,
        );
    }

    packages.sort();
    Ok(packages)
}

pub fn find_root_handoff(repo_root: &Path) -> Result<Option<PathBuf>> {
    let mut matches = Vec::new();
    for entry in fs::read_dir(repo_root)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(OsStr::to_str) else {
            continue;
        };
        if name.starts_with("HANDOFF.") && name.ends_with(".yaml") {
            matches.push(path);
        }
    }
    matches.sort();
    Ok(matches.into_iter().next())
}

pub fn write_gitignore_block(repo_root: &Path) -> Result<()> {
    let gitignore_path = repo_root.join(".gitignore");
    let existing = fs::read_to_string(&gitignore_path).unwrap_or_default();
    let block = [
        "# handoff-begin",
        ".ctx/*",
        "!.ctx/HANDOFF.*.yaml",
        ".ctx/HANDOFF.*.state.json",
        "!.ctx/handoff.*.config.toml.example",
        ".ctx/HANDOFF.*.*.state.json",
        ".ctx/HANDOFF.hj.hj.state.json",
        ".ctx/.initialized",
        "# handoff-end",
    ];

    let mut output = Vec::new();
    let mut in_block = false;
    let mut replaced = false;

    for line in existing.lines() {
        match line {
            "# handoff-begin" => {
                if !replaced {
                    output.extend(block.iter().map(|value| (*value).to_string()));
                    replaced = true;
                }
                in_block = true;
            }
            "# handoff-end" => {
                in_block = false;
            }
            _ if !in_block => output.push(line.to_string()),
            _ => {}
        }
    }

    if !replaced {
        if !output.is_empty() && output.last().is_some_and(|line| !line.is_empty()) {
            output.push(String::new());
        }
        output.extend(block.iter().map(|value| (*value).to_string()));
    }

    fs::write(&gitignore_path, output.join("\n") + "\n")
        .with_context(|| format!("failed to write {}", gitignore_path.display()))?;
    Ok(())
}

pub fn is_ignored_dir(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(OsStr::to_str),
        Some(".git" | "target" | "vendor" | "__pycache__" | "worktrees" | ".tmp-dogfood" | "examples")
    )
}

pub fn git_output<I, S>(cwd: &Path, args: I) -> Result<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    command_output("git", cwd, args)
}

fn command_output<I, S>(program: &str, cwd: &Path, args: I) -> Result<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .output()
        .with_context(|| format!("failed to run {program}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("{program} failed: {}", stderr.trim());
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn parses_package_name_from_cargo_manifest() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"hj-cli\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();

        assert_eq!(
            manifest_name(dir.path()).unwrap().as_deref(),
            Some("hj-cli")
        );
    }

    #[test]
    fn rewrites_managed_gitignore_block() {
        let dir = tempfile::tempdir().unwrap();
        let gitignore = dir.path().join(".gitignore");
        fs::write(
            &gitignore,
            "target/\n# handoff-begin\nold\n# handoff-end\nnode_modules/\n",
        )
        .unwrap();

        write_gitignore_block(dir.path()).unwrap();
        let updated = fs::read_to_string(gitignore).unwrap();

        assert!(updated.contains(".ctx/*"));
        assert!(updated.contains(".ctx/HANDOFF.*.*.state.json"));
        assert!(updated.contains(".ctx/HANDOFF.hj.hj.state.json"));
        assert!(updated.contains("target/"));
        assert!(updated.contains("node_modules/"));
        assert!(!updated.contains("\nold\n"));
    }

    #[test]
    fn derive_project_name_falls_back_to_dir_basename() {
        let dir = tempfile::tempdir().unwrap();
        // No manifest files present, so it should fall back to the directory basename
        let result = derive_project_name(dir.path(), dir.path()).unwrap();
        let expected = dir
            .path()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_ascii_lowercase()
            .replace([' ', '/'], "-");
        assert_eq!(result, expected);
    }

    #[test]
    fn find_root_handoff_finds_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("HANDOFF.test.repo.yaml"), "project: test\n").unwrap();
        fs::write(dir.path().join("README.md"), "# hello\n").unwrap();

        let found = find_root_handoff(dir.path()).unwrap();
        assert!(found.is_some());
        let path = found.unwrap();
        assert!(path.file_name().unwrap().to_str().unwrap().starts_with("HANDOFF."));
    }
}
