pub mod cli;
pub mod core;
pub mod errors;

use std::io::Write;

use clap::Parser;

pub use core::init::{EnvProcessInit, ProcessInit};
pub use core::runner::RunResult;

pub use core::*;

pub async fn run(init: impl ProcessInit) -> RunResult {
    let args = init.args();
    match cli::Args::try_parse_from(args) {
        Ok(args) => {
            let cwd = init.cwd().to_path_buf();
            let (mut stdout, stderr) = init.take_streams();

            runner::run(args.command, &cwd, &mut stdout, stderr).await
        }
        Err(err) => {
            let (mut stdout, mut stderr) = init.take_streams();

            let is_help_or_missing = matches!(
                err.kind(),
                clap::error::ErrorKind::DisplayHelp
                    | clap::error::ErrorKind::MissingSubcommand
                    | clap::error::ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
            );

            if err.use_stderr() {
                if is_help_or_missing {
                    let _ = cli::print_banner(&mut stderr);
                }
                write!(stderr, "{}", err.render().ansi())
            } else {
                if is_help_or_missing {
                    let _ = cli::print_banner(&mut stdout);
                }
                write!(stdout, "{}", err.render().ansi())
            }
            .map(|_| RunResult::UsageError)
            .unwrap_or(RunResult::IoError)
        }
    }
}
