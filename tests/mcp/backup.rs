use tempfile::TempDir;

use super::common::{MEAL, connect, ok, tool_err};

#[tokio::test]
async fn backup_round_trips_through_mcp() {
    let dir = TempDir::new().unwrap();
    let client = connect(&dir).await;
    ok(
        &client,
        "schema_add",
        rmcp::object!({ "name": "nutrition.meal", "spec": MEAL }),
    )
    .await;
    ok(
        &client,
        "log",
        rmcp::object!({
            "schema": "nutrition.meal",
            "entries": [{
                "at": "2026-08-22T08:14:00Z",
                "fields": { "when": "breakfast", "what": "eggs", "kcal": 568 }
            }]
        }),
    )
    .await;
    let dest = dir.path().join("copy.db");
    let out = ok(
        &client,
        "backup",
        rmcp::object!({ "path": dest.to_str().unwrap() }),
    )
    .await;
    assert!(out.is_empty(), "{out}");
    assert!(dest.is_file());
    let err = tool_err(
        &client,
        "backup",
        rmcp::object!({ "path": dest.to_str().unwrap() }),
    )
    .await;
    assert!(err.contains("file exists"), "{err}");
}
