use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock, ServerCapabilities, ServerInfo};
use rmcp::transport::stdio;
use rmcp::{
    ErrorData as McpError, ServerHandler, ServiceExt, schemars, tool, tool_handler, tool_router,
};
use serde::Deserialize;

use crate::error::{Error, Fail};
use crate::input::cmd;
use crate::spec::FieldType;
use crate::{Bottle, Cmd};

#[derive(Clone)]
struct Server {
    bottle: Arc<Mutex<Bottle>>,
}

pub async fn serve(path: &Path, agent: Option<String>, tz: Option<&str>) -> Result<(), Error> {
    let bottle = Bottle::open(path, agent, tz)?;
    let server = Server {
        bottle: Arc::new(Mutex::new(bottle)),
    };
    let running = server
        .serve(stdio())
        .await
        .map_err(|e| Error::Fail(Fail::Io(e.to_string())))?;
    running
        .waiting()
        .await
        .map_err(|e| Error::Fail(Fail::Io(e.to_string())))?;
    Ok(())
}

fn tool_result(result: Result<String, Error>) -> Result<CallToolResult, McpError> {
    match result {
        Ok(body) => Ok(CallToolResult::success(vec![ContentBlock::text(body)])),
        Err(err) => Ok(CallToolResult::error(vec![ContentBlock::text(
            err.to_string(),
        )])),
    }
}

fn pairs(map: HashMap<String, String>) -> Vec<(String, String)> {
    map.into_iter().collect()
}

fn cells(map: HashMap<String, serde_json::Value>) -> Result<Vec<(String, String)>, McpError> {
    map.into_iter().map(|(k, v)| Ok((k, cell(&v)?))).collect()
}

fn cell(value: &serde_json::Value) -> Result<String, McpError> {
    match value {
        serde_json::Value::String(s) => Ok(s.clone()),
        serde_json::Value::Number(n) => Ok(n.to_string()),
        serde_json::Value::Bool(b) => Ok(b.to_string()),
        serde_json::Value::Null => Ok(String::new()),
        _ => Err(McpError::invalid_params(
            "field values must be strings, numbers, or booleans",
            None,
        )),
    }
}

fn field_type(raw: &str) -> Result<FieldType, McpError> {
    match raw {
        "text" => Ok(FieldType::Text),
        "number" => Ok(FieldType::Number),
        "enum" => Ok(FieldType::Enum),
        other => Err(McpError::invalid_params(
            format!("unknown type: {other}"),
            None,
        )),
    }
}

impl Server {
    fn run(&self, cmd: Cmd) -> Result<CallToolResult, McpError> {
        let mut bottle = self
            .bottle
            .lock()
            .map_err(|_| McpError::internal_error("lock poisoned", None))?;
        tool_result(crate::execute(&mut bottle, cmd))
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct HelpParams {
    command: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SchemaShowParams {
    name: String,
    #[serde(default)]
    yaml: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SchemaAddParams {
    name: String,
    file: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SchemaAddFieldParams {
    schema: String,
    name: String,
    #[serde(rename = "type")]
    type_: String,
    values: Option<Vec<String>>,
    default: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SchemaAddValueParams {
    schema: String,
    field: String,
    value: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SchemaNameParams {
    name: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct LogEntry {
    at: Option<String>,
    agent: Option<String>,
    links: Option<HashMap<String, String>>,
    #[serde(flatten)]
    fields: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct LogParams {
    schema: String,
    at: Option<String>,
    agent: Option<String>,
    #[serde(default)]
    links: HashMap<String, String>,
    fields: Option<HashMap<String, serde_json::Value>>,
    entries: Option<Vec<LogEntry>>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct LsParams {
    schema: String,
    from: Option<String>,
    to: Option<String>,
    agent: Option<String>,
    #[serde(rename = "where", default)]
    wheres: HashMap<String, String>,
    #[serde(default)]
    links: HashMap<String, String>,
    #[serde(default)]
    include_ignored: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct IdParams {
    schema: String,
    id: i64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SumParams {
    schema: String,
    field: String,
    from: Option<String>,
    to: Option<String>,
    agent: Option<String>,
    #[serde(rename = "where", default)]
    wheres: HashMap<String, String>,
    #[serde(default)]
    links: HashMap<String, String>,
    group: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct FilterParams {
    schema: String,
    agent: Option<String>,
    #[serde(rename = "where", default)]
    wheres: HashMap<String, String>,
    #[serde(default)]
    links: HashMap<String, String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct AmendParams {
    schema: String,
    id: i64,
    at: Option<String>,
    agent: Option<String>,
    #[serde(default)]
    links: HashMap<String, String>,
    #[serde(default)]
    unlink: Vec<String>,
    #[serde(default)]
    fields: HashMap<String, serde_json::Value>,
}

#[tool_router]
impl Server {
    #[tool(description = "Print the long explanation of a command")]
    fn help(&self, Parameters(p): Parameters<HelpParams>) -> Result<CallToolResult, McpError> {
        tool_result(crate::help(p.command.as_deref()))
    }

    #[tool(description = "List registered schemas")]
    fn schema_list(&self) -> Result<CallToolResult, McpError> {
        self.run(Cmd::SchemaList)
    }

    #[tool(description = "Print the field list of a schema")]
    fn schema_show(
        &self,
        Parameters(p): Parameters<SchemaShowParams>,
    ) -> Result<CallToolResult, McpError> {
        self.run(Cmd::SchemaShow(cmd::SchemaShow {
            name: p.name,
            yaml: p.yaml,
        }))
    }

    #[tool(description = "Register a type from a YAML file")]
    fn schema_add(
        &self,
        Parameters(p): Parameters<SchemaAddParams>,
    ) -> Result<CallToolResult, McpError> {
        self.run(Cmd::SchemaAdd(cmd::SchemaAdd {
            name: p.name,
            file: p.file.into(),
        }))
    }

    #[tool(description = "Add one field to an existing schema")]
    fn schema_add_field(
        &self,
        Parameters(p): Parameters<SchemaAddFieldParams>,
    ) -> Result<CallToolResult, McpError> {
        self.run(Cmd::SchemaAddField(cmd::SchemaAddField {
            schema: p.schema,
            name: p.name,
            type_: field_type(&p.type_)?,
            values: p.values,
            default: p.default,
        }))
    }

    #[tool(description = "Append one value to an enum field")]
    fn schema_add_value(
        &self,
        Parameters(p): Parameters<SchemaAddValueParams>,
    ) -> Result<CallToolResult, McpError> {
        self.run(Cmd::SchemaAddValue(cmd::SchemaAddValue {
            schema: p.schema,
            field: p.field,
            value: p.value,
        }))
    }

    #[tool(description = "Retire a schema so log fails and reads still work")]
    fn schema_retire(
        &self,
        Parameters(p): Parameters<SchemaNameParams>,
    ) -> Result<CallToolResult, McpError> {
        self.run(Cmd::SchemaRetire(cmd::SchemaRetire { name: p.name }))
    }

    #[tool(description = "Drop a schema and its entries")]
    fn schema_drop(
        &self,
        Parameters(p): Parameters<SchemaNameParams>,
    ) -> Result<CallToolResult, McpError> {
        self.run(Cmd::SchemaDrop(cmd::SchemaDrop { name: p.name }))
    }

    #[tool(description = "Write one entry, or many in one transaction")]
    fn log(&self, Parameters(p): Parameters<LogParams>) -> Result<CallToolResult, McpError> {
        match (p.fields, p.entries) {
            (Some(_), Some(_)) => Err(McpError::invalid_params(
                "do not send both fields and entries",
                None,
            )),
            (None, None) => Err(McpError::invalid_params(
                "log requires fields or entries",
                None,
            )),
            (Some(fields), None) => self.run(Cmd::Log(cmd::Log {
                schema: p.schema,
                at: p.at,
                agent: p.agent,
                links: pairs(p.links),
                fields: cells(fields)?,
            })),
            (None, Some(entries)) => {
                if entries.is_empty() {
                    return Err(McpError::invalid_params("entries is empty", None));
                }
                let mut logs = Vec::new();
                for entry in entries {
                    logs.push(cmd::Log {
                        schema: p.schema.clone(),
                        at: entry.at.or_else(|| p.at.clone()),
                        agent: entry.agent.or_else(|| p.agent.clone()),
                        links: pairs(entry.links.unwrap_or_else(|| p.links.clone())),
                        fields: cells(entry.fields)?,
                    });
                }
                let mut bottle = self
                    .bottle
                    .lock()
                    .map_err(|_| McpError::internal_error("lock poisoned", None))?;
                tool_result(crate::log_entries(&mut bottle, logs))
            }
        }
    }

    #[tool(description = "List entries of a schema")]
    fn ls(&self, Parameters(p): Parameters<LsParams>) -> Result<CallToolResult, McpError> {
        self.run(Cmd::Ls(cmd::Ls {
            schema: p.schema,
            from: p.from,
            to: p.to,
            agent: p.agent,
            wheres: pairs(p.wheres),
            links: pairs(p.links),
            include_ignored: p.include_ignored,
        }))
    }

    #[tool(description = "Print one entry by schema and id")]
    fn get(&self, Parameters(p): Parameters<IdParams>) -> Result<CallToolResult, McpError> {
        self.run(Cmd::Get(cmd::Get {
            schema: p.schema,
            id: p.id,
        }))
    }

    #[tool(description = "Total a number field")]
    fn sum(&self, Parameters(p): Parameters<SumParams>) -> Result<CallToolResult, McpError> {
        self.run(Cmd::Sum(cmd::Sum {
            schema: p.schema,
            field: p.field,
            from: p.from,
            to: p.to,
            agent: p.agent,
            wheres: pairs(p.wheres),
            links: pairs(p.links),
            group: p.group,
        }))
    }

    #[tool(description = "Print the most recent entry of a schema")]
    fn last(&self, Parameters(p): Parameters<FilterParams>) -> Result<CallToolResult, McpError> {
        self.run(Cmd::Last(cmd::Last {
            schema: p.schema,
            agent: p.agent,
            wheres: pairs(p.wheres),
            links: pairs(p.links),
        }))
    }

    #[tool(description = "List entries for the current civil day")]
    fn today(&self, Parameters(p): Parameters<FilterParams>) -> Result<CallToolResult, McpError> {
        self.run(Cmd::Today(cmd::Today {
            schema: p.schema,
            agent: p.agent,
            wheres: pairs(p.wheres),
            links: pairs(p.links),
        }))
    }

    #[tool(description = "Change an existing entry in place")]
    fn amend(&self, Parameters(p): Parameters<AmendParams>) -> Result<CallToolResult, McpError> {
        self.run(Cmd::Amend(cmd::Amend {
            schema: p.schema,
            id: p.id,
            at: p.at,
            agent: p.agent,
            links: pairs(p.links),
            unlinks: p.unlink,
            fields: cells(p.fields)?,
        }))
    }

    #[tool(description = "Hide an entry from lists and totals")]
    fn ignore(&self, Parameters(p): Parameters<IdParams>) -> Result<CallToolResult, McpError> {
        self.run(Cmd::Ignore(cmd::Ignore {
            schema: p.schema,
            id: p.id,
        }))
    }
}

#[tool_handler]
impl ServerHandler for Server {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions(include_str!("help/mcp.md"))
    }
}
