use tempfile::TempDir;

use super::common::{MEAL, SESSION, SET, connect, ok};

#[tokio::test]
async fn unlink_and_relink() {
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
    ok(
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
    let amended = ok(
        &client,
        "amend",
        rmcp::object!({
            "schema": "fitness.set",
            "id": 1,
            "at": "2026-08-22T08:05:00Z",
            "agent": "coach",
            "unlink": ["session"],
            "fields": { "reps": 6 }
        }),
    )
    .await;
    assert!(!amended.contains("session=fitness.session/1"), "{amended}");
    let relinked = ok(
        &client,
        "amend",
        rmcp::object!({
            "schema": "fitness.set",
            "id": 1,
            "links": { "session": "fitness.session/1" }
        }),
    )
    .await;
    assert!(relinked.contains("session=fitness.session/1"), "{relinked}");
}

#[tokio::test]
async fn ignore_hides_from_ls() {
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
    let ignored = ok(
        &client,
        "ignore",
        rmcp::object!({ "schema": "nutrition.meal", "id": 1 }),
    )
    .await;
    assert!(ignored.starts_with("id\tat\n"), "{ignored}");
    let hidden = ok(&client, "ls", rmcp::object!({ "schema": "nutrition.meal" })).await;
    assert!(!hidden.contains("eggs"), "{hidden}");
    let shown = ok(
        &client,
        "ls",
        rmcp::object!({
            "schema": "nutrition.meal",
            "include_ignored": true
        }),
    )
    .await;
    assert!(shown.contains("eggs"), "{shown}");
    assert!(shown.contains("true"), "{shown}");
    let restored = ok(
        &client,
        "unignore",
        rmcp::object!({ "schema": "nutrition.meal", "id": 1 }),
    )
    .await;
    assert!(restored.starts_with("id\tat\n"), "{restored}");
    let visible = ok(&client, "ls", rmcp::object!({ "schema": "nutrition.meal" })).await;
    assert!(visible.contains("eggs"), "{visible}");
    assert!(!visible.contains("true"), "{visible}");
}
