use std::io::IsTerminal;
use std::path::PathBuf;

use aes_foundation::Reporter;
use clap::Args;

use crate::{cli::Run, context::Context, core::runner::RunResult, errors, fs};

/// Validate and publish the schema to the remote Aegis backend server
#[derive(Debug, Args)]
pub struct ExportArgs {
    /// Path to the .aes schema source file
    pub path: PathBuf,

    /// Remote Aegis gRPC server address
    #[arg(long, default_value = "http://127.0.0.1:50051")]
    pub server: String,

    /// Pre-Shared Key (PSK) credential for server authentication
    #[arg(long, short, env = "AEGIS_KEY")]
    pub key: Option<String>,
}

impl Run for ExportArgs {
    async fn run(self, ctx: &mut Context<'_>) -> std::io::Result<RunResult> {
        let start_time = std::time::Instant::now();

        let path = fs::resolve_path(&self.path, ctx.cwd);

        let Ok(source) = fs::read(&path) else {
            ctx.reporter_for(&path, "")
                .report(errors::failed_to_read_file(&path));
            return Ok(RunResult::IoError);
        };

        let mut reporter = ctx.reporter_for(&path, &source);

        let mut compiler = aes_compiler::Compiler::default();
        let file_id = compiler.add_file(&path, source);

        let Some(schema) = compiler.export_schema(file_id, &mut reporter) else {
            return Ok(RunResult::CompilationFailed);
        };

        let mut client = match aes_ir::Client::connect(self.server.clone()).await {
            Ok(c) => c.with_token(self.key),
            Err(err) => {
                reporter.report(errors::failed_to_connect_backend(&self.server, err));
                return Ok(RunResult::BackendError);
            }
        };

        let hash = match client.write(schema).await {
            Ok(hash) => hash,
            Err(err) => {
                reporter.report(errors::failed_to_publish_schema(err));
                return Ok(RunResult::BackendError);
            }
        };

        // Print success revision token to stdout
        writeln!(ctx.stdout, "schema exported: {}", hash.digest)?;

        let elapsed = start_time.elapsed();
        let use_color = std::io::stderr().is_terminal();

        if use_color {
            let bold_green = anstyle::Style::new()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::BrightGreen)))
                .effects(anstyle::Effects::BOLD);
            let cyan = anstyle::Style::new()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Cyan)));
            let reset = anstyle::Reset;

            writeln!(
                ctx.stderr,
                "{bold_green}   Exported{reset} schema revision {cyan}{}{reset} to {cyan}{}{reset} in {cyan}{:.2?}{reset}",
                hash.digest,
                self.server,
                elapsed,
                bold_green = bold_green,
                cyan = cyan,
                reset = reset,
            )?;
        } else {
            writeln!(
                ctx.stderr,
                "   Exported schema revision {} to {} in {:.2?}",
                hash.digest, self.server, elapsed,
            )?;
        }

        Ok(RunResult::Succeeded)
    }
}
