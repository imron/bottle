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

use crate::error::{Error, Fail, Usage};
use crate::help as bottle_help;
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
    tz: TimeZone,
}

pub async fn serve(path: &Path, agent: Option<String>, tz: Option<&str>) -> Result<(), Error> {
    let bottle = Bottle::open(path, agent, tz)?;
    let tz = bottle.tz().clone();
    let server = Server {
        bottle: Arc::new(Mutex::new(bottle)),
        tz,
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

fn output(result: Result<String, Error>) -> CallToolResult {
    match result {
        Ok(body) => CallToolResult::success(vec![ContentBlock::text(body)]),
        Err(err) => CallToolResult::error(vec![ContentBlock::text(err.to_string())]),
    }
}

impl Server {
    fn run(&self, op: Result<Op, Error>, style: Style) -> CallToolResult {
        let op = match op {
            Ok(op) => op,
            Err(err) => return output(Err(err)),
        };
        let mut bottle = self.bottle.lock().unwrap_or_else(|e| e.into_inner());
        output(execute(&mut bottle, Request::new(op, style)))
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
struct ScopeParams {
    schema: String,
    agent: Option<String>,
    #[serde(rename = "where", default)]
    wheres: HashMap<String, FieldCell>,
    #[serde(default)]
    links: HashMap<String, String>,
}

impl ScopeParams {
    fn into_scope(self) -> ScopeInput {
        ScopeInput {
            schema: self.schema,
            agent: self.agent,
            wheres: cells(self.wheres),
            links: pairs(self.links),
        }
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct LsParams {
    #[serde(flatten)]
    scope: ScopeParams,
    from: Option<String>,
    to: Option<String>,
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
    #[serde(flatten)]
    scope: ScopeParams,
    field: String,
    from: Option<String>,
    to: Option<String>,
    group: Option<String>,
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
        Ok(output(bottle_help(p.command.as_deref())))
    }

    #[tool(description = "List registered schemas")]
    fn schema_list(&self) -> Result<CallToolResult, McpError> {
        Ok(self.run(Ok(Op::SchemaList), Style::Tsv))
    }

    #[tool(description = "Print the field list of a schema")]
    fn schema_show(
        &self,
        Parameters(p): Parameters<SchemaShowParams>,
    ) -> Result<CallToolResult, McpError> {
        let style = if p.yaml { Style::Yaml } else { Style::Tsv };
        let show = parse_schema_show(p.name);
        Ok(self.run(show.map(Op::SchemaShow), style))
    }

    #[tool(description = "Register a type from a YAML spec")]
    fn schema_add(
        &self,
        Parameters(p): Parameters<SchemaAddParams>,
    ) -> Result<CallToolResult, McpError> {
        let spec = parse_schema_add(p.name, SpecSource::Yaml(p.spec));
        Ok(self.run(spec.map(Op::SchemaAdd), Style::Tsv))
    }

    #[tool(description = "Add one field to an existing schema")]
    fn schema_add_field(
        &self,
        Parameters(p): Parameters<SchemaAddFieldParams>,
    ) -> Result<CallToolResult, McpError> {
        let type_ = p.type_.parse();
        let field = type_
            .and_then(|type_| parse_schema_add_field(p.schema, p.name, type_, p.values, p.default));
        Ok(self.run(field.map(Op::SchemaAddField), Style::Tsv))
    }

    #[tool(description = "Append one value to an enum field")]
    fn schema_add_value(
        &self,
        Parameters(p): Parameters<SchemaAddValueParams>,
    ) -> Result<CallToolResult, McpError> {
        let value = parse_schema_add_value(p.schema, p.field, p.value);
        Ok(self.run(value.map(Op::SchemaAddValue), Style::Tsv))
    }

    #[tool(description = "Retire a schema so log fails and reads still work")]
    fn schema_retire(
        &self,
        Parameters(p): Parameters<SchemaNameParams>,
    ) -> Result<CallToolResult, McpError> {
        let retire = parse_schema_retire(p.name);
        Ok(self.run(retire.map(Op::SchemaRetire), Style::Tsv))
    }

    #[tool(description = "Drop a schema and its entries")]
    fn schema_drop(
        &self,
        Parameters(p): Parameters<SchemaNameParams>,
    ) -> Result<CallToolResult, McpError> {
        let drop = parse_schema_drop(p.name);
        Ok(self.run(drop.map(Op::SchemaDrop), Style::Tsv))
    }

    #[tool(description = "Write one entry, or many in one transaction")]
    fn log(&self, Parameters(p): Parameters<LogParams>) -> Result<CallToolResult, McpError> {
        if p.entries.is_empty() {
            return Ok(self.run(Err(Error::Usage(Usage::EmptyLog)), Style::Tsv));
        }
        let mut logs = Vec::new();
        for entry in p.entries {
            match parse_log(
                p.schema.clone(),
                entry.at,
                entry.agent,
                pairs(entry.links),
                cells(entry.fields),
                &self.tz,
            ) {
                Ok(log) => logs.push(log),
                Err(err) => return Ok(self.run(Err(err), Style::Tsv)),
            }
        }
        Ok(self.run(Ok(Op::Log(logs)), Style::Tsv))
    }

    #[tool(description = "List entries of a schema")]
    fn ls(&self, Parameters(p): Parameters<LsParams>) -> Result<CallToolResult, McpError> {
        let list = parse_ls(
            p.scope.into_scope(),
            p.from,
            p.to,
            p.include_ignored,
            &self.tz,
        );
        Ok(self.run(list.map(Op::List), Style::Tsv))
    }

    #[tool(description = "Print one entry by schema and id")]
    fn get(&self, Parameters(p): Parameters<IdParams>) -> Result<CallToolResult, McpError> {
        let get = parse_get(p.schema, p.id);
        Ok(self.run(get.map(Op::Get), Style::Tsv))
    }

    #[tool(description = "Total a number field")]
    fn sum(&self, Parameters(p): Parameters<SumParams>) -> Result<CallToolResult, McpError> {
        let sum = parse_sum(
            p.scope.into_scope(),
            p.field,
            p.from,
            p.to,
            p.group,
            &self.tz,
        );
        Ok(self.run(sum.map(Op::Sum), Style::Tsv))
    }

    #[tool(description = "Print the most recent entry of a schema")]
    fn last(&self, Parameters(p): Parameters<ScopeParams>) -> Result<CallToolResult, McpError> {
        let last = parse_last(p.into_scope());
        Ok(self.run(last.map(Op::Last), Style::Tsv))
    }

    #[tool(description = "List entries for the current civil day")]
    fn today(&self, Parameters(p): Parameters<ScopeParams>) -> Result<CallToolResult, McpError> {
        let today = parse_today(p.into_scope());
        Ok(self.run(today.map(Op::Today), Style::Tsv))
    }

    #[tool(description = "Change an existing entry in place")]
    fn amend(&self, Parameters(p): Parameters<AmendParams>) -> Result<CallToolResult, McpError> {
        let amend = parse_amend(
            AmendInput {
                schema: p.schema,
                id: p.id,
                at: p.at,
                agent: p.agent,
                links: pairs(p.links),
                unlinks: p.unlink,
                fields: cells(p.fields),
            },
            &self.tz,
        );
        Ok(self.run(amend.map(Op::Amend), Style::Tsv))
    }

    #[tool(description = "Hide an entry from lists and totals")]
    fn ignore(&self, Parameters(p): Parameters<IdParams>) -> Result<CallToolResult, McpError> {
        let ignore = parse_ignore(p.schema, p.id);
        Ok(self.run(ignore.map(Op::Ignore), Style::Tsv))
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
    use rmcp::schemars;

    use super::LsParams;

    #[test]
    fn json_number_keeps_decimal_text() {
        let n: serde_json::Number = serde_json::from_str("1.10").unwrap();
        assert_eq!(n.to_string(), "1.10");
        let n: serde_json::Number = serde_json::from_str("9007199254740993").unwrap();
        assert_eq!(n.to_string(), "9007199254740993");
    }

    #[test]
    fn ls_flattens_scope_and_rejects_unknown() {
        let p: LsParams =
            serde_json::from_str(r#"{"schema":"meal","from":"2026-08-22","where":{"kcal":1}}"#)
                .unwrap();
        assert_eq!(p.scope.schema, "meal");
        assert_eq!(p.from.as_deref(), Some("2026-08-22"));
        assert!(p.scope.wheres.contains_key("kcal"));
        let err = serde_json::from_str::<LsParams>(r#"{"schema":"meal","nope":true}"#).unwrap_err();
        assert!(
            err.to_string().contains("nope") || err.to_string().contains("unknown"),
            "{err}"
        );
        let schema = serde_json::to_value(schemars::schema_for!(LsParams)).unwrap();
        let keys = schema["properties"].as_object().unwrap();
        assert!(keys.contains_key("schema"), "{schema}");
        assert!(keys.contains_key("from"), "{schema}");
        assert!(!keys.contains_key("scope"), "{schema}");
    }
}
