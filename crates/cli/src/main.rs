use clap::Parser;
use std::env;
use std::process::ExitCode;

use cargo_runner::cli::{Cargo, Command, Runner};
use cargo_runner::display::ide_json::ErrorOutput;
use cargo_runner::display::style;

fn main() -> ExitCode {
    // Initialize tracing based on RUST_LOG env var
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            if style::json_error_mode() {
                // IDE contract: structured error on stdout (parseable); details also on stderr.
                let out = ErrorOutput::from_anyhow(&err);
                println!("{}", out.to_json_string());
                eprintln!("{}", out.message);
            } else {
                eprintln!("Error: {err:#}");
            }
            ExitCode::from(1)
        }
    }
}

fn run() -> anyhow::Result<()> {
    let runner = parse_runner();
    style::init(runner.quiet, runner.no_emoji);
    runner.command.execute()
}

fn parse_runner() -> Runner {
    let args: Vec<String> = env::args().collect();

    // When cargo invokes a subcommand, it looks for cargo-<subcommand> binary
    // and passes the subcommand name as the first argument (`runner`).
    if args.get(1).is_some_and(|arg| arg == "runner") {
        let cargo = Cargo::parse();
        let Command::Runner(runner) = cargo.command;
        runner
    } else {
        Runner::parse()
    }
}
