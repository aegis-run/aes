use std::io::{IsTerminal, Write};
use std::sync::Arc;

use aes_foundation::{
    Diagnostic, Error, GraphicalReportHandler, GraphicalTheme, NamedSource, Severity,
    diagnostic::DiagnosticSource,
};
use tokio::sync::mpsc;

/// Messages sent to the reporter actor.
pub enum ReporterMessage {
    Report(Error),
    ReportBatch(Vec<Error>),
}

/// The public interface to the reporter actor.
///
/// `Reporter` is a lightweight handle used to send enriched [`miette::Error`]
/// diagnostics to the background reporter actor.
#[derive(Clone)]
pub struct Reporter {
    sender: mpsc::UnboundedSender<ReporterMessage>,
}

impl Reporter {
    /// Sends a single enriched error to the actor.
    pub fn report(&self, error: Error) -> Result<(), ReporterSendError> {
        self.sender
            .send(ReporterMessage::Report(error))
            .map_err(|_| ReporterSendError::ReceiverClosed)
    }

    /// Sends a batch of enriched errors to the actor.
    pub fn report_batch(&self, errors: Vec<Error>) -> Result<(), ReporterSendError> {
        self.sender
            .send(ReporterMessage::ReportBatch(errors))
            .map_err(|_| ReporterSendError::ReceiverClosed)
    }

    /// Creates a source-aware reporter adapter for a specific source context.
    ///
    /// The returned [`SourceAwareReporter`] implements [`aes_foundation::Reporter`]
    /// and can be used with synchronous compiler passes.
    pub fn with_source(&self, source: DiagnosticSource<'_>) -> SourceAwareReporter {
        SourceAwareReporter {
            reporter: self.clone(),
            source: Arc::new(source.to_named_source()),
            has_errors: false,
        }
    }

    /// A convenience method to create a source-aware reporter for a specific file.
    pub fn for_file<'a>(
        &self,
        cwd: Option<&'a std::path::Path>,
        path: &'a std::path::Path,
        source_text: &'a str,
    ) -> SourceAwareReporter {
        self.with_source(DiagnosticSource {
            cwd,
            path,
            source_text,
        })
    }
}

/// An adapter that bridges synchronous [`Diagnostic`] events to the asynchronous
/// [`Reporter`] by enriching them with a fixed source context.
pub struct SourceAwareReporter {
    reporter: Reporter,
    source: Arc<NamedSource<String>>,
    has_errors: bool,
}

impl SourceAwareReporter {
    /// Returns the underlying reporter handle.
    pub fn handle(&self) -> Reporter {
        self.reporter.clone()
    }
}

impl aes_foundation::Reporter for SourceAwareReporter {
    fn report(&mut self, diagnostic: Diagnostic) {
        if diagnostic.is_error() {
            self.has_errors = true;
        }

        let error = diagnostic.enrich_with_source(Arc::clone(&self.source));
        let _ = self.reporter.report(error);
    }

    fn has_errors(&self) -> bool {
        self.has_errors
    }
}

pub struct DiagnosticReportResult {
    pub error_count: u32,
    pub warning_count: u32,
}

#[derive(Debug, thiserror::Error)]
pub enum ReporterIoError {
    #[error("failed to write diagnostic: {source}")]
    WriteFailed {
        #[from]
        source: std::io::Error,
    },
    #[error("failed to flush reporter: {source}")]
    FlushFailed { source: std::io::Error },
}

#[derive(Debug, thiserror::Error)]
pub enum ReporterSendError {
    #[error("reporter actor is closed")]
    ReceiverClosed,
}

struct ReporterActor {
    receiver: mpsc::UnboundedReceiver<ReporterMessage>,
    renderer: DiagnosticRenderer,
    error_count: u32,
    warning_count: u32,
}

impl ReporterActor {
    fn run(mut self, mut writer: impl Write) -> Result<DiagnosticReportResult, ReporterIoError> {
        while let Some(msg) = self.receiver.blocking_recv() {
            match msg {
                ReporterMessage::Report(error) => {
                    if error.severity() == Some(Severity::Error) {
                        self.error_count += 1;
                    } else {
                        self.warning_count += 1;
                    }
                    self.renderer.write_error(&mut writer, error)?;
                }
                ReporterMessage::ReportBatch(errors) => {
                    for error in errors {
                        if error.severity() == Some(Severity::Error) {
                            self.error_count += 1;
                        } else {
                            self.warning_count += 1;
                        }
                        self.renderer.write_error(&mut writer, error)?;
                    }
                }
            }
        }

        if self.error_count > 0 || self.warning_count > 0 {
            let use_color = std::io::stderr().is_terminal();
            let tag = if self.error_count > 0 {
                if use_color {
                    "\x1b[1;31m   Failed\x1b[0m"
                } else {
                    "   Failed"
                }
            } else if use_color {
                "\x1b[1;33m  Warning\x1b[0m"
            } else {
                "  Warning"
            };

            let summary = match (self.error_count, self.warning_count) {
                (0, 0) => unreachable!(),
                (err, 0) => format!(
                    "{} with {} {}",
                    tag,
                    err,
                    if err == 1 { "error" } else { "errors" }
                ),
                (0, warn) => format!(
                    "{} with {} {}",
                    tag,
                    warn,
                    if warn == 1 { "warning" } else { "warnings" }
                ),
                (err, warn) => format!(
                    "{} with {} {} and {} {}",
                    tag,
                    err,
                    if err == 1 { "error" } else { "errors" },
                    warn,
                    if warn == 1 { "warning" } else { "warnings" }
                ),
            };

            writeln!(writer, "\n{}", summary)?;
        }

        writer
            .flush()
            .map_err(|e| ReporterIoError::FlushFailed { source: e })?;

        Ok(DiagnosticReportResult {
            error_count: self.error_count,
            warning_count: self.warning_count,
        })
    }
}

pub fn spawn_reporter(
    renderer: DiagnosticRenderer,
    writer: impl Write + Send + 'static,
) -> (
    Reporter,
    tokio::task::JoinHandle<Result<DiagnosticReportResult, ReporterIoError>>,
) {
    let (sender, receiver) = mpsc::unbounded_channel();
    let actor = ReporterActor {
        receiver,
        renderer,
        error_count: 0,
        warning_count: 0,
    };

    let handle = tokio::task::spawn_blocking(move || actor.run(writer));
    (Reporter { sender }, handle)
}

pub struct DiagnosticRenderer {
    handler: GraphicalReportHandler,
}

impl Default for DiagnosticRenderer {
    fn default() -> Self {
        Self {
            handler: GraphicalReportHandler::new_themed(GraphicalTheme::unicode()),
        }
    }
}

impl DiagnosticRenderer {
    pub fn write_error(
        &self,
        writer: &mut impl Write,
        error: Error,
    ) -> Result<(), ReporterIoError> {
        let mut out = String::new();
        if let Err(e) = self.handler.render_report(&mut out, error.as_ref()) {
            // If graphical rendering fails, fallback to simple display
            writeln!(writer, "error: {error} (rendering failed: {e})")?;
        } else {
            writer.write_all(out.as_bytes())?;
        }
        Ok(())
    }
}
