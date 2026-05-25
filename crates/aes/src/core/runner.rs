use std::{io::Write, path::Path, process::ExitCode};

use crate::{
    cli::{Command, Run},
    context::Context,
    core::reporter::{DiagnosticRenderer, spawn_reporter},
};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum RunResult {
    Succeeded,
    CompilationFailed,
    UsageError,
    IoError,
    BackendError,
}

impl RunResult {
    fn exit_code(&self) -> u8 {
        match self {
            RunResult::Succeeded => 0,
            RunResult::CompilationFailed => 1,
            RunResult::UsageError => 2,
            RunResult::IoError => 3,
            RunResult::BackendError => 4,
        }
    }
}

impl std::process::Termination for RunResult {
    fn report(self) -> ExitCode {
        ExitCode::from(self.exit_code())
    }
}

pub async fn run<W: Write + Send + 'static>(
    command: Command,
    cwd: &Path,
    stdout: &mut impl Write,
    stderr: W,
) -> RunResult {
    let (reporter, actor_join_handle) = spawn_reporter(DiagnosticRenderer::default(), stderr);

    let result = command
        .run(&mut Context {
            cwd,
            stdout,
            stderr: &mut std::io::sink(),
            reporter: reporter.clone(),
        })
        .await;

    // Drop the handle so the actor can finish.
    drop(reporter);

    let report_result = actor_join_handle.await;

    match result {
        Ok(RunResult::Succeeded) => match report_result {
            Ok(Ok(report)) => {
                if report.error_count > 0 {
                    RunResult::CompilationFailed
                } else {
                    RunResult::Succeeded
                }
            }
            _ => RunResult::IoError,
        },
        Ok(res) => res,
        Err(_) => RunResult::IoError,
    }
}
