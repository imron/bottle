use rmcp::model::{CallToolRequestParams, CallToolResult};
use rmcp::service::{RunningService, ServiceError};
use rmcp::transport::{ConfigureCommandExt, TokioChildProcess};
use rmcp::{RoleClient, ServiceExt};
use tempfile::TempDir;
use tokio::process::Command;

fn text_of(result: &CallToolResult) -> String {
    result
        .content
        .iter()
        .filter_map(|b| b.as_text().map(|t| t.text.as_str()))
        .collect::<Vec<_>>()
        .join("")
}

async fn connect(dir: &TempDir) -> RunningService<RoleClient, ()> {
    let db = dir.path().join("bottle.db");
    let transport =
        TokioChildProcess::new(Command::new(env!("CARGO_BIN_EXE_bottle")).configure(|cmd| {
            cmd.arg("--db").arg(&db).arg("mcp");
        }))
        .expect("spawn bottle mcp");
    ().serve(transport).await.expect("mcp handshake")
}

fn params(name: &'static str, args: rmcp::model::JsonObject) -> CallToolRequestParams {
    CallToolRequestParams::new(name).with_arguments(args)
}

async fn ok(
    client: &RunningService<RoleClient, ()>,
    name: &'static str,
    args: rmcp::model::JsonObject,
) -> String {
    let result = client
        .call_tool(params(name, args))
        .await
        .unwrap_or_else(|e| panic!("{name}: {e}"));
    assert_ne!(
        result.is_error,
        Some(true),
        "{name} tool error: {}",
        text_of(&result)
    );
    text_of(&result)
}

async fn tool_err(
    client: &RunningService<RoleClient, ()>,
    name: &'static str,
    args: rmcp::model::JsonObject,
) -> String {
    let result = client
        .call_tool(params(name, args))
        .await
        .unwrap_or_else(|e| panic!("{name} should be a tool error, got protocol: {e}"));
    assert_eq!(
        result.is_error,
        Some(true),
        "{name} expected tool error, got: {}",
        text_of(&result)
    );
    text_of(&result)
}

async fn proto_err(
    client: &RunningService<RoleClient, ()>,
    name: &'static str,
    args: rmcp::model::JsonObject,
) -> String {
    match client.call_tool(params(name, args)).await {
        Err(ServiceError::McpError(err)) => err.to_string(),
        Err(err) => panic!("{name}: expected McpError, got {err}"),
        Ok(result) => panic!(
            "{name}: expected protocol error, got tool result {}",
            text_of(&result)
        ),
    }
}

async fn param_err(
    client: &RunningService<RoleClient, ()>,
    name: &'static str,
    args: rmcp::model::JsonObject,
) -> String {
    match client.call_tool(params(name, args)).await {
        Err(ServiceError::McpError(err)) => err.to_string(),
        Err(err) => panic!("{name}: expected param error, got {err}"),
        Ok(result) => {
            let body = text_of(&result);
            assert!(
                result.is_error == Some(true) || body.contains("deserialize"),
                "{name}: expected param error, got {body}"
            );
            body
        }
    }
}

const MEAL: &str = r#"
fields:
  - name: when
    type: enum
    required: true
    values: [breakfast, lunch]
  - name: what
    type: text
    required: true
  - name: kcal
    type: number
    required: true
  - name: fat
    type: number
    required: false
"#;

const SESSION: &str = r#"
fields:
  - name: title
    type: text
    required: false
"#;

const SET: &str = r#"
fields:
  - name: movement
    type: text
    required: true
  - name: reps
    type: number
    required: true
"#;

#[tokio::test]
async fn mcp_lists_tools_and_schema_list() {
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
async fn mcp_tools_cover_the_surface() {
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

    assert!(
        ok(
            &client,
            "schema_add",
            rmcp::object!({ "name": "nutrition.meal", "spec": MEAL }),
        )
        .await
        .is_empty()
    );
    assert!(
        ok(
            &client,
            "schema_add",
            rmcp::object!({ "name": "fitness.session", "spec": SESSION }),
        )
        .await
        .is_empty()
    );
    assert!(
        ok(
            &client,
            "schema_add",
            rmcp::object!({ "name": "fitness.set", "spec": SET }),
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

    let posted = ok(
        &client,
        "log",
        rmcp::object!({
            "schema": "nutrition.meal",
            "entries": [{
                "at": "2026-08-22T08:14:00Z",
                "agent": "tester",
                "when": "breakfast",
                "what": "eggs",
                "kcal": 568,
                "fat": null,
                "note": "hi",
                "extra": 0
            }]
        }),
    )
    .await;
    assert!(posted.starts_with("id\tat\tlinks\n1\t"), "{posted}");

    let session_row = ok(
        &client,
        "log",
        rmcp::object!({
            "schema": "fitness.session",
            "entries": [{ "at": "2026-08-22T08:00:00Z", "title": "upper" }]
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
                "movement": "squat",
                "reps": 8
            }]
        }),
    )
    .await;
    assert!(set_row.contains("session=fitness.session/1"), "{set_row}");

    let batch = ok(
        &client,
        "log",
        rmcp::object!({
            "schema": "fitness.set",
            "entries": [
                {
                    "movement": "press",
                    "reps": 5,
                    "at": "2026-08-22T08:02:00Z",
                    "links": { "session": "fitness.session/1" }
                },
                {
                    "movement": "plank",
                    "reps": 1,
                    "at": "2026-08-22T08:03:00Z",
                    "agent": "coach"
                }
            ]
        }),
    )
    .await;
    assert_eq!(batch.lines().count(), 3, "{batch}");
    assert!(batch.contains("session=fitness.session/1"), "{batch}");

    let lunch = ok(
        &client,
        "log",
        rmcp::object!({
            "schema": "nutrition.meal",
            "entries": [{
                "at": "2026-08-23T12:00:00Z",
                "when": "lunch",
                "what": "rice",
                "kcal": 200,
                "extra": 0
            }]
        }),
    )
    .await;
    assert!(lunch.contains("\n2\t"), "{lunch}");

    let now = ok(
        &client,
        "log",
        rmcp::object!({
            "schema": "nutrition.meal",
            "entries": [{ "when": "lunch", "what": "now", "kcal": 1, "extra": 0 }]
        }),
    )
    .await;
    assert!(now.contains("\n3\t"), "{now}");

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
            "entries": [{ "when": "lunch", "what": "x", "kcal": 1, "extra": 0 }]
        }),
    )
    .await;
    assert!(retired_log.contains("retired"), "{retired_log}");

    let missing = tool_err(
        &client,
        "get",
        rmcp::object!({ "schema": "nutrition.meal", "id": 99 }),
    )
    .await;
    assert!(missing.contains("not found"), "{missing}");

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
    ok(
        &client,
        "amend",
        rmcp::object!({
            "schema": "fitness.set",
            "id": 2,
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

#[tokio::test]
async fn mcp_log_entries_is_one_transaction() {
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
                { "when": "breakfast", "what": "eggs", "kcal": 1, "at": "2026-08-22T08:14:00Z" },
                { "when": "lunch", "what": "rice", "kcal": 2, "at": "2026-08-22T08:14:00Z" }
            ]
        }),
    )
    .await;
    assert!(result.starts_with("id\tat\tlinks\n"), "{result}");
    assert_eq!(result.lines().count(), 3, "{result}");
}

#[tokio::test]
async fn mcp_log_shape_errors_are_protocol() {
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
    let empty = proto_err(
        &client,
        "log",
        rmcp::object!({
            "schema": "nutrition.meal",
            "entries": []
        }),
    )
    .await;
    assert!(empty.contains("entries is empty"), "{empty}");
}

#[tokio::test]
async fn mcp_rejects_bad_field_type_and_value_shape() {
    let dir = TempDir::new().unwrap();
    let client = connect(&dir).await;
    let bad_type = tool_err(
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
        bad_type.contains("nope") && bad_type.contains("unknown"),
        "{bad_type}"
    );
    let bad_cell = param_err(
        &client,
        "log",
        rmcp::object!({
            "schema": "nutrition.meal",
            "entries": [{ "when": ["breakfast"] }]
        }),
    )
    .await;
    assert!(bad_cell.contains("strings or numbers"), "{bad_cell}");
    let bad_bool = param_err(
        &client,
        "log",
        rmcp::object!({
            "schema": "nutrition.meal",
            "entries": [{ "when": true }]
        }),
    )
    .await;
    assert!(bad_bool.contains("strings or numbers"), "{bad_bool}");
}
