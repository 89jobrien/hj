use std::{
    ffi::OsStr,
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use hj_core::{Handoff, HandoffItem, HandoffState, infer_priority};
use walkdir::WalkDir;

// Re-export detect types so existing downstream code (`hj_git::RepoContext`, etc.) keeps working.
pub use hj_core::detect::{
    HandoffPaths, RefreshReport, RepoContext, branch_name, current_short_head, derive_project_name,
    discover, git_output, is_ignored_dir, today,
};

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
            let handoff: Handoff = match serde_yaml::from_str(&contents) {
                Ok(h) => h,
                Err(e) => {
                    eprintln!(
                        "warning: skipping malformed handoff {}: {e}",
                        path.display()
                    );
                    continue;
                }
            };
            let project_name = handoff
                .project
                .clone()
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| {
                    derive_project_name(&repo_root, &repo_root)
                        .unwrap_or_else(|_| "unknown".into())
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

fn is_handoff_file(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(OsStr::to_str) else {
        return false;
    };

    (name.starts_with("HANDOFF.") && (name.ends_with(".yaml") || name.ends_with(".md")))
        && !name.ends_with(".state.json")
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
    let state_name = name.replace(".yaml", ".state.json");
    let state_path = handoff_path.with_file_name(state_name);
    if !state_path.exists() {
        return Ok((None, None));
    }

    let contents = fs::read_to_string(&state_path)
        .with_context(|| format!("failed to read {}", state_path.display()))?;
    let state: HandoffState = serde_json::from_str(&contents)
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
