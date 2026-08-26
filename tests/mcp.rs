use rmcp::model::CallToolRequestParams;
use rmcp::service::RunningService;
use rmcp::transport::{ConfigureCommandExt, TokioChildProcess};
use rmcp::{RoleClient, ServiceExt};
use tempfile::TempDir;
use tokio::process::Command;

fn text_of(result: &rmcp::model::CallToolResult) -> String {
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
async fn mcp_log_entries_is_one_transaction() {
    let dir = TempDir::new().unwrap();
    let spec = dir.path().join("meal.yaml");
    std::fs::write(
        &spec,
        r#"
fields:
  - name: when
    type: enum
    required: true
    values: [breakfast, lunch]
  - name: kcal
    type: number
    required: true
"#,
    )
    .unwrap();
    let db = dir.path().join("bottle.db");
    let add = std::process::Command::new(env!("CARGO_BIN_EXE_bottle"))
        .arg("--db")
        .arg(&db)
        .args([
            "schema",
            "add",
            "nutrition.meal",
            "--file",
            spec.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        add.status.success(),
        "{}",
        String::from_utf8_lossy(&add.stderr)
    );

    let client = connect(&dir).await;
    let result = client
        .call_tool(
            CallToolRequestParams::new("log").with_arguments(rmcp::object!({
                "schema": "nutrition.meal",
                "at": "2026-08-22T08:14:00Z",
                "entries": [
                    { "when": "breakfast", "kcal": 1 },
                    { "when": "lunch", "kcal": 2 }
                ]
            })),
        )
        .await
        .expect("log entries");
    assert_ne!(result.is_error, Some(true), "{}", text_of(&result));
    let body = text_of(&result);
    assert!(body.starts_with("id\tat\tlinks\n"), "{body}");
    assert_eq!(body.lines().count(), 3, "{body}");

    let ls = client
        .call_tool(
            CallToolRequestParams::new("ls").with_arguments(rmcp::object!({
                "schema": "nutrition.meal"
            })),
        )
        .await
        .expect("ls");
    let ls_body = text_of(&ls);
    assert!(ls_body.contains("breakfast"), "{ls_body}");
    assert!(ls_body.contains("lunch"), "{ls_body}");
}
