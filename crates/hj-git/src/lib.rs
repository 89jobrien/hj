use std::{
    ffi::OsStr,
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, anyhow, bail};
use hj_core::{Handoff, HandoffItem, HandoffState, infer_priority, sanitize_name};
use walkdir::WalkDir;

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
    pub project: String,
    pub base_name: String,
}

#[derive(Debug, Clone)]
pub struct RefreshReport {
    pub ctx_dir: PathBuf,
    pub packages: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SurveyHandoff {
    pub path: PathBuf,
    pub repo_root: PathBuf,
    pub project_name: String,
    pub branch: Option<String>,
    pub build: Option<String>,
    pub tests: Option<String>,
    pub items: Vec<HandoffItem>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TodoMarker {
    pub path: PathBuf,
    pub line: usize,
    pub text: String,
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
        let state_path = ctx_dir.join(format!("HANDOFF.{project}.{}.state.yaml", self.base_name));
        let rendered_path = ctx_dir.join("HANDOFF.md");

        Ok(HandoffPaths {
            repo_root: self.repo_root.clone(),
            ctx_dir,
            handoff_path,
            state_path,
            rendered_path,
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
            let state_path = ctx_dir.join(format!("HANDOFF.{pkg}.{}.state.yaml", self.base_name));
            if !state_path.exists() {
                let state = HandoffState {
                    updated: Some(today.clone()),
                    branch: Some(branch.clone()),
                    build: Some("unknown".to_string()),
                    tests: Some("unknown".to_string()),
                    notes: None,
                    touched_files: Vec::new(),
                    extra: Default::default(),
                };
                fs::write(&state_path, serde_yaml::to_string(&state)?)
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

    pub fn working_tree_files(&self) -> Result<Vec<String>> {
        let output = git_output(
            &self.repo_root,
            ["status", "--short", "--untracked-files=all"],
        )?;
        let mut files = Vec::new();
        for line in output.lines() {
            if line.len() < 4 {
                continue;
            }
            let raw = line[3..].trim();
            if raw.is_empty() {
                continue;
            }
            let file = raw
                .split(" -> ")
                .last()
                .map(str::trim)
                .unwrap_or(raw)
                .to_string();
            if !files.iter().any(|existing| existing == &file) {
                files.push(file);
            }
        }
        Ok(files)
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

pub fn discover_handoffs(base: &Path, max_depth: usize) -> Result<Vec<SurveyHandoff>> {
    let base = fs::canonicalize(base)
        .with_context(|| format!("failed to canonicalize {}", base.display()))?;
    let mut results = Vec::new();

    for entry in WalkDir::new(&base)
        .max_depth(max_depth)
        .into_iter()
        .filter_entry(|entry| !is_ignored_dir(entry.path()))
    {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        if !is_handoff_file(path) {
            continue;
        }

        let Some(repo_root) = repo_root_for(path.parent().unwrap_or(&base)) else {
            continue;
        };

        let branch = branch_name(&repo_root)
            .ok()
            .filter(|value| !value.is_empty());

        if path.extension().and_then(OsStr::to_str) == Some("yaml") {
            let contents = fs::read_to_string(path)
                .with_context(|| format!("failed to read {}", path.display()))?;
            let handoff: Handoff = serde_yaml::from_str(&contents)
                .with_context(|| format!("failed to parse {}", path.display()))?;
            let project_name = handoff
                .project
                .clone()
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| {
                    derive_project_name(&repo_root, &repo_root).unwrap_or_else(|_| "unknown".into())
                });
            let items = handoff
                .items
                .into_iter()
                .filter(|item| item.is_open_or_blocked())
                .collect::<Vec<_>>();

            let (build, tests) = read_state_fields(path)?;
            results.push(SurveyHandoff {
                path: path.to_path_buf(),
                repo_root,
                project_name,
                branch,
                build,
                tests,
                items,
            });
            continue;
        }

        let items = parse_markdown_handoff(path)?;
        let project_name = derive_project_name(&repo_root, &repo_root)?;
        results.push(SurveyHandoff {
            path: path.to_path_buf(),
            repo_root,
            project_name,
            branch,
            build: None,
            tests: None,
            items,
        });
    }

    results.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(results)
}

pub fn discover_todo_markers(base: &Path, max_depth: usize) -> Result<Vec<TodoMarker>> {
    let base = fs::canonicalize(base)
        .with_context(|| format!("failed to canonicalize {}", base.display()))?;
    let mut markers = Vec::new();

    for entry in WalkDir::new(&base)
        .max_depth(max_depth)
        .into_iter()
        .filter_entry(|entry| !is_ignored_dir(entry.path()))
    {
        let entry = entry?;
        if !entry.file_type().is_file() || !is_marker_file(entry.path()) {
            continue;
        }

        let file = fs::File::open(entry.path())
            .with_context(|| format!("failed to read {}", entry.path().display()))?;
        for (idx, line) in BufReader::new(file).lines().enumerate() {
            let line = line?;
            if let Some(marker) = extract_marker(&line) {
                markers.push(TodoMarker {
                    path: entry.path().to_path_buf(),
                    line: idx + 1,
                    text: marker.to_string(),
                });
            }
        }
    }

    Ok(markers)
}

fn derive_project_name(cwd: &Path, repo_root: &Path) -> Result<String> {
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

fn manifest_name(dir: &Path) -> Result<Option<String>> {
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

fn scan_package_names(repo_root: &Path) -> Result<Vec<String>> {
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
            if !packages.iter().any(|existing| existing == &name) {
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

fn is_ignored_dir(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(OsStr::to_str),
        Some(".git" | "target" | "vendor" | "__pycache__")
    )
}

fn is_handoff_file(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(OsStr::to_str) else {
        return false;
    };

    (name.starts_with("HANDOFF.") && (name.ends_with(".yaml") || name.ends_with(".md")))
        && !name.ends_with(".state.yaml")
}

fn is_marker_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(OsStr::to_str),
        Some("rs" | "sh" | "py" | "toml")
    )
}

fn extract_marker(line: &str) -> Option<&str> {
    ["TODO:", "FIXME:", "HACK:", "XXX:"]
        .into_iter()
        .find(|needle| line.contains(needle))
}

fn read_state_fields(handoff_path: &Path) -> Result<(Option<String>, Option<String>)> {
    let Some(name) = handoff_path.file_name().and_then(OsStr::to_str) else {
        return Ok((None, None));
    };
    let state_name = name.replace(".yaml", ".state.yaml");
    let state_path = handoff_path.with_file_name(state_name);
    if !state_path.exists() {
        return Ok((None, None));
    }

    let contents = fs::read_to_string(&state_path)
        .with_context(|| format!("failed to read {}", state_path.display()))?;
    let state: HandoffState = serde_yaml::from_str(&contents)
        .with_context(|| format!("failed to parse {}", state_path.display()))?;
    Ok((state.build, state.tests))
}

fn repo_root_for(dir: &Path) -> Option<PathBuf> {
    git_output(dir, ["rev-parse", "--show-toplevel"])
        .ok()
        .map(|value| PathBuf::from(value.trim()))
}

fn parse_markdown_handoff(path: &Path) -> Result<Vec<HandoffItem>> {
    let contents =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let mut items = Vec::new();
    let mut in_section = false;

    for line in contents.lines() {
        let trimmed = line.trim();
        let normalized = trimmed.trim_start_matches('#').trim().to_ascii_lowercase();
        if matches!(
            normalized.as_str(),
            "known gaps" | "next up" | "parked" | "remaining work"
        ) {
            in_section = true;
            continue;
        }
        if trimmed.starts_with('#') {
            in_section = false;
            continue;
        }
        if !in_section {
            continue;
        }
        let bullet = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
            .or_else(|| trimmed.strip_prefix("1. "));
        let Some(title) = bullet else {
            continue;
        };
        let priority = infer_priority(title, None);
        items.push(HandoffItem {
            id: format!("md-{}", items.len() + 1),
            priority: Some(priority),
            status: Some("open".into()),
            title: title.to_string(),
            ..HandoffItem::default()
        });
    }

    Ok(items)
}

fn find_root_handoff(repo_root: &Path) -> Result<Option<PathBuf>> {
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

fn write_gitignore_block(repo_root: &Path) -> Result<()> {
    let gitignore_path = repo_root.join(".gitignore");
    let existing = fs::read_to_string(&gitignore_path).unwrap_or_default();
    let block = [
        "# handoff-begin",
        ".ctx/*",
        "!.ctx/HANDOFF.*.yaml",
        ".ctx/HANDOFF.*.state.yaml",
        "!.ctx/handoff.*.config.toml.example",
        ".ctx/HANDOFF.*.*.state.yaml",
        ".ctx/HANDOFF.hj.hj.state.yaml",
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

fn git_output<I, S>(cwd: &Path, args: I) -> Result<String>
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

    use super::{manifest_name, write_gitignore_block};

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
        assert!(updated.contains(".ctx/HANDOFF.*.*.state.yaml"));
        assert!(updated.contains(".ctx/HANDOFF.hj.hj.state.yaml"));
        assert!(updated.contains("target/"));
        assert!(updated.contains("node_modules/"));
        assert!(!updated.contains("\nold\n"));
    }
}
