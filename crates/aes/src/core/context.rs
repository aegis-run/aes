use std::{io::Write, path::Path};

use crate::core::reporter::{Reporter, SourceAwareReporter};

pub struct Context<'a> {
    pub cwd: &'a Path,
    pub stdout: &'a mut dyn Write,
    pub stderr: &'a mut dyn Write,
    pub reporter: Reporter,
}

impl Context<'_> {
    /// Creates a source-aware reporter for the given file, using the context's CWD.
    pub fn reporter_for<'a>(&self, path: &'a Path, source_text: &'a str) -> SourceAwareReporter {
        self.reporter.for_file(Some(self.cwd), path, source_text)
    }
}
