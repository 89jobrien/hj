use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow};
use hj_core::{HandupItem, HandupProject, HandupRecommendation, HandupReport};
use hj_core::{discover, today};
use hj_git::{SurveyHandoff, TodoMarker, discover_handoffs, discover_todo_markers};
use hj_sqlite::{HandupCheckpoint, HandupDb};

use crate::cli::HandupArgs;

pub(crate) fn handup(args: HandupArgs) -> Result<()> {
    let cwd =
        fs::canonicalize(Path::new(".")).context("failed to canonicalize current directory")?;
    let generated = today(&cwd)?;
    let basename = cwd
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| anyhow!("current directory has no basename"))?;

    let surveys = discover_handoffs(&cwd, args.max_depth)?;
    let markers = discover_todo_markers(&cwd, args.max_depth)?;
    let report = build_handup_report(&cwd, &generated, surveys, markers);

    let handup_dir = handup_output_dir()?;
    fs::create_dir_all(&handup_dir)
        .with_context(|| format!("failed to create {}", handup_dir.display()))?;
    let json_path = handup_dir.join("HANDUP.json");
    fs::write(&json_path, serde_json::to_string_pretty(&report)?)
        .with_context(|| format!("failed to write {}", json_path.display()))?;

    let db = HandupDb::new()?;
    let db_path = db.checkpoint(&HandupCheckpoint {
        project: basename.to_string(),
        cwd: cwd.display().to_string(),
        generated: generated.clone(),
        recommendation: report.recommendation.reason.clone(),
        json_path: json_path.display().to_string(),
    })?;

    print_handup_summary(&report, &json_path, &db_path);
    Ok(())
}

fn build_handup_report(
    cwd: &Path,
    generated: &str,
    surveys: Vec<SurveyHandoff>,
    markers: Vec<TodoMarker>,
) -> HandupReport {
    let marker_groups = group_markers(markers);
    let mut projects_by_root = BTreeMap::<String, HandupProject>::new();

    for survey in surveys {
        let repo_root = survey.repo_root.display().to_string();
        let todos = marker_groups.get(&repo_root).cloned().unwrap_or_default();
        let entry = projects_by_root
            .entry(repo_root.clone())
            .or_insert_with(|| HandupProject {
                name: survey.project_name.clone(),
                path: repo_root.clone(),
                repo_root: repo_root.clone(),
                handoff_path: Some(survey.path.display().to_string()),
                branch: survey.branch.clone(),
                build: survey.build.clone(),
                tests: survey.tests.clone(),
                items: Vec::new(),
                todos: Vec::new(),
            });

        if entry.handoff_path.is_none() {
            entry.handoff_path = Some(survey.path.display().to_string());
        }
        if entry.branch.is_none() {
            entry.branch = survey.branch.clone();
        }
        if entry.build.is_none() {
            entry.build = survey.build.clone();
        }
        if entry.tests.is_none() {
            entry.tests = survey.tests.clone();
        }
        for item in survey.items {
            let priority = item.inferred_priority();
            let status = item.status.clone().unwrap_or_else(|| "open".into());
            entry.items.push(HandupItem {
                id: item.id,
                priority,
                status,
                title: item.title,
            });
        }
        if entry.items.is_empty() || entry.todos.len() <= 5 {
            entry.todos = todos;
        }
    }

    for (repo_root, todos) in marker_groups {
        projects_by_root
            .entry(repo_root.clone())
            .and_modify(|project| {
                if project.handoff_path.is_none() || todos.len() > 5 {
                    project.todos = todos.clone();
                }
            })
            .or_insert_with(|| {
                let name = Path::new(&repo_root)
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                HandupProject {
                    name,
                    path: repo_root.clone(),
                    repo_root,
                    handoff_path: None,
                    branch: None,
                    build: None,
                    tests: None,
                    items: Vec::new(),
                    todos,
                }
            });
    }

    let mut projects = projects_by_root.into_values().collect::<Vec<_>>();
    projects.sort_by(|left, right| {
        let left_p0 = left
            .items
            .iter()
            .filter(|item| item.priority == "P0")
            .count();
        let right_p0 = right
            .items
            .iter()
            .filter(|item| item.priority == "P0")
            .count();
        let left_p1 = left
            .items
            .iter()
            .filter(|item| item.priority == "P1")
            .count();
        let right_p1 = right
            .items
            .iter()
            .filter(|item| item.priority == "P1")
            .count();

        right_p0
            .cmp(&left_p0)
            .then(right_p1.cmp(&left_p1))
            .then(left.name.cmp(&right.name))
    });

    let recommendation = recommend_project(&projects);
    HandupReport {
        generated: generated.to_string(),
        cwd: cwd.display().to_string(),
        projects,
        recommendation,
    }
}

fn group_markers(markers: Vec<TodoMarker>) -> BTreeMap<String, Vec<String>> {
    let mut grouped = BTreeMap::<String, Vec<String>>::new();
    for marker in markers {
        let repo_root = discover(marker.path.parent().unwrap_or(Path::new(".")))
            .map(|context| context.repo_root.display().to_string())
            .unwrap_or_else(|_| {
                marker
                    .path
                    .parent()
                    .unwrap_or(Path::new("."))
                    .display()
                    .to_string()
            });
        grouped.entry(repo_root).or_default().push(format!(
            "{}:{}  {}",
            marker.path.display(),
            marker.line,
            marker.text
        ));
    }
    grouped
}

fn recommend_project(projects: &[HandupProject]) -> HandupRecommendation {
    let Some(project) = projects.first() else {
        return HandupRecommendation {
            project: None,
            reason: "No open handoff items or TODO markers found.".into(),
        };
    };

    let p0 = project
        .items
        .iter()
        .filter(|item| item.priority == "P0")
        .count();
    let p1 = project
        .items
        .iter()
        .filter(|item| item.priority == "P1")
        .count();
    let reason = if p0 > 0 {
        format!("{p0} P0 item(s) need attention")
    } else if p1 > 0 {
        format!("{p1} P1 item(s) ready for follow-up")
    } else if !project.items.is_empty() {
        format!("{} open item(s) remain in handoff", project.items.len())
    } else if !project.todos.is_empty() {
        format!("{} inline TODO marker(s) found", project.todos.len())
    } else {
        "No open work found.".into()
    };

    HandupRecommendation {
        project: Some(project.name.clone()),
        reason,
    }
}

fn handup_output_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("could not determine home directory"))?;
    let cwd =
        fs::canonicalize(Path::new(".")).context("failed to canonicalize current directory")?;
    let basename = cwd
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| anyhow!("current directory has no basename"))?;
    Ok(home.join(".ctx/handoffs").join(basename))
}

fn print_handup_summary(report: &HandupReport, json_path: &Path, db_path: &Path) {
    if report.projects.is_empty() {
        println!(
            "No open handoff items or TODO markers found under `{}`.",
            report.cwd
        );
        println!("Findings written to: {}", json_path.display());
        println!("Checkpointed to: {}", db_path.display());
        return;
    }

    println!("## handup — {} ({})", report.cwd, report.generated);
    println!();

    for project in &report.projects {
        println!("### {} — {}", project.name, project.path);
        if project.branch.is_some() || project.build.is_some() || project.tests.is_some() {
            println!(
                "Branch: {} | Build: {} | Tests: {}",
                project.branch.as_deref().unwrap_or("unknown"),
                project.build.as_deref().unwrap_or("unknown"),
                project.tests.as_deref().unwrap_or("unknown")
            );
        }

        let mut items = project.items.clone();
        items.sort_by(|left, right| {
            left.priority
                .cmp(&right.priority)
                .then(left.id.cmp(&right.id))
        });
        for item in items {
            println!(
                "  {}  [{}] {} (status: {})",
                item.priority, item.id, item.title, item.status
            );
        }
        for todo in &project.todos {
            println!("  TODO  {todo}");
        }
        println!();
    }

    println!("---");
    println!("## Where to next?");
    match report.recommendation.project.as_deref() {
        Some(project) => {
            println!(
                "Highest urgency: {project} — {}",
                report.recommendation.reason
            );
        }
        None => {
            println!("{}", report.recommendation.reason);
        }
    }
    if let Some(project) = report.projects.first() {
        println!("Suggested: cd {} && /atelier:handon", project.path);
    }
    println!(
        "Findings written to: {} — checkpointed to {}",
        json_path.display(),
        db_path.display()
    );
}
