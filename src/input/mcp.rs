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
use crate::ledger::Op;
use crate::{Bottle, Request, Style, execute};
use rmcp::service::ServerInitializeError;

use super::{
    AmendInput, ScopeInput, SpecSource, amend as parse_amend, get as parse_get,
    ignore as parse_ignore, last as parse_last, log as parse_log, ls as parse_ls,
    schema_add as parse_schema_add, schema_add_field as parse_schema_add_field,
    schema_add_value as parse_schema_add_value, schema_drop as parse_schema_drop,
    schema_retire as parse_schema_retire, schema_show as parse_schema_show, sum as parse_sum,
    today as parse_today,
};
use jiff::tz::TimeZone;

fn pairs(map: HashMap<String, String>) -> Vec<(String, String)> {
    map.into_iter().collect()
}

#[derive(Debug, schemars::JsonSchema)]
#[serde(untagged)]
enum FieldCell {
    String(String),
    Number(serde_json::Number),
    Null,
}

impl<'de> Deserialize<'de> for FieldCell {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        match value {
            serde_json::Value::String(s) => Ok(Self::String(s)),
            serde_json::Value::Number(n) => Ok(Self::Number(n)),
            serde_json::Value::Null => Ok(Self::Null),
            _ => Err(serde::de::Error::custom(
                "field values must be strings or numbers",
            )),
        }
    }
}

fn cells(map: HashMap<String, FieldCell>) -> Vec<(String, String)> {
    map.into_iter()
        .map(|(k, v)| {
            let value = match v {
                FieldCell::String(s) => s,
                FieldCell::Number(n) => n.to_string(),
                FieldCell::Null => String::new(),
            };
            (k, value)
        })
        .collect()
}

#[derive(Clone)]
struct Server {
    bottle: Arc<Mutex<Bottle>>,
}

pub async fn serve(path: &Path, agent: Option<String>, tz: Option<&str>) -> Result<(), Error> {
    let bottle = Bottle::open(path, agent, tz)?;
    let server = Server {
        bottle: Arc::new(Mutex::new(bottle)),
    };
    let running = server.serve(stdio()).await?;
    running.waiting().await?;
    Ok(())
}

impl From<ServerInitializeError> for Error {
    fn from(err: ServerInitializeError) -> Self {
        Self::Fail(Fail::Io(err.to_string()))
    }
}

impl From<tokio::task::JoinError> for Error {
    fn from(err: tokio::task::JoinError) -> Self {
        Self::Fail(Fail::Io(err.to_string()))
    }
}

fn tool_result(result: Result<String, Error>) -> Result<CallToolResult, McpError> {
    match result {
        Ok(body) => Ok(CallToolResult::success(vec![ContentBlock::text(body)])),
        Err(err) => Ok(CallToolResult::error(vec![ContentBlock::text(
            err.to_string(),
        )])),
    }
}

impl Server {
    fn execute_parsed(
        &self,
        f: impl FnOnce(&TimeZone) -> Result<Request, Error>,
    ) -> Result<CallToolResult, McpError> {
        let mut bottle = self.bottle.lock().unwrap_or_else(|e| e.into_inner());
        match f(bottle.tz()) {
            Ok(request) => tool_result(execute(&mut bottle, request)),
            Err(err) => tool_result(Err(err)),
        }
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct HelpParams {
    command: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct SchemaShowParams {
    name: String,
    #[serde(default)]
    yaml: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct SchemaAddParams {
    name: String,
    spec: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct SchemaAddFieldParams {
    schema: String,
    name: String,
    #[serde(rename = "type")]
    type_: String,
    values: Option<Vec<String>>,
    default: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct SchemaAddValueParams {
    schema: String,
    field: String,
    value: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct SchemaNameParams {
    name: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct LogEntry {
    at: Option<String>,
    agent: Option<String>,
    #[serde(default)]
    links: HashMap<String, String>,
    #[serde(flatten)]
    fields: HashMap<String, FieldCell>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct LogParams {
    schema: String,
    entries: Vec<LogEntry>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
struct IdParams {
    schema: String,
    id: i64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
struct FilterParams {
    schema: String,
    agent: Option<String>,
    #[serde(rename = "where", default)]
    wheres: HashMap<String, String>,
    #[serde(default)]
    links: HashMap<String, String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
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
    fields: HashMap<String, FieldCell>,
}

#[tool_router]
impl Server {
    #[tool(description = "Print the long explanation of a command")]
    fn help(&self, Parameters(p): Parameters<HelpParams>) -> Result<CallToolResult, McpError> {
        tool_result(crate::help(p.command.as_deref()))
    }

    #[tool(description = "List registered schemas")]
    fn schema_list(&self) -> Result<CallToolResult, McpError> {
        self.execute_parsed(|_| Ok(Request::new(Op::SchemaList, Style::Tsv)))
    }

    #[tool(description = "Print the field list of a schema")]
    fn schema_show(
        &self,
        Parameters(p): Parameters<SchemaShowParams>,
    ) -> Result<CallToolResult, McpError> {
        let style = if p.yaml { Style::Yaml } else { Style::Tsv };
        self.execute_parsed(|_| {
            Ok(Request::new(
                Op::SchemaShow(parse_schema_show(p.name)?),
                style,
            ))
        })
    }

    #[tool(description = "Register a type from a YAML spec")]
    fn schema_add(
        &self,
        Parameters(p): Parameters<SchemaAddParams>,
    ) -> Result<CallToolResult, McpError> {
        self.execute_parsed(|_| {
            Ok(Request::new(
                Op::SchemaAdd(parse_schema_add(p.name, SpecSource::Yaml(p.spec))?),
                Style::Tsv,
            ))
        })
    }

    #[tool(description = "Add one field to an existing schema")]
    fn schema_add_field(
        &self,
        Parameters(p): Parameters<SchemaAddFieldParams>,
    ) -> Result<CallToolResult, McpError> {
        self.execute_parsed(|_| {
            let type_ = p.type_.parse()?;
            Ok(Request::new(
                Op::SchemaAddField(parse_schema_add_field(
                    p.schema, p.name, type_, p.values, p.default,
                )?),
                Style::Tsv,
            ))
        })
    }

    #[tool(description = "Append one value to an enum field")]
    fn schema_add_value(
        &self,
        Parameters(p): Parameters<SchemaAddValueParams>,
    ) -> Result<CallToolResult, McpError> {
        self.execute_parsed(|_| {
            Ok(Request::new(
                Op::SchemaAddValue(parse_schema_add_value(p.schema, p.field, p.value)?),
                Style::Tsv,
            ))
        })
    }

    #[tool(description = "Retire a schema so log fails and reads still work")]
    fn schema_retire(
        &self,
        Parameters(p): Parameters<SchemaNameParams>,
    ) -> Result<CallToolResult, McpError> {
        self.execute_parsed(|_| {
            Ok(Request::new(
                Op::SchemaRetire(parse_schema_retire(p.name)?),
                Style::Tsv,
            ))
        })
    }

    #[tool(description = "Drop a schema and its entries")]
    fn schema_drop(
        &self,
        Parameters(p): Parameters<SchemaNameParams>,
    ) -> Result<CallToolResult, McpError> {
        self.execute_parsed(|_| {
            Ok(Request::new(
                Op::SchemaDrop(parse_schema_drop(p.name)?),
                Style::Tsv,
            ))
        })
    }

    #[tool(description = "Write one entry, or many in one transaction")]
    fn log(&self, Parameters(p): Parameters<LogParams>) -> Result<CallToolResult, McpError> {
        if p.entries.is_empty() {
            return Err(McpError::invalid_params("entries is empty", None));
        }
        self.execute_parsed(|tz| {
            let mut logs = Vec::new();
            for entry in p.entries {
                logs.push(parse_log(
                    p.schema.clone(),
                    entry.at,
                    entry.agent,
                    pairs(entry.links),
                    cells(entry.fields),
                    tz,
                )?);
            }
            Ok(Request::new(Op::Log(logs), Style::Tsv))
        })
    }

    #[tool(description = "List entries of a schema")]
    fn ls(&self, Parameters(p): Parameters<LsParams>) -> Result<CallToolResult, McpError> {
        self.execute_parsed(|tz| {
            Ok(Request::new(
                Op::List(parse_ls(
                    ScopeInput {
                        schema: p.schema,
                        agent: p.agent,
                        wheres: pairs(p.wheres),
                        links: pairs(p.links),
                    },
                    p.from,
                    p.to,
                    p.include_ignored,
                    tz,
                )?),
                Style::Tsv,
            ))
        })
    }

    #[tool(description = "Print one entry by schema and id")]
    fn get(&self, Parameters(p): Parameters<IdParams>) -> Result<CallToolResult, McpError> {
        self.execute_parsed(|_| {
            Ok(Request::new(
                Op::Get(parse_get(p.schema, p.id)?),
                Style::Tsv,
            ))
        })
    }

    #[tool(description = "Total a number field")]
    fn sum(&self, Parameters(p): Parameters<SumParams>) -> Result<CallToolResult, McpError> {
        self.execute_parsed(|tz| {
            Ok(Request::new(
                Op::Sum(parse_sum(
                    ScopeInput {
                        schema: p.schema,
                        agent: p.agent,
                        wheres: pairs(p.wheres),
                        links: pairs(p.links),
                    },
                    p.field,
                    p.from,
                    p.to,
                    p.group,
                    tz,
                )?),
                Style::Tsv,
            ))
        })
    }

    #[tool(description = "Print the most recent entry of a schema")]
    fn last(&self, Parameters(p): Parameters<FilterParams>) -> Result<CallToolResult, McpError> {
        self.execute_parsed(|_| {
            Ok(Request::new(
                Op::Last(parse_last(ScopeInput {
                    schema: p.schema,
                    agent: p.agent,
                    wheres: pairs(p.wheres),
                    links: pairs(p.links),
                })?),
                Style::Tsv,
            ))
        })
    }

    #[tool(description = "List entries for the current civil day")]
    fn today(&self, Parameters(p): Parameters<FilterParams>) -> Result<CallToolResult, McpError> {
        self.execute_parsed(|_| {
            Ok(Request::new(
                Op::Today(parse_today(ScopeInput {
                    schema: p.schema,
                    agent: p.agent,
                    wheres: pairs(p.wheres),
                    links: pairs(p.links),
                })?),
                Style::Tsv,
            ))
        })
    }

    #[tool(description = "Change an existing entry in place")]
    fn amend(&self, Parameters(p): Parameters<AmendParams>) -> Result<CallToolResult, McpError> {
        self.execute_parsed(|tz| {
            Ok(Request::new(
                Op::Amend(parse_amend(
                    AmendInput {
                        schema: p.schema,
                        id: p.id,
                        at: p.at,
                        agent: p.agent,
                        links: pairs(p.links),
                        unlinks: p.unlink,
                        fields: cells(p.fields),
                    },
                    tz,
                )?),
                Style::Tsv,
            ))
        })
    }

    #[tool(description = "Hide an entry from lists and totals")]
    fn ignore(&self, Parameters(p): Parameters<IdParams>) -> Result<CallToolResult, McpError> {
        self.execute_parsed(|_| {
            Ok(Request::new(
                Op::Ignore(parse_ignore(p.schema, p.id)?),
                Style::Tsv,
            ))
        })
    }
}

#[tool_handler]
impl ServerHandler for Server {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions(include_str!("help/mcp.md"))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn json_number_keeps_decimal_text() {
        let n: serde_json::Number = serde_json::from_str("1.10").unwrap();
        assert_eq!(n.to_string(), "1.10");
        let n: serde_json::Number = serde_json::from_str("9007199254740993").unwrap();
        assert_eq!(n.to_string(), "9007199254740993");
    }
}
