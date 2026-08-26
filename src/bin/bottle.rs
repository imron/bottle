use std::path::PathBuf;
use std::process::ExitCode;

use bottle::{Cmd, Error, help, run};
use clap::Parser;

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
        Cmd::Help { topic } => {
            let topic = if topic.is_empty() {
                None
            } else {
                Some(topic.join(" "))
            };
            help(topic.as_deref())
        }
        Cmd::Mcp => {
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
            run(Some(&db), None, None, command)
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
    command: Cmd,
}
