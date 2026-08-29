use rmcp::RoleClient;
use rmcp::service::RunningService;
use tempfile::TempDir;

use super::common::{MEAL, SESSION, SET, connect, ok, param_err, tool_err};

async fn seed_meals_and_sets(client: &RunningService<RoleClient, ()>) {
    ok(
        client,
        "schema_add",
        rmcp::object!({ "name": "nutrition.meal", "spec": MEAL }),
    )
    .await;
    ok(
        client,
        "schema_add_field",
        rmcp::object!({
            "schema": "nutrition.meal",
            "name": "extra",
            "type": "number",
            "default": "0"
        }),
    )
    .await;
    ok(
        client,
        "schema_add",
        rmcp::object!({ "name": "fitness.session", "spec": SESSION }),
    )
    .await;
    ok(
        client,
        "schema_add",
        rmcp::object!({ "name": "fitness.set", "spec": SET }),
    )
    .await;
    ok(
        client,
        "log",
        rmcp::object!({
            "schema": "nutrition.meal",
            "entries": [{
                "at": "2026-08-22T08:14:00Z",
                "agent": "tester",
                "fields": { "when": "breakfast", "what": "eggs", "kcal": 568, "extra": 0 }
            }]
        }),
    )
    .await;
    ok(
        client,
        "log",
        rmcp::object!({
            "schema": "nutrition.meal",
            "entries": [{
                "at": "2026-08-23T12:00:00Z",
                "fields": { "when": "lunch", "what": "rice", "kcal": 200, "extra": 0 }
            }]
        }),
    )
    .await;
    ok(
        client,
        "log",
        rmcp::object!({
            "schema": "fitness.session",
            "entries": [{ "at": "2026-08-22T08:00:00Z", "fields": { "title": "upper" } }]
        }),
    )
    .await;
    ok(
        client,
        "log",
        rmcp::object!({
            "schema": "fitness.set",
            "entries": [
                {
                    "at": "2026-08-22T08:01:00Z",
                    "links": { "session": "fitness.session/1" },
                    "fields": { "movement": "squat", "reps": 8 }
                },
                {
                    "at": "2026-08-22T08:02:00Z",
                    "links": { "session": "fitness.session/1" },
                    "fields": { "movement": "press", "reps": 5 }
                },
                {
                    "at": "2026-08-22T08:03:00Z",
                    "fields": { "movement": "plank", "reps": 1 }
                }
            ]
        }),
    )
    .await;
}

#[tokio::test]
async fn ls_filters() {
    let dir = TempDir::new().unwrap();
    let client = connect(&dir).await;
    seed_meals_and_sets(&client).await;
    let filtered = ok(
        &client,
        "ls",
        rmcp::object!({
            "schema": "nutrition.meal",
            "from": "2026-08-22",
            "to": "2026-08-22",
            "agent": "tester",
            "where": { "when": "breakfast" }
        }),
    )
    .await;
    assert!(filtered.contains("eggs"), "{filtered}");
    assert!(!filtered.contains("rice"), "{filtered}");
    let by_kcal = ok(
        &client,
        "ls",
        rmcp::object!({
            "schema": "nutrition.meal",
            "where": { "kcal": 568 }
        }),
    )
    .await;
    assert!(by_kcal.contains("eggs"), "{by_kcal}");
    assert!(!by_kcal.contains("rice"), "{by_kcal}");
    let excluded = ok(
        &client,
        "ls",
        rmcp::object!({
            "schema": "nutrition.meal",
            "exclude": [{ "field": "when", "value": "lunch" }]
        }),
    )
    .await;
    assert!(excluded.contains("eggs"), "{excluded}");
    assert!(!excluded.contains("rice"), "{excluded}");
    let linked = ok(
        &client,
        "ls",
        rmcp::object!({
            "schema": "fitness.set",
            "links": { "session": "fitness.session/1" }
        }),
    )
    .await;
    assert!(linked.contains("squat"), "{linked}");
    assert!(linked.contains("press"), "{linked}");
    assert!(!linked.contains("plank"), "{linked}");
}

#[tokio::test]
async fn get_sum_last() {
    let dir = TempDir::new().unwrap();
    let client = connect(&dir).await;
    seed_meals_and_sets(&client).await;
    let got = ok(
        &client,
        "get",
        rmcp::object!({ "schema": "nutrition.meal", "id": 1 }),
    )
    .await;
    assert!(got.contains("ignored"), "{got}");
    assert!(got.contains("true") || got.contains("false"), "{got}");
    let total = ok(
        &client,
        "sum",
        rmcp::object!({
            "schema": "nutrition.meal",
            "field": "kcal",
            "from": "2026-08-22",
            "to": "2026-08-22"
        }),
    )
    .await;
    assert!(total.contains("568"), "{total}");
    let grouped = ok(
        &client,
        "sum",
        rmcp::object!({
            "schema": "fitness.set",
            "field": "reps",
            "group": "session",
            "where": { "movement": "squat" }
        }),
    )
    .await;
    assert!(grouped.contains("fitness.session/1"), "{grouped}");
    let last = ok(
        &client,
        "last",
        rmcp::object!({
            "schema": "nutrition.meal",
            "where": { "what": "rice" }
        }),
    )
    .await;
    assert!(last.contains("rice"), "{last}");
    let missing = tool_err(
        &client,
        "get",
        rmcp::object!({ "schema": "nutrition.meal", "id": 99 }),
    )
    .await;
    assert!(missing.contains("not found"), "{missing}");
}

#[tokio::test]
async fn today_lists_current_civil_day() {
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
            "name": "extra",
            "type": "number",
            "default": "0"
        }),
    )
    .await;
    ok(
        &client,
        "log",
        rmcp::object!({
            "schema": "nutrition.meal",
            "entries": [{ "fields": { "when": "lunch", "what": "now", "kcal": 1, "extra": 0 } }]
        }),
    )
    .await;
    let today = ok(
        &client,
        "today",
        rmcp::object!({
            "schema": "nutrition.meal",
            "agent": "bottle"
        }),
    )
    .await;
    assert!(today.contains("now"), "{today}");
}

#[tokio::test]
async fn where_rejects_bool() {
    let dir = TempDir::new().unwrap();
    let client = connect(&dir).await;
    let bad_where = param_err(
        &client,
        "ls",
        rmcp::object!({
            "schema": "nutrition.meal",
            "where": { "kcal": true }
        }),
    )
    .await;
    assert!(bad_where.contains("strings or numbers"), "{bad_where}");
}
