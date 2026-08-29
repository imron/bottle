use rmcp::model::CallToolRequestParams;
use tempfile::TempDir;

use super::common::{MEAL, SESSION, SET, connect, ok, param_err, text_of, tool_err};

#[tokio::test]
async fn lists_tools_and_empty_schema_list() {
    let dir = TempDir::new().unwrap();
    let client = connect(&dir).await;
    let tools = client.list_all_tools().await.expect("list tools");
    let names: Vec<String> = tools.iter().map(|t| t.name.to_string()).collect();
    for name in [
        "help",
        "schema_list",
        "schema_show",
        "schema_add",
        "schema_add_field",
        "schema_add_value",
        "schema_rename_field",
        "schema_retire",
        "schema_drop",
        "log",
        "ls",
        "get",
        "sum",
        "last",
        "today",
        "amend",
        "ignore",
        "unignore",
        "backup",
    ] {
        assert!(
            names.iter().any(|n| n == name),
            "missing {name} in {names:?}"
        );
    }
    let result = client
        .call_tool(CallToolRequestParams::new("schema_list"))
        .await
        .expect("schema_list");
    assert_ne!(result.is_error, Some(true), "{}", text_of(&result));
    assert_eq!(text_of(&result), "name\tretired\n");
}

#[tokio::test]
async fn help_pages() {
    let dir = TempDir::new().unwrap();
    let client = connect(&dir).await;
    let overview = client
        .call_tool(CallToolRequestParams::new("help"))
        .await
        .expect("help");
    assert!(
        text_of(&overview).starts_with("# overview\n"),
        "{}",
        text_of(&overview)
    );
    let log_help = ok(&client, "help", rmcp::object!({ "command": "log" })).await;
    assert!(log_help.starts_with("# log\n"), "{log_help}");
    let unknown_help = tool_err(&client, "help", rmcp::object!({ "command": "nope" })).await;
    assert!(
        unknown_help.contains("unknown help topic"),
        "{unknown_help}"
    );
}

#[tokio::test]
async fn add_show_list_and_mutate() {
    let dir = TempDir::new().unwrap();
    let client = connect(&dir).await;
    assert!(
        ok(
            &client,
            "schema_add",
            rmcp::object!({ "name": "nutrition.meal", "spec": MEAL }),
        )
        .await
        .is_empty()
    );
    let listed = ok(&client, "schema_list", rmcp::object!({})).await;
    assert!(listed.contains("nutrition.meal"), "{listed}");
    let show = ok(
        &client,
        "schema_show",
        rmcp::object!({ "name": "nutrition.meal" }),
    )
    .await;
    assert!(show.contains("when\tenum"), "{show}");
    let yaml = ok(
        &client,
        "schema_show",
        rmcp::object!({ "name": "nutrition.meal", "yaml": true }),
    )
    .await;
    assert!(yaml.contains("type: enum"), "{yaml}");
    assert!(
        ok(
            &client,
            "schema_add_field",
            rmcp::object!({
                "schema": "nutrition.meal",
                "name": "note",
                "type": "text"
            }),
        )
        .await
        .is_empty()
    );
    assert!(
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
        .await
        .is_empty()
    );
    assert!(
        ok(
            &client,
            "schema_add_field",
            rmcp::object!({
                "schema": "nutrition.meal",
                "name": "mood",
                "type": "enum",
                "values": ["ok", "good"]
            }),
        )
        .await
        .is_empty()
    );
    assert!(
        ok(
            &client,
            "schema_add_value",
            rmcp::object!({
                "schema": "nutrition.meal",
                "field": "when",
                "value": "brunch"
            }),
        )
        .await
        .is_empty()
    );
    assert!(
        ok(
            &client,
            "schema_rename_field",
            rmcp::object!({
                "schema": "nutrition.meal",
                "from": "note",
                "to": "memo"
            }),
        )
        .await
        .is_empty()
    );
    let show = ok(
        &client,
        "schema_show",
        rmcp::object!({ "name": "nutrition.meal" }),
    )
    .await;
    assert!(show.contains("memo\ttext"), "{show}");
    assert!(!show.contains("note\ttext"), "{show}");
}

#[tokio::test]
async fn unknown_add_field_type() {
    let dir = TempDir::new().unwrap();
    let client = connect(&dir).await;
    let bad_type = param_err(
        &client,
        "schema_add_field",
        rmcp::object!({
            "schema": "nutrition.meal",
            "name": "note",
            "type": "nope"
        }),
    )
    .await;
    assert!(
        bad_type.contains("nope") && (bad_type.contains("unknown") || bad_type.contains("invalid")),
        "{bad_type}"
    );
}

#[tokio::test]
async fn retire_blocks_log() {
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
    assert!(
        ok(
            &client,
            "schema_retire",
            rmcp::object!({ "name": "nutrition.meal" }),
        )
        .await
        .is_empty()
    );
    let retired_log = tool_err(
        &client,
        "log",
        rmcp::object!({
            "schema": "nutrition.meal",
            "entries": [{ "fields": { "when": "lunch", "what": "x", "kcal": 1, "extra": 0 } }]
        }),
    )
    .await;
    assert!(retired_log.contains("retired"), "{retired_log}");
}

#[tokio::test]
async fn drop_after_unlink() {
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
    ok(
        &client,
        "amend",
        rmcp::object!({
            "schema": "fitness.set",
            "id": 1,
            "unlink": ["session"]
        }),
    )
    .await;
    assert!(
        ok(
            &client,
            "schema_drop",
            rmcp::object!({ "name": "fitness.session" }),
        )
        .await
        .is_empty()
    );
}
