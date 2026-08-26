use std::path::PathBuf;
use std::process::ExitCode;

use bottle::{Cmd, Error, FieldType, cmd, help, run};
use clap::{Args, Parser, Subcommand};

fn main() -> ExitCode {
    match run_cli(Cli::parse()) {
        Ok(out) => {
            print!("{out}");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("{err}");
            ExitCode::from(err.exit_code() as u8)
        }
    }
}

fn run_cli(cli: Cli) -> Result<String, Error> {
    match cli.command {
        Command::Help { topic } => {
            let topic = if topic.is_empty() {
                None
            } else {
                Some(topic.join(" "))
            };
            help(topic.as_deref())
        }
        Command::Mcp => {
            let db = match cli.db {
                Some(path) => path,
                None => bottle::default_db_path()?,
            };
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| bottle::Error::Fail(bottle::Fail::Io(e.to_string())))?;
            rt.block_on(bottle::mcp(&db, None, None))?;
            Ok(String::new())
        }
        command => {
            let db = match cli.db {
                Some(path) => path,
                None => bottle::default_db_path()?,
            };
            run(Some(&db), None, None, command.into_cmd())
        }
    }
}

fn parse_kv(s: &str) -> Result<(String, String), String> {
    match s.split_once('=') {
        Some((name, value)) if !name.is_empty() => Ok((name.to_string(), value.to_string())),
        _ => Err("expected name=value".into()),
    }
}

fn parse_field_type(s: &str) -> Result<FieldType, String> {
    match s {
        "text" => Ok(FieldType::Text),
        "number" => Ok(FieldType::Number),
        "enum" => Ok(FieldType::Enum),
        other => Err(format!("unknown type: {other}")),
    }
}

fn parse_values(s: &str) -> Result<Vec<String>, std::convert::Infallible> {
    Ok(s.split(',').map(str::to_string).collect())
}

#[derive(Parser)]
#[command(
    name = "bottle",
    about = "a store for events",
    disable_help_subcommand = true
)]
struct Cli {
    #[arg(long, global = true)]
    db: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Print the long explanation of a command
    Help {
        topic: Vec<String>,
    },
    /// Run bottle as an MCP server on stdio
    Mcp,
    #[command(subcommand)]
    Schema(SchemaCommand),
    Log(LogArgs),
    Ls(LsArgs),
    Get(GetArgs),
    Sum(SumArgs),
    Last(Filters),
    Today(Filters),
    Amend(AmendArgs),
    Ignore(IgnoreArgs),
}

#[derive(Subcommand)]
enum SchemaCommand {
    List,
    Show {
        name: String,
        #[arg(long)]
        yaml: bool,
    },
    Add {
        name: String,
        #[arg(long, required = true)]
        file: PathBuf,
    },
    #[command(name = "add-field")]
    AddField {
        schema: String,
        #[arg(long)]
        name: String,
        #[arg(long = "type", value_parser = parse_field_type)]
        type_: FieldType,
        #[arg(long, value_parser = parse_values)]
        values: Option<Vec<String>>,
        #[arg(long)]
        default: Option<String>,
    },
    #[command(name = "add-value")]
    AddValue {
        schema: String,
        #[arg(long)]
        field: String,
        #[arg(long)]
        value: String,
    },
    Retire {
        name: String,
    },
    Drop {
        name: String,
    },
}

#[derive(Args)]
struct Filters {
    schema: String,
    #[arg(long)]
    agent: Option<String>,
    #[arg(long = "where", value_name = "field=value", value_parser = parse_kv)]
    wheres: Vec<(String, String)>,
    #[arg(long = "link", value_name = "name=SCHEMA/ID", value_parser = parse_kv)]
    links: Vec<(String, String)>,
}

#[derive(Args)]
struct LogArgs {
    schema: String,
    #[arg(long)]
    at: Option<String>,
    #[arg(long)]
    agent: Option<String>,
    #[arg(long = "link", value_name = "name=SCHEMA/ID", value_parser = parse_kv)]
    links: Vec<(String, String)>,
    #[arg(trailing_var_arg = true, value_parser = parse_kv)]
    fields: Vec<(String, String)>,
}

#[derive(Args)]
struct LsArgs {
    #[command(flatten)]
    filters: Filters,
    #[arg(long)]
    from: Option<String>,
    #[arg(long)]
    to: Option<String>,
    #[arg(long)]
    include_ignored: bool,
}

#[derive(Args)]
struct GetArgs {
    schema: String,
    id: i64,
}

#[derive(Args)]
struct SumArgs {
    #[command(flatten)]
    filters: Filters,
    field: String,
    #[arg(long)]
    from: Option<String>,
    #[arg(long)]
    to: Option<String>,
    #[arg(long)]
    group: Option<String>,
}

#[derive(Args)]
struct AmendArgs {
    schema: String,
    id: i64,
    #[arg(long)]
    at: Option<String>,
    #[arg(long)]
    agent: Option<String>,
    #[arg(long = "link", value_name = "name=SCHEMA/ID", value_parser = parse_kv)]
    links: Vec<(String, String)>,
    #[arg(long = "unlink")]
    unlinks: Vec<String>,
    #[arg(trailing_var_arg = true, value_parser = parse_kv)]
    fields: Vec<(String, String)>,
}

#[derive(Args)]
struct IgnoreArgs {
    schema: String,
    id: i64,
}

impl Command {
    fn into_cmd(self) -> Cmd {
        match self {
            Self::Help { .. } | Self::Mcp => {
                unreachable!("help and mcp are handled before into_cmd")
            }
            Self::Schema(SchemaCommand::List) => Cmd::SchemaList,
            Self::Schema(SchemaCommand::Show { name, yaml }) => {
                Cmd::SchemaShow(cmd::SchemaShow { name, yaml })
            }
            Self::Schema(SchemaCommand::Add { name, file }) => {
                Cmd::SchemaAdd(cmd::SchemaAdd { name, file })
            }
            Self::Schema(SchemaCommand::AddField {
                schema,
                name,
                type_,
                values,
                default,
            }) => Cmd::SchemaAddField(cmd::SchemaAddField {
                schema,
                name,
                type_,
                values,
                default,
            }),
            Self::Schema(SchemaCommand::AddValue {
                schema,
                field,
                value,
            }) => Cmd::SchemaAddValue(cmd::SchemaAddValue {
                schema,
                field,
                value,
            }),
            Self::Schema(SchemaCommand::Retire { name }) => {
                Cmd::SchemaRetire(cmd::SchemaRetire { name })
            }
            Self::Schema(SchemaCommand::Drop { name }) => Cmd::SchemaDrop(cmd::SchemaDrop { name }),
            Self::Log(args) => Cmd::Log(cmd::Log {
                schema: args.schema,
                at: args.at,
                agent: args.agent,
                links: args.links,
                fields: args.fields,
            }),
            Self::Ls(args) => Cmd::Ls(cmd::Ls {
                schema: args.filters.schema,
                from: args.from,
                to: args.to,
                agent: args.filters.agent,
                wheres: args.filters.wheres,
                links: args.filters.links,
                include_ignored: args.include_ignored,
            }),
            Self::Get(args) => Cmd::Get(cmd::Get {
                schema: args.schema,
                id: args.id,
            }),
            Self::Sum(args) => Cmd::Sum(cmd::Sum {
                schema: args.filters.schema,
                field: args.field,
                from: args.from,
                to: args.to,
                agent: args.filters.agent,
                wheres: args.filters.wheres,
                links: args.filters.links,
                group: args.group,
            }),
            Self::Last(filters) => Cmd::Last(cmd::Last {
                schema: filters.schema,
                agent: filters.agent,
                wheres: filters.wheres,
                links: filters.links,
            }),
            Self::Today(filters) => Cmd::Today(cmd::Today {
                schema: filters.schema,
                agent: filters.agent,
                wheres: filters.wheres,
                links: filters.links,
            }),
            Self::Amend(args) => Cmd::Amend(cmd::Amend {
                schema: args.schema,
                id: args.id,
                at: args.at,
                agent: args.agent,
                links: args.links,
                unlinks: args.unlinks,
                fields: args.fields,
            }),
            Self::Ignore(args) => Cmd::Ignore(cmd::Ignore {
                schema: args.schema,
                id: args.id,
            }),
        }
    }
}
