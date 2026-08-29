use std::path::PathBuf;
use std::process::ExitCode;

use bottle::{Bottle, Cmd, Error, execute, help, parse, zone};
use clap::{Parser, Subcommand};

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
                .build()?;
            rt.block_on(bottle::mcp(&db, None, None))?;
            Ok(String::new())
        }
        Command::Ledger(command) => {
            let db = match cli.db {
                Some(path) => path,
                None => bottle::default_db_path()?,
            };
            let tz = zone(None)?;
            let style = bottle::style(&command);
            let request = parse(command, &tz)?;
            let mut bottle = Bottle::open(&db, None, None)?;
            execute(&mut bottle, request, style)
        }
    }
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
    Help { topic: Vec<String> },
    /// Run bottle as an MCP server on stdio
    Mcp,
    #[command(flatten)]
    Ledger(Cmd),
}
