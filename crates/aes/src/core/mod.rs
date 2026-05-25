pub mod context;
pub mod fs;
pub mod init;
pub mod reporter;
pub mod runner;

pub type Result = std::result::Result<runner::RunResult, runner::RunResult>;
