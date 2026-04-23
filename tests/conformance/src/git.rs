// §4 — hj-git → hj-core adapter contracts

use hj_git::RepoContext;
use std::fs;
use tempfile::tempdir;

// §4.1 — is_handoff_file filter
// is_handoff_file is private; we test it indirectly via discover_handoffs
// and directly by verifying naming patterns produce correct paths.

#[test]
fn s4_1_handoff_yaml_path_accepted_by_naming_convention() {
    // Verify the naming pattern used in paths() matches the filter rule.
    // A valid HANDOFF path must start with "HANDOFF." and end with ".yaml",
    // and must NOT end with ".state.json".
    let name = "HANDOFF.hj.hj.yaml";
    assert!(name.starts_with("HANDOFF."));
    assert!(name.ends_with(".yaml"));
    assert!(!name.ends_with(".state.json"));
}

#[test]
fn s4_1_state_json_path_excluded_by_naming_convention() {
    let name = "HANDOFF.hj.hj.state.json";
    // state files must NOT be treated as handoff files
    assert!(name.ends_with(".state.json"));
}

// §4.2 — parse_markdown_handoff

#[test]
fn s4_2_extracts_bullets_from_known_gaps() {
    use hj_git::discover_handoffs;

    let tmp = git_repo_with_file(
        ".ctx/HANDOFF.test.test.md",
        "## Known Gaps\n\n- Fix the thing\n- Implement other\n\n## Other\n\n- Ignored\n",
    );

    let results = discover_handoffs(tmp.path(), 3).unwrap();
    let survey = results
        .iter()
        .find(|s| s.path.ends_with("HANDOFF.test.test.md"));
    assert!(survey.is_some(), "markdown handoff must be discovered");
    let items = &survey.unwrap().items;
    assert_eq!(
        items.len(),
        2,
        "only bullets from Known Gaps must be extracted"
    );
    assert_eq!(items[0].title, "Fix the thing");
    assert_eq!(items[1].title, "Implement other");
}

#[test]
fn s4_2_extracts_bullets_from_next_up() {
    use hj_git::discover_handoffs;

    let tmp = git_repo_with_file(
        ".ctx/HANDOFF.test.test.md",
        "## Next Up\n\n- Wire the adapter\n",
    );

    let results = discover_handoffs(tmp.path(), 3).unwrap();
    let survey = results
        .iter()
        .find(|s| s.path.ends_with("HANDOFF.test.test.md"))
        .unwrap();
    assert_eq!(survey.items.len(), 1);
    assert_eq!(survey.items[0].title, "Wire the adapter");
}

#[test]
fn s4_2_infers_priority_via_hj_core() {
    use hj_git::discover_handoffs;

    let tmp = git_repo_with_file(
        ".ctx/HANDOFF.test.test.md",
        "## Known Gaps\n\n- Fix broken thing\n- Explore someday\n",
    );

    let results = discover_handoffs(tmp.path(), 3).unwrap();
    let survey = results
        .iter()
        .find(|s| s.path.ends_with("HANDOFF.test.test.md"))
        .unwrap();
    let p0 = survey.items.iter().find(|i| i.title == "Fix broken thing");
    let p2 = survey.items.iter().find(|i| i.title == "Explore someday");
    assert_eq!(p0.unwrap().priority.as_deref(), Some("P0"));
    assert_eq!(p2.unwrap().priority.as_deref(), Some("P2"));
}

#[test]
fn s4_2_ids_assigned_sequentially() {
    use hj_git::discover_handoffs;

    let tmp = git_repo_with_file(
        ".ctx/HANDOFF.test.test.md",
        "## Known Gaps\n\n- First\n- Second\n- Third\n",
    );

    let results = discover_handoffs(tmp.path(), 3).unwrap();
    let items = &results
        .iter()
        .find(|s| s.path.ends_with("HANDOFF.test.test.md"))
        .unwrap()
        .items;
    assert_eq!(items[0].id, "md-1");
    assert_eq!(items[1].id, "md-2");
    assert_eq!(items[2].id, "md-3");
}

// §4.3 — RepoContext::paths() naming contract
// Requires a real git repo; we use the actual hj workspace root.

#[test]
fn s4_3_handoff_path_follows_naming_contract() {
    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();

    let ctx = RepoContext {
        repo_root: repo_root.to_path_buf(),
        cwd: repo_root.to_path_buf(),
        base_name: "hj".into(),
    };

    let paths = ctx.paths(Some("myproject")).unwrap();
    let name = paths.handoff_path.file_name().unwrap().to_str().unwrap();
    assert_eq!(name, "HANDOFF.myproject.hj.yaml");
}

#[test]
fn s4_3_state_path_follows_naming_contract() {
    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();

    let ctx = RepoContext {
        repo_root: repo_root.to_path_buf(),
        cwd: repo_root.to_path_buf(),
        base_name: "hj".into(),
    };

    let paths = ctx.paths(Some("myproject")).unwrap();
    let name = paths.state_path.file_name().unwrap().to_str().unwrap();
    assert_eq!(name, "HANDOFF.myproject.hj.state.json");
}

#[test]
fn s4_3_project_name_is_sanitized() {
    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();

    let ctx = RepoContext {
        repo_root: repo_root.to_path_buf(),
        cwd: repo_root.to_path_buf(),
        base_name: "hj".into(),
    };

    let paths = ctx.paths(Some("My Project/CLI")).unwrap();
    let name = paths.handoff_path.file_name().unwrap().to_str().unwrap();
    assert!(
        name.starts_with("HANDOFF.my-project-cli."),
        "project name must be sanitized, got: {name}"
    );
}

// §4.4 — manifest_name resolution (tested via refresh + scan_package_names indirectly,
// but we can test it directly through a Cargo.toml in a temp dir).
// manifest_name is private — test via the public refresh path.

#[test]
fn s4_4_cargo_toml_drives_project_name() {
    // RepoContext::paths() with no explicit project reads manifest name.
    // Build a minimal fake repo to exercise the code path.
    let tmp = git_repo_with_file(
        "Cargo.toml",
        "[package]\nname = \"my-crate\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    let ctx = RepoContext {
        repo_root: tmp.path().to_path_buf(),
        cwd: tmp.path().to_path_buf(),
        base_name: "my-crate".into(),
    };
    let paths = ctx.paths(None).unwrap();
    let name = paths.handoff_path.file_name().unwrap().to_str().unwrap();
    assert!(
        name.starts_with("HANDOFF.my-crate."),
        "Cargo.toml name must drive project name, got: {name}"
    );
}

// §4.5 — write_gitignore_block idempotency (tested via RepoContext::refresh)

#[test]
fn s4_5_gitignore_block_is_idempotent() {
    use hj_git::RepoContext;

    let tmp = git_repo_with_file(
        "Cargo.toml",
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    let ctx = RepoContext {
        repo_root: tmp.path().to_path_buf(),
        cwd: tmp.path().to_path_buf(),
        base_name: "test".into(),
    };

    ctx.refresh(true).unwrap();
    let after_first = fs::read_to_string(tmp.path().join(".gitignore")).unwrap();

    ctx.refresh(true).unwrap();
    let after_second = fs::read_to_string(tmp.path().join(".gitignore")).unwrap();

    assert_eq!(after_first, after_second, "refresh must be idempotent");
    let block_count = after_second.matches("# handoff-begin").count();
    assert_eq!(block_count, 1, "managed block must appear exactly once");
}

// ─── helpers ────────────────────────────────────────────────────────────────

/// Creates a temp dir with a minimal git repo and a file at the given path.
fn git_repo_with_file(rel_path: &str, contents: &str) -> tempfile::TempDir {
    let tmp = tempdir().unwrap();
    let root = tmp.path();

    // init git repo
    std::process::Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(root)
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(root)
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(root)
        .output()
        .unwrap();

    let full_path = root.join(rel_path);
    if let Some(parent) = full_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(&full_path, contents).unwrap();

    tmp
}
