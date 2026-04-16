use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Handoff {
    #[serde(default)]
    pub project: Option<String>,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub updated: Option<String>,
    #[serde(default)]
    pub items: Vec<HandoffItem>,
    #[serde(default)]
    pub log: Vec<LogEntry>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_yaml::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HandoffItem {
    pub id: String,
    #[serde(default)]
    pub doob_uuid: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub priority: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default)]
    pub completed: Option<String>,
    #[serde(default)]
    pub extra: Vec<ExtraEntry>,
    #[serde(flatten)]
    pub extra_fields: BTreeMap<String, serde_yaml::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExtraEntry {
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default)]
    pub r#type: Option<String>,
    #[serde(default)]
    pub field: Option<String>,
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub reviewed: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(flatten)]
    pub extra_fields: BTreeMap<String, serde_yaml::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LogEntry {
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub commits: Vec<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_yaml::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HandoffState {
    #[serde(default)]
    pub updated: Option<String>,
    #[serde(default)]
    pub branch: Option<String>,
    #[serde(default)]
    pub build: Option<String>,
    #[serde(default)]
    pub tests: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub touched_files: Vec<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_yaml::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, Eq, PartialEq)]
pub struct HandupReport {
    pub generated: String,
    pub cwd: String,
    #[serde(default)]
    pub projects: Vec<HandupProject>,
    pub recommendation: HandupRecommendation,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, Eq, PartialEq)]
pub struct HandupProject {
    pub name: String,
    pub path: String,
    pub repo_root: String,
    pub handoff_path: Option<String>,
    pub branch: Option<String>,
    pub build: Option<String>,
    pub tests: Option<String>,
    #[serde(default)]
    pub items: Vec<HandupItem>,
    #[serde(default)]
    pub todos: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, Eq, PartialEq)]
pub struct HandupItem {
    pub id: String,
    pub priority: String,
    pub status: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, Eq, PartialEq)]
pub struct HandupRecommendation {
    pub project: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ReconcileMode {
    Sync,
    Audit,
}

#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct ReconcileReport {
    pub project: String,
    pub captured_count: usize,
    pub created_count: usize,
    pub not_captured: Vec<String>,
    pub orphaned: Vec<String>,
    pub closed_upstream: Vec<String>,
}

#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct TodoSnapshot {
    pub active_titles: Vec<String>,
    pub closed_titles: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ReconcileCreate {
    pub title: String,
    pub priority: Option<String>,
}

#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct ReconcilePlan {
    pub creates: Vec<ReconcileCreate>,
    pub report: ReconcileReport,
}

impl Handoff {
    pub fn active_items(&self) -> impl Iterator<Item = &HandoffItem> {
        self.items.iter().filter(|item| item.is_open_or_blocked())
    }

    pub fn ensure_project(&mut self, project: &str) {
        if self.project.as_deref().unwrap_or_default().is_empty() {
            self.project = Some(project.to_string());
        }
    }

    pub fn ensure_id_prefix(&mut self, project: &str) {
        if self.id.as_deref().unwrap_or_default().is_empty() {
            self.id = Some(default_id_prefix(project));
        }
    }
}

impl HandoffItem {
    pub fn is_open_or_blocked(&self) -> bool {
        matches!(self.status.as_deref(), Some("open" | "blocked"))
    }

    pub fn todo_title(&self) -> String {
        let base = self
            .name
            .as_deref()
            .filter(|value| !value.is_empty() && *value != "null")
            .map(titleize_slug)
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| self.title.clone());

        if self.status.as_deref() == Some("blocked") {
            format!("{base} [BLOCKED]")
        } else {
            base
        }
    }

    pub fn doob_title(&self) -> String {
        self.todo_title()
    }

    pub fn title_variants(&self) -> Vec<String> {
        let mut variants = Vec::new();
        let title = self.title.clone();
        let blocked_title = format!("{title} [BLOCKED]");
        let todo_title = self.todo_title();
        let blocked_todo_title = if todo_title.ends_with(" [BLOCKED]") {
            todo_title.clone()
        } else {
            format!("{todo_title} [BLOCKED]")
        };

        for value in [title, blocked_title, todo_title, blocked_todo_title] {
            if !value.is_empty() && !variants.iter().any(|existing| existing == &value) {
                variants.push(value);
            }
        }

        variants
    }

    pub fn inferred_priority(&self) -> String {
        self.priority
            .clone()
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| infer_priority(self.title.as_str(), self.description.as_deref()))
    }
}

pub fn sanitize_name(raw: &str) -> String {
    raw.trim().to_ascii_lowercase().replace([' ', '/'], "-")
}

pub fn default_id_prefix(project: &str) -> String {
    let cleaned = sanitize_name(project);
    cleaned.chars().take(7).collect()
}

pub fn titleize_slug(slug: &str) -> String {
    slug.split('-')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => {
                    let mut word = first.to_uppercase().collect::<String>();
                    word.push_str(chars.as_str());
                    word
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn infer_priority(title: &str, description: Option<&str>) -> String {
    let title = title.to_ascii_lowercase();
    let description = description.unwrap_or_default().to_ascii_lowercase();
    let combined = format!("{title} {description}");

    if [
        "broken",
        "fails",
        "segfault",
        "panic",
        "security",
        "blocked",
        "urgent",
        "can't deploy",
    ]
    .iter()
    .any(|needle| combined.contains(needle))
    {
        return "P0".to_string();
    }

    if [
        "fix",
        "implement",
        "refactor",
        "wire",
        "small change",
        "known fix",
    ]
    .iter()
    .any(|needle| combined.contains(needle))
    {
        return "P1".to_string();
    }

    "P2".to_string()
}

pub fn build_reconcile_plan(
    project: &str,
    handoff: &Handoff,
    snapshot: &TodoSnapshot,
    mode: ReconcileMode,
) -> ReconcilePlan {
    let mut captured_count = 0usize;
    let mut created_count = 0usize;
    let mut not_captured = Vec::new();
    let mut closed_upstream = Vec::new();
    let mut creates = Vec::new();
    let mut handoff_titles = std::collections::BTreeSet::new();

    for item in handoff.active_items() {
        for variant in item.title_variants() {
            handoff_titles.insert(variant);
        }

        if contains_any(&snapshot.active_titles, item) {
            captured_count += 1;
            continue;
        }

        if contains_any(&snapshot.closed_titles, item) {
            closed_upstream.push(item.todo_title());
            continue;
        }

        match mode {
            ReconcileMode::Sync => {
                creates.push(ReconcileCreate {
                    title: item.todo_title(),
                    priority: item.priority.clone(),
                });
                captured_count += 1;
                created_count += 1;
            }
            ReconcileMode::Audit => not_captured.push(item.todo_title()),
        }
    }

    let orphaned = snapshot
        .active_titles
        .iter()
        .filter(|title| !handoff_titles.contains(*title))
        .cloned()
        .collect::<Vec<_>>();

    ReconcilePlan {
        creates,
        report: ReconcileReport {
            project: project.to_string(),
            captured_count,
            created_count,
            not_captured,
            orphaned,
            closed_upstream,
        },
    }
}

fn contains_any(existing: &[String], item: &HandoffItem) -> bool {
    item.title_variants()
        .into_iter()
        .any(|variant| existing.iter().any(|title| title == &variant))
}

#[cfg(test)]
mod tests {
    use super::{
        Handoff, HandoffItem, HandoffState, ReconcileMode, TodoSnapshot, build_reconcile_plan,
        default_id_prefix, infer_priority, sanitize_name, titleize_slug,
    };

    #[test]
    fn sanitize_project_name() {
        assert_eq!(sanitize_name("My Project/CLI"), "my-project-cli");
    }

    #[test]
    fn default_prefix_uses_first_seven_chars() {
        assert_eq!(default_id_prefix("atelier"), "atelier");
        assert_eq!(default_id_prefix("sanctum"), "sanctum");
    }

    #[test]
    fn doob_title_prefers_slug() {
        let item = HandoffItem {
            id: "x-1".into(),
            name: Some("wire-render-pass".into()),
            status: Some("blocked".into()),
            title: "ignored".into(),
            ..HandoffItem::default()
        };

        assert_eq!(titleize_slug("wire-render-pass"), "Wire Render Pass");
        assert_eq!(item.doob_title(), "Wire Render Pass [BLOCKED]");
    }

    #[test]
    fn infer_priority_uses_signal_words() {
        assert_eq!(infer_priority("CI broken", None), "P0");
        assert_eq!(infer_priority("Implement handup parity", None), "P1");
        assert_eq!(infer_priority("Explore someday", None), "P2");
    }

    #[test]
    fn reconcile_plan_is_backend_agnostic() {
        let handoff = Handoff {
            project: Some("hj".into()),
            items: vec![
                HandoffItem {
                    id: "hj-1".into(),
                    priority: Some("P1".into()),
                    status: Some("open".into()),
                    title: "Already tracked".into(),
                    ..HandoffItem::default()
                },
                HandoffItem {
                    id: "hj-2".into(),
                    priority: Some("P2".into()),
                    status: Some("open".into()),
                    title: "Needs create".into(),
                    ..HandoffItem::default()
                },
                HandoffItem {
                    id: "hj-3".into(),
                    priority: Some("P1".into()),
                    status: Some("blocked".into()),
                    title: "Closed upstream".into(),
                    ..HandoffItem::default()
                },
            ],
            ..Handoff::default()
        };
        let snapshot = TodoSnapshot {
            active_titles: vec!["Already tracked".into(), "Orphaned task".into()],
            closed_titles: vec!["Closed upstream [BLOCKED]".into()],
        };

        let audit = build_reconcile_plan("hj", &handoff, &snapshot, ReconcileMode::Audit);
        assert_eq!(audit.creates.len(), 0);
        assert_eq!(audit.report.captured_count, 1);
        assert_eq!(audit.report.not_captured, vec!["Needs create".to_string()]);
        assert_eq!(
            audit.report.closed_upstream,
            vec!["Closed upstream [BLOCKED]".to_string()]
        );
        assert_eq!(audit.report.orphaned, vec!["Orphaned task".to_string()]);

        let sync = build_reconcile_plan("hj", &handoff, &snapshot, ReconcileMode::Sync);
        assert_eq!(sync.creates.len(), 1);
        assert_eq!(sync.creates[0].title, "Needs create");
        assert_eq!(sync.creates[0].priority.as_deref(), Some("P2"));
        assert_eq!(sync.report.captured_count, 2);
        assert_eq!(sync.report.created_count, 1);
        assert!(sync.report.not_captured.is_empty());
    }

    #[test]
    fn state_omits_empty_touched_files() {
        let state = HandoffState {
            branch: Some("main".into()),
            build: Some("clean".into()),
            tests: Some("passing".into()),
            ..HandoffState::default()
        };

        let rendered = serde_yaml::to_string(&state).expect("serialize state");
        assert!(!rendered.contains("touched_files"));
    }
}
