// §3 — hj-doob → hj-core adapter contracts
// §3.3 (DoobClient::snapshot) is an integration test skipped without live doob.

use hj_doob::{map_priority, unique_titles};

// §3.1 — map_priority shell parity

#[test]
fn s3_1_p0_maps_to_5() {
    assert_eq!(map_priority(Some("P0")), 5);
}

#[test]
fn s3_1_p1_maps_to_4() {
    assert_eq!(map_priority(Some("P1")), 4);
}

#[test]
fn s3_1_p2_maps_to_3() {
    assert_eq!(map_priority(Some("P2")), 3);
}

#[test]
fn s3_1_unknown_priority_maps_to_1() {
    assert_eq!(map_priority(Some("other")), 1);
    assert_eq!(map_priority(None), 1);
    assert_eq!(map_priority(Some("")), 1);
}

// §3.2 — unique_titles

#[test]
fn s3_2_deduplicates_exact_match() {
    let result = unique_titles(vec!["A".into(), "B".into(), "A".into()]);
    assert_eq!(result.iter().filter(|s| s.as_str() == "A").count(), 1);
}

#[test]
fn s3_2_drops_empty_strings() {
    let result = unique_titles(vec!["A".into(), String::new(), "B".into()]);
    assert!(
        !result.contains(&String::new()),
        "empty string must be dropped"
    );
}

#[test]
fn s3_2_result_is_sorted() {
    let result = unique_titles(vec!["C".into(), "A".into(), "B".into()]);
    let mut sorted = result.clone();
    sorted.sort();
    assert_eq!(result, sorted, "unique_titles must return sorted order");
}

// §3.3 — DoobClient::snapshot [integration — requires live doob]

#[test]
#[ignore = "requires doob on PATH"]
fn s3_3_snapshot_returns_todo_snapshot_type() {
    use hj_doob::DoobClient;
    let client = DoobClient::new(std::env::current_dir().unwrap());
    // Just verify it returns the correct hj-core type without panicking.
    let snapshot = client.snapshot("hj");
    assert!(snapshot.is_ok(), "snapshot must succeed with doob on PATH");
}
