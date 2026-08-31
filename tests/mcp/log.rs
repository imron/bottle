use tempfile::TempDir;

use super::common::{MEAL, SESSION, SET, connect, ok, param_err, tool_err};

#[tokio::test]
async fn writes_cells_and_links() {
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
        "schema_add_field",
        rmcp::object!({
            "schema": "nutrition.meal",
            "name": "note",
            "type": "text"
        }),
    )
    .await;
    ok(
        &client,
        "schema_add_field",
        rmcp::object!({
            "schema": "nutrition.meal",
            "name": "extra",
            "type": "number",
            "default": "0"
        }),
    )
    .await;
    let posted = ok(
        &client,
        "log",
        rmcp::object!({
            "schema": "nutrition.meal",
            "entries": [{
                "at": "2026-08-22T08:14:00Z",
                "agent": "tester",
                "fields": {
                    "when": "breakfast",
                    "what": "eggs",
                    "kcal": 568,
                    "fat": null,
                    "note": "hi",
                    "extra": 0
                }
            }]
        }),
    )
    .await;
    assert!(posted.starts_with("id\tat\tlinks\n1\t"), "{posted}");

    ok(
        &client,
        "schema_add",
        rmcp::object!({ "name": "fitness.session", "spec": SESSION }),
    )
    .await;
    ok(
        &client,
        "schema_add",
        rmcp::object!({ "name": "fitness.set", "spec": SET }),
    )
    .await;
    let session_row = ok(
        &client,
        "log",
        rmcp::object!({
            "schema": "fitness.session",
            "entries": [{ "at": "2026-08-22T08:00:00Z", "fields": { "title": "upper" } }]
        }),
    )
    .await;
    assert!(session_row.contains("\n1\t"), "{session_row}");
    let set_row = ok(
        &client,
        "log",
        rmcp::object!({
            "schema": "fitness.set",
            "entries": [{
                "at": "2026-08-22T08:01:00Z",
                "links": { "session": "fitness.session/1" },
                "fields": { "movement": "squat", "reps": 8 }
            }]
        }),
    )
    .await;
    assert!(set_row.contains("session=fitness.session/1"), "{set_row}");
}

#[tokio::test]
async fn batch_and_now() {
    let dir = TempDir::new().unwrap();
    let client = connect(&dir).await;
    ok(
        &client,
        "schema_add",
        rmcp::object!({ "name": "fitness.session", "spec": SESSION }),
    )
    .await;
    ok(
        &client,
        "schema_add",
        rmcp::object!({ "name": "fitness.set", "spec": SET }),
    )
    .await;
    ok(
        &client,
        "log",
        rmcp::object!({
            "schema": "fitness.session",
            "entries": [{ "at": "2026-08-22T08:00:00Z", "fields": { "title": "upper" } }]
        }),
    )
    .await;
    let batch = ok(
        &client,
        "log",
        rmcp::object!({
            "schema": "fitness.set",
            "entries": [
                {
                    "at": "2026-08-22T08:02:00Z",
                    "links": { "session": "fitness.session/1" },
                    "fields": { "movement": "press", "reps": 5 }
                },
                {
                    "at": "2026-08-22T08:03:00Z",
                    "agent": "coach",
                    "fields": { "movement": "plank", "reps": 1 }
                }
            ]
        }),
    )
    .await;
    assert_eq!(batch.lines().count(), 3, "{batch}");
    assert!(batch.contains("session=fitness.session/1"), "{batch}");

    ok(
        &client,
        "schema_add",
        rmcp::object!({ "name": "nutrition.meal", "spec": MEAL }),
    )
    .await;
    ok(
        &client,
        "schema_add_field",
        rmcp::object!({
            "schema": "nutrition.meal",
            "name": "extra",
            "type": "number",
            "default": "0"
        }),
    )
    .await;
    let now = ok(
        &client,
        "log",
        rmcp::object!({
            "schema": "nutrition.meal",
            "entries": [{ "fields": { "when": "lunch", "what": "now", "kcal": 1, "extra": 0 } }]
        }),
    )
    .await;
    assert!(now.contains("\n1\t"), "{now}");
}

#[tokio::test]
async fn entries_are_one_transaction() {
    let dir = TempDir::new().unwrap();
    let client = connect(&dir).await;
    ok(
        &client,
        "schema_add",
        rmcp::object!({ "name": "nutrition.meal", "spec": MEAL }),
    )
    .await;
    let result = ok(
        &client,
        "log",
        rmcp::object!({
            "schema": "nutrition.meal",
            "entries": [
                { "at": "2026-08-22T08:14:00Z", "fields": { "when": "breakfast", "what": "eggs", "kcal": 1 } },
                { "at": "2026-08-22T08:14:00Z", "fields": { "when": "lunch", "what": "rice", "kcal": 2 } }
            ]
        }),
    )
    .await;
    assert!(result.starts_with("id\tat\tlinks\n"), "{result}");
    assert_eq!(result.lines().count(), 3, "{result}");
}

#[tokio::test]
async fn shape_errors_are_protocol() {
    let dir = TempDir::new().unwrap();
    let client = connect(&dir).await;
    let unknown = param_err(
        &client,
        "log",
        rmcp::object!({
            "schema": "nutrition.meal",
            "fields": { "when": "breakfast" }
        }),
    )
    .await;
    assert!(unknown.contains("fields"), "{unknown}");
    let missing = param_err(
        &client,
        "log",
        rmcp::object!({ "schema": "nutrition.meal" }),
    )
    .await;
    assert!(missing.contains("entries"), "{missing}");
    let leftover = param_err(
        &client,
        "log",
        rmcp::object!({
            "schema": "nutrition.meal",
            "entries": [{ "when": "breakfast" }]
        }),
    )
    .await;
    assert!(
        leftover.contains("when") || leftover.contains("unknown"),
        "{leftover}"
    );
}

#[tokio::test]
async fn empty_log_is_tool_error() {
    let dir = TempDir::new().unwrap();
    let client = connect(&dir).await;
    let empty = tool_err(
        &client,
        "log",
        rmcp::object!({
            "schema": "nutrition.meal",
            "entries": []
        }),
    )
    .await;
    assert!(empty.contains("log requires at least one entry"), "{empty}");
}

#[tokio::test]
async fn check_does_not_write() {
    let dir = TempDir::new().unwrap();
    let client = connect(&dir).await;
    ok(
        &client,
        "schema_add",
        rmcp::object!({ "name": "nutrition.meal", "spec": MEAL }),
    )
    .await;
    let out = ok(
        &client,
        "log",
        rmcp::object!({
            "schema": "nutrition.meal",
            "check": true,
            "entries": [{
                "at": "2026-08-22T08:14:00Z",
                "fields": { "when": "breakfast", "what": "eggs", "kcal": 1 }
            }]
        }),
    )
    .await;
    assert_eq!(out, "rows\n1\n");
    let ls = ok(&client, "ls", rmcp::object!({ "schema": "nutrition.meal" })).await;
    assert_eq!(ls, "id\tat\tlinks\twhen\twhat\tkcal\tfat\tagent\n");
}

#[tokio::test]
async fn rejects_bad_value_shape() {
    let dir = TempDir::new().unwrap();
    let client = connect(&dir).await;
    let bad_cell = param_err(
        &client,
        "log",
        rmcp::object!({
            "schema": "nutrition.meal",
            "entries": [{ "fields": { "when": ["breakfast"] } }]
        }),
    )
    .await;
    assert!(bad_cell.contains("strings or numbers"), "{bad_cell}");
    let bad_bool = param_err(
        &client,
        "log",
        rmcp::object!({
            "schema": "nutrition.meal",
            "entries": [{ "fields": { "when": true } }]
        }),
    )
    .await;
    assert!(bad_bool.contains("strings or numbers"), "{bad_bool}");
}
