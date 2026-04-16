use std::{collections::BTreeSet, path::Path, process::Command};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum TodoStatus {
    Pending,
    InProgress,
    Completed,
    Cancelled,
}

impl TodoStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone)]
pub struct DoobClient {
    cwd: std::path::PathBuf,
}

#[derive(Debug, Clone, Default)]
pub struct ReconcileReport {
    pub project: String,
    pub captured_count: usize,
    pub created_count: usize,
    pub not_captured: Vec<String>,
    pub orphaned: Vec<String>,
    pub closed_upstream: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct TodoList {
    #[serde(default)]
    todos: Vec<Todo>,
}

#[derive(Debug, Deserialize)]
struct Todo {
    #[serde(default)]
    content: String,
}

impl DoobClient {
    pub fn new(cwd: impl Into<std::path::PathBuf>) -> Self {
        Self { cwd: cwd.into() }
    }

    pub fn list_titles(&self, project: &str, status: TodoStatus) -> Result<Vec<String>> {
        let output = Command::new("doob")
            .args([
                "todo",
                "list",
                "-p",
                project,
                "--status",
                status.as_str(),
                "--json",
            ])
            .current_dir(&self.cwd)
            .output()
            .context("failed to run doob todo list")?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let parsed: TodoList = serde_json::from_slice(&output.stdout)
            .context("failed to parse doob todo list output")?;
        Ok(parsed
            .todos
            .into_iter()
            .map(|todo| todo.content)
            .filter(|content| !content.is_empty())
            .collect())
    }

    pub fn add(&self, project: &str, title: &str, priority: u8, tags: &[String]) -> Result<()> {
        let mut command = Command::new("doob");
        command
            .args(["todo", "add", title, "--priority"])
            .arg(priority.to_string())
            .args(["-p", project]);
        if !tags.is_empty() {
            command.args(["-t", &tags.join(",")]);
        }

        let status = command
            .current_dir(&self.cwd)
            .status()
            .context("failed to run doob todo add")?;
        if !status.success() {
            bail!("doob todo add failed for `{title}`");
        }
        Ok(())
    }
}

pub fn map_priority(priority: Option<&str>) -> u8 {
    match priority {
        Some("P0") => 5,
        Some("P1") => 4,
        Some("P2") => 3,
        _ => 1,
    }
}

pub fn unique_titles<I>(titles: I) -> Vec<String>
where
    I: IntoIterator<Item = String>,
{
    let mut set = BTreeSet::new();
    for title in titles {
        if !title.is_empty() {
            set.insert(title);
        }
    }
    set.into_iter().collect()
}

pub fn ensure_doob_on_path(cwd: &Path) -> Result<()> {
    ensure_command("doob", cwd)
}

fn ensure_command(program: &str, cwd: &Path) -> Result<()> {
    let output = Command::new("sh")
        .args(["-c", &format!("command -v {program}")])
        .current_dir(cwd)
        .output()
        .with_context(|| format!("failed to probe {program}"))?;
    if !output.status.success() {
        bail!("{program} not on PATH");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{map_priority, unique_titles};

    #[test]
    fn priority_mapping_matches_shell_script() {
        assert_eq!(map_priority(Some("P0")), 5);
        assert_eq!(map_priority(Some("P1")), 4);
        assert_eq!(map_priority(Some("P2")), 3);
        assert_eq!(map_priority(Some("other")), 1);
    }

    #[test]
    fn deduplicates_titles() {
        let values = unique_titles(vec![
            "A".to_string(),
            "B".to_string(),
            "A".to_string(),
            String::new(),
        ]);
        assert_eq!(values, vec!["A".to_string(), "B".to_string()]);
    }
}
