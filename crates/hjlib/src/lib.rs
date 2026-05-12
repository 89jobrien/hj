pub mod detect;
pub mod doob;
pub mod git;
pub mod render;
pub mod sqlite;

pub use detect::{
    HandoffPaths, RefreshReport, RepoContext, branch_name, current_short_head, discover, today,
};

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

/// A commit reference in a log entry. Accepts both a bare SHA string and the
/// `{sha, branch}` object form. Serializes as `{sha, branch}` when branch is
/// present, or a bare string when branch is absent (round-trips correctly).
///
/// Uses a hand-written `Deserialize` to coerce YAML-number SHA values (e.g.
/// `7516e53` parses as float in YAML 1.2) to their string representation.
#[derive(Debug, Clone, Serialize, Eq, PartialEq)]
pub enum CommitRef {
    /// Plain SHA string: `- abc1234`
    Sha(String),
    /// Object form: `- {sha: abc1234, branch: main}`
    Object {
        sha: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        branch: Option<String>,
    },
}

impl CommitRef {
    pub fn sha(&self) -> &str {
        match self {
            CommitRef::Sha(s) => s,
            CommitRef::Object { sha, .. } => sha,
        }
    }
}

/// Coerce any YAML scalar value to a String, handling the case where a SHA
/// like `7516e53` is parsed by serde_yaml as a float.
fn yaml_value_to_sha(v: &serde_yaml::Value) -> Option<String> {
    match v {
        serde_yaml::Value::String(s) => Some(s.clone()),
        serde_yaml::Value::Number(n) => Some(format!("{n}")),
        serde_yaml::Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

impl<'de> serde::Deserialize<'de> for CommitRef {
    fn deserialize<D: serde::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        let v = serde_yaml::Value::deserialize(de)
            .map_err(|e| serde::de::Error::custom(e.to_string()))?;

        match &v {
            // Bare scalar — treat as plain SHA
            serde_yaml::Value::String(s) => return Ok(CommitRef::Sha(s.clone())),
            serde_yaml::Value::Number(n) => return Ok(CommitRef::Sha(format!("{n}"))),
            _ => {}
        }

        // Mapping — expect {sha: ..., branch?: ...}
        if let serde_yaml::Value::Mapping(ref m) = v {
            let sha_key = serde_yaml::Value::String("sha".into());
            let branch_key = serde_yaml::Value::String("branch".into());

            if let Some(sha_val) = m.get(&sha_key) {
                let sha = yaml_value_to_sha(sha_val).ok_or_else(|| {
                    serde::de::Error::custom(format!(
                        "commit sha is not a scalar: {sha_val:?}"
                    ))
                })?;
                let branch = m.get(&branch_key).and_then(|v| v.as_str()).map(str::to_string);
                return Ok(CommitRef::Object { sha, branch });
            }
        }

        Err(serde::de::Error::custom(format!(
            "expected a SHA string or {{sha, branch}} object, got: {v:?}"
        )))
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ValidationWarning {
    /// An item is missing its `id` field
    ItemMissingId { index: usize, title: String },
    /// An item looks like a log entry (has date/summary/commits but no meaningful item fields)
    LogEntryInItems { index: usize, date: String },
    /// A log entry is missing its summary
    LogEntryMissingSummary { index: usize },
    /// Duplicate item id
    DuplicateItemId { id: String, indices: Vec<usize> },
}

impl fmt::Display for ValidationWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationWarning::ItemMissingId { index, title } => {
                write!(f, "items[{index}]: missing id field (title: '{title}')")
            }
            ValidationWarning::LogEntryInItems { index, date } => {
                write!(f, "items[{index}]: looks like a log entry (date: {date})")
            }
            ValidationWarning::LogEntryMissingSummary { index } => {
                write!(f, "log[{index}]: missing summary")
            }
            ValidationWarning::DuplicateItemId { id, indices } => {
                write!(f, "duplicate item id '{id}' at indices {indices:?}")
            }
        }
    }
}

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issue: Option<u64>,
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

/// A session log entry. Uses a hand-written `Deserialize` impl to work around a
/// serde_yaml v0.9 bug where `#[serde(flatten)]` + `#[serde(untagged)]` on a
/// sibling field causes the deserializer to pass the entire map to the untagged
/// enum deserializer when extra (unknown) fields like `session` are present.
#[derive(Debug, Clone, Serialize, Default)]
pub struct LogEntry {
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub commits: Vec<CommitRef>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_yaml::Value>,
}

impl<'de> serde::Deserialize<'de> for LogEntry {
    fn deserialize<D: serde::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        // Collect the entire YAML mapping into a Value first, then extract fields
        // individually. This avoids the serde_yaml flatten + untagged bug.
        let map = serde_yaml::Mapping::deserialize(de)
            .map_err(|e| serde::de::Error::custom(e.to_string()))?;

        let date = map
            .get("date")
            .and_then(|v| v.as_str())
            .map(str::to_string);

        let summary = map
            .get("summary")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let commits: Vec<CommitRef> = map
            .get("commits")
            .and_then(|v| v.as_sequence())
            .map(|seq| {
                seq.iter()
                    .filter_map(|v| {
                        serde_yaml::from_value::<CommitRef>(v.clone())
                            .map_err(|e| {
                                eprintln!("warning: skipping unrecognised commit entry: {e}");
                            })
                            .ok()
                    })
                    .collect()
            })
            .unwrap_or_default();

        let mut extra = BTreeMap::new();
        for (k, v) in &map {
            if let Some(key) = k.as_str() {
                if key != "date" && key != "summary" && key != "commits" {
                    extra.insert(key.to_string(), v.clone());
                }
            }
        }

        Ok(LogEntry { date, summary, commits, extra })
    }
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_log: Option<String>,
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

    pub fn validate(&self) -> Vec<ValidationWarning> {
        let mut warnings = Vec::new();

        let mut id_positions: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (i, item) in self.items.iter().enumerate() {
            if item.id.is_empty() {
                if Self::looks_like_log_entry(item) {
                    let date = item
                        .extra_fields
                        .get("date")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    warnings.push(ValidationWarning::LogEntryInItems { index: i, date });
                } else {
                    warnings.push(ValidationWarning::ItemMissingId {
                        index: i,
                        title: item.title.clone(),
                    });
                }
            } else {
                id_positions.entry(item.id.clone()).or_default().push(i);
            }
        }

        for (id, indices) in id_positions {
            if indices.len() > 1 {
                warnings.push(ValidationWarning::DuplicateItemId { id, indices });
            }
        }

        for (i, entry) in self.log.iter().enumerate() {
            if entry.summary.is_empty() {
                warnings.push(ValidationWarning::LogEntryMissingSummary { index: i });
            }
        }

        warnings
    }

    pub fn repair(&mut self) -> Vec<String> {
        let mut descriptions = Vec::new();
        let mut kept_items = Vec::new();

        for (i, item) in self.items.drain(..).enumerate() {
            if Self::looks_like_log_entry(&item) {
                let date = item
                    .extra_fields
                    .get("date")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let summary = item
                    .extra_fields
                    .get("summary")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let commits: Vec<CommitRef> = item
                    .extra_fields
                    .get("commits")
                    .and_then(|v| v.as_sequence())
                    .map(|seq| {
                        seq.iter()
                            .filter_map(|v| match v {
                                serde_yaml::Value::String(s) => {
                                    Some(CommitRef::Sha(s.clone()))
                                }
                                serde_yaml::Value::Mapping(m) => {
                                    let sha = m
                                        .get(serde_yaml::Value::String("sha".into()))
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string())?;
                                    let branch = m
                                        .get(serde_yaml::Value::String("branch".into()))
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string());
                                    Some(CommitRef::Object { sha, branch })
                                }
                                _ => None,
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();

                let mut extra = BTreeMap::new();
                for (k, v) in &item.extra_fields {
                    if k != "date" && k != "summary" && k != "commits" {
                        extra.insert(k.clone(), v.clone());
                    }
                }

                let date_str = date.as_deref().unwrap_or("unknown").to_string();
                descriptions.push(format!(
                    "moved log entry (date: {date_str}) from items[{i}] to log"
                ));

                self.log.push(LogEntry {
                    date,
                    summary,
                    commits,
                    extra,
                });
            } else {
                kept_items.push(item);
            }
        }

        self.items = kept_items;

        self.log.sort_by(|a, b| {
            let da = a.date.as_deref().unwrap_or("");
            let db = b.date.as_deref().unwrap_or("");
            db.cmp(da)
        });

        descriptions
    }

    fn looks_like_log_entry(item: &HandoffItem) -> bool {
        if !item.id.is_empty() {
            return false;
        }
        if item.extra_fields.contains_key("date") {
            return true;
        }
        item.extra_fields.contains_key("summary") && item.extra_fields.contains_key("commits")
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
        CommitRef, Handoff, HandoffItem, HandoffState, LogEntry, ReconcileMode, TodoSnapshot,
        ValidationWarning, build_reconcile_plan, default_id_prefix, infer_priority, sanitize_name,
        titleize_slug,
    };
    use std::collections::BTreeMap;

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
    fn log_commits_accept_bare_sha_and_object_form() {
        let yaml = r#"
log:
  - date: "20260422.120000"
    summary: bare sha form
    commits:
      - abc1234
      - def5678
  - date: "20260422.130000"
    summary: object form
    commits:
      - sha: aaa1111
        branch: main
      - sha: bbb2222
        branch: main
"#;
        let handoff: Handoff = serde_yaml::from_str(yaml).expect("parse");
        assert_eq!(
            handoff.log[0].commits,
            vec![CommitRef::Sha("abc1234".into()), CommitRef::Sha("def5678".into())]
        );
        assert_eq!(
            handoff.log[1].commits,
            vec![
                CommitRef::Object { sha: "aaa1111".into(), branch: Some("main".into()) },
                CommitRef::Object { sha: "bbb2222".into(), branch: Some("main".into()) },
            ]
        );
        // sha() accessor works for both forms
        assert_eq!(handoff.log[0].commits[0].sha(), "abc1234");
        assert_eq!(handoff.log[1].commits[0].sha(), "aaa1111");
    }

    #[test]
    fn log_commits_with_session_field_parse() {
        // Regression: session: N field in log entry caused flatten+deserialize_with interaction
        // to pass the entire log map to the commits deserializer.
        let yaml = r#"
log:
  - date: "20260509.225051"
    summary: "Session 43: something"
    commits:
      - sha: 659f1c2
        branch: main
      - sha: 7516e53
        branch: main
    session: 43
"#;
        let handoff: Handoff = serde_yaml::from_str(yaml).expect("parse with session field");
        assert_eq!(handoff.log[0].commits.len(), 2);
        assert_eq!(handoff.log[0].commits[0].sha(), "659f1c2");
    }

    #[test]
    fn validate_catches_item_missing_id() {
        let handoff = Handoff {
            items: vec![HandoffItem {
                id: String::new(),
                title: "some task".into(),
                ..HandoffItem::default()
            }],
            ..Handoff::default()
        };
        let warnings = handoff.validate();
        assert_eq!(
            warnings,
            vec![ValidationWarning::ItemMissingId {
                index: 0,
                title: "some task".into()
            }]
        );
    }

    #[test]
    fn validate_catches_log_entry_in_items() {
        let mut extra_fields = BTreeMap::new();
        extra_fields.insert(
            "date".into(),
            serde_yaml::Value::String("20260424:152652".into()),
        );
        extra_fields.insert(
            "summary".into(),
            serde_yaml::Value::String("did stuff".into()),
        );
        let handoff = Handoff {
            items: vec![HandoffItem {
                id: String::new(),
                extra_fields,
                ..HandoffItem::default()
            }],
            ..Handoff::default()
        };
        let warnings = handoff.validate();
        assert!(warnings.contains(&ValidationWarning::LogEntryInItems {
            index: 0,
            date: "20260424:152652".into()
        }));
    }

    #[test]
    fn validate_catches_duplicate_item_ids() {
        let handoff = Handoff {
            items: vec![
                HandoffItem {
                    id: "hj-1".into(),
                    title: "first".into(),
                    ..HandoffItem::default()
                },
                HandoffItem {
                    id: "hj-2".into(),
                    title: "unique".into(),
                    ..HandoffItem::default()
                },
                HandoffItem {
                    id: "hj-1".into(),
                    title: "duplicate".into(),
                    ..HandoffItem::default()
                },
            ],
            ..Handoff::default()
        };
        let warnings = handoff.validate();
        assert!(warnings.contains(&ValidationWarning::DuplicateItemId {
            id: "hj-1".into(),
            indices: vec![0, 2]
        }));
    }

    #[test]
    fn validate_catches_log_entry_missing_summary() {
        let handoff = Handoff {
            log: vec![LogEntry {
                date: Some("20260424:152652".into()),
                summary: String::new(),
                ..LogEntry::default()
            }],
            ..Handoff::default()
        };
        let warnings = handoff.validate();
        assert_eq!(
            warnings,
            vec![ValidationWarning::LogEntryMissingSummary { index: 0 }]
        );
    }

    #[test]
    fn validate_clean_handoff_returns_empty() {
        let handoff = Handoff {
            items: vec![HandoffItem {
                id: "hj-1".into(),
                title: "valid".into(),
                ..HandoffItem::default()
            }],
            log: vec![LogEntry {
                date: Some("20260424:152652".into()),
                summary: "did things".into(),
                ..LogEntry::default()
            }],
            ..Handoff::default()
        };
        assert!(handoff.validate().is_empty());
    }

    #[test]
    fn repair_moves_log_entries_from_items_to_log() {
        let mut extra_fields = BTreeMap::new();
        extra_fields.insert(
            "date".into(),
            serde_yaml::Value::String("20260424:152652".into()),
        );
        extra_fields.insert(
            "summary".into(),
            serde_yaml::Value::String("did stuff".into()),
        );
        extra_fields.insert(
            "commits".into(),
            serde_yaml::Value::Sequence(vec![serde_yaml::Value::String("abc123".into())]),
        );
        let mut handoff = Handoff {
            items: vec![
                HandoffItem {
                    id: "hj-1".into(),
                    title: "valid".into(),
                    ..HandoffItem::default()
                },
                HandoffItem {
                    id: String::new(),
                    extra_fields,
                    ..HandoffItem::default()
                },
            ],
            ..Handoff::default()
        };
        let descriptions = handoff.repair();
        assert_eq!(handoff.items.len(), 1);
        assert_eq!(handoff.items[0].id, "hj-1");
        assert_eq!(handoff.log.len(), 1);
        assert_eq!(handoff.log[0].date.as_deref(), Some("20260424:152652"));
        assert_eq!(handoff.log[0].summary, "did stuff");
        assert_eq!(handoff.log[0].commits, vec![CommitRef::Sha("abc123".into())]);
        assert!(!descriptions.is_empty());
    }

    #[test]
    fn repair_preserves_valid_items() {
        let mut handoff = Handoff {
            items: vec![HandoffItem {
                id: "hj-1".into(),
                title: "valid".into(),
                ..HandoffItem::default()
            }],
            ..Handoff::default()
        };
        let descriptions = handoff.repair();
        assert_eq!(handoff.items.len(), 1);
        assert!(descriptions.is_empty());
    }

    #[test]
    fn repair_returns_descriptions() {
        let mut extra_fields = BTreeMap::new();
        extra_fields.insert(
            "date".into(),
            serde_yaml::Value::String("20260424:152652".into()),
        );
        extra_fields.insert("summary".into(), serde_yaml::Value::String("work".into()));
        let mut handoff = Handoff {
            items: vec![HandoffItem {
                id: String::new(),
                extra_fields,
                ..HandoffItem::default()
            }],
            ..Handoff::default()
        };
        let descriptions = handoff.repair();
        assert_eq!(descriptions.len(), 1);
        assert!(descriptions[0].contains("20260424:152652"));
        assert!(descriptions[0].contains("items[0]"))
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
        assert!(!rendered.contains("last_log"));
    }
}
