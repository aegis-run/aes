use aes_foundation::Reporter;
use std::path::PathBuf;

use clap::{Args, ValueEnum};

use crate::{cli::Run, context::Context, core::runner::RunResult, errors, fs};

/// Compile and dump the schema IR representation locally
#[derive(Debug, Args)]
pub struct DumpArgs {
    /// Path to the .aes schema source file
    pub path: PathBuf,

    /// Output format for the serialized schema representation
    #[arg(long, value_enum, default_value = "proto")]
    pub format: DumpFormat,

    /// Optional file path to write the output to (defaults to stdout)
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

/// Supported local export formats
#[derive(Debug, Clone, Copy, Eq, PartialEq, ValueEnum)]
pub enum DumpFormat {
    /// Protobuf binary format (canonical IR representation)
    Proto,
    /// Human-readable Rust debug representation
    Debug,
}

impl Run for DumpArgs {
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

        match self.format {
            DumpFormat::Proto => {
                let bytes = schema.encode_bytes();
                if let Some(out) = &self.output {
                    let resolved_out = fs::resolve_path(out, ctx.cwd);
                    std::fs::write(resolved_out, bytes)?;
                } else {
                    ctx.stdout.write_all(&bytes)?;
                }
            }
            DumpFormat::Debug => {
                if let Some(out) = &self.output {
                    let resolved_out = fs::resolve_path(out, ctx.cwd);
                    std::fs::write(resolved_out, format!("{:#?}\n", schema))?;
                } else {
                    writeln!(ctx.stdout, "{:#?}", schema)?;
                }
            }
        }

        let elapsed = start_time.elapsed();

        let types_count = schema.types.len();
        let relations_count = schema
            .types
            .iter()
            .map(|t| t.relations.len())
            .sum::<usize>();
        let permissions_count = schema
            .types
            .iter()
            .map(|t| t.permissions.len())
            .sum::<usize>();

        let bold_green = anstyle::Style::new()
            .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::BrightGreen)))
            .effects(anstyle::Effects::BOLD);

        let cyan =
            anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Cyan)));

        let reset = anstyle::Reset;

        writeln!(
            ctx.stdout,
            "{bold_green}   Compiled{reset} schema in {cyan}{elapsed:.2?}{reset} \
             (1 file, {cyan}{types_count} types{reset}, \
             {cyan}{relations_count} relations{reset}, \
             {cyan}{permissions_count} permissions{reset})",
        )?;

        Ok(RunResult::Succeeded)
    }
}
