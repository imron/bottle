use rmcp::model::{CallToolRequestParams, CallToolResult};
use rmcp::service::{RunningService, ServiceError};
use rmcp::transport::{ConfigureCommandExt, TokioChildProcess};
use rmcp::{RoleClient, ServiceExt};
use tempfile::TempDir;
use tokio::process::Command;

pub fn text_of(result: &CallToolResult) -> String {
    result
        .content
        .iter()
        .filter_map(|b| b.as_text().map(|t| t.text.as_str()))
        .collect::<Vec<_>>()
        .join("")
}

pub async fn connect(dir: &TempDir) -> RunningService<RoleClient, ()> {
    let db = dir.path().join("bottle.db");
    let transport =
        TokioChildProcess::new(Command::new(env!("CARGO_BIN_EXE_bottle")).configure(|cmd| {
            cmd.arg("--db").arg(&db).arg("mcp");
        }))
        .expect("spawn bottle mcp");
    ().serve(transport).await.expect("mcp handshake")
}

pub fn params(name: &'static str, args: rmcp::model::JsonObject) -> CallToolRequestParams {
    CallToolRequestParams::new(name).with_arguments(args)
}

pub async fn ok(
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

pub async fn tool_err(
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

pub async fn param_err(
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

pub const MEAL: &str = r#"
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

pub const SESSION: &str = r#"
fields:
  - name: title
    type: text
    required: false
"#;

pub const SET: &str = r#"
fields:
  - name: movement
    type: text
    required: true
  - name: reps
    type: number
    required: true
"#;
