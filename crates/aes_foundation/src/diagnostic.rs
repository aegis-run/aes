use std::borrow::Cow;
use std::path::Path;
use std::sync::Arc;

pub use miette::{GraphicalReportHandler, GraphicalTheme, LabeledSpan, NamedSource};
pub type Result<T> = std::result::Result<T, Diagnostic>;
pub type Error = miette::Error;
pub type Severity = miette::Severity;
type CowStr = Cow<'static, str>;

/// Metadata used to enrich a [`Diagnostic`] with source context.
#[derive(Debug, Clone, Copy)]
pub struct DiagnosticSource<'a> {
    pub cwd: Option<&'a Path>,
    pub path: &'a Path,
    pub source_text: &'a str,
}

impl<'a> DiagnosticSource<'a> {
    pub fn display_name(&self) -> String {
        let display_path = if let Some(cwd) = self.cwd {
            self.path
                .strip_prefix(cwd)
                .unwrap_or(self.path)
                .to_string_lossy()
        } else {
            self.path.to_string_lossy()
        };

        // Normalize path separators to '/' for stable CLI/snapshot output.
        display_path.replace('\\', "/")
    }

    pub fn to_named_source(&self) -> NamedSource<String> {
        NamedSource::new(self.display_name(), self.source_text.to_owned())
    }
}

/// The core error-reporting structure for the Aegis compiler, wrapping [`miette`].
///
/// `Diagnostic` provides a fluent builder API (e.g., `with_code`, `with_label`) for constructing
/// rich, human-readable error messages. Rather than panicking or immediately printing to stderr,
/// diagnostics are yielded to a [`Reporter`] interface which decides how to display or buffer them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    message: CowStr,
    labels: Option<Vec<miette::LabeledSpan>>,
    help: Option<CowStr>,
    severity: miette::Severity,
    code: DiagnosticCode,
}

impl Diagnostic {
    pub fn error(message: impl Into<CowStr>) -> Self {
        Self::new(message, miette::Severity::Error)
    }

    pub fn warn(message: impl Into<CowStr>) -> Self {
        Self::new(message, miette::Severity::Warning)
    }

    pub fn advice(message: impl Into<CowStr>) -> Self {
        Self::new(message, miette::Severity::Advice)
    }

    fn new(message: impl Into<CowStr>, severity: miette::Severity) -> Self {
        Self {
            message: message.into(),
            labels: None,
            help: None,
            severity,
            code: DiagnosticCode::default(),
        }
    }

    /// Attach a diagnostic code: `scope(number)`, e.g. `aes::lexer(E001)`.
    pub fn with_code(self, scope: impl Into<CowStr>, number: impl Into<CowStr>) -> Self {
        self.with_code_scope(scope).with_code_number(number)
    }

    pub fn with_code_scope(mut self, scope: impl Into<CowStr>) -> Self {
        if self.code.scope.is_none() {
            self.code.scope = Some(scope.into());
        }
        self
    }

    pub fn with_code_number(mut self, number: impl Into<CowStr>) -> Self {
        if self.code.number.is_none() {
            self.code.number = Some(number.into());
        }
        self
    }

    /// Attach a help message shown below the diagnostic.
    pub fn with_help(mut self, help: impl Into<CowStr>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// Set a single label, replacing any existing labels.
    pub fn with_label(mut self, label: impl Into<miette::LabeledSpan>) -> Self {
        self.labels = Some(vec![label.into()]);
        self
    }

    /// Set multiple labels, replacing any existing labels.
    pub fn with_labels(
        mut self,
        labels: impl IntoIterator<Item = impl Into<miette::LabeledSpan>>,
    ) -> Self {
        self.labels = Some(labels.into_iter().map(Into::into).collect());
        self
    }

    pub fn with_source_code<T: miette::SourceCode + Send + Sync + 'static>(
        self,
        code: T,
    ) -> miette::Error {
        miette::Error::from(self).with_source_code(code)
    }

    /// Append a single label to any existing labels.
    pub fn and_label(mut self, label: impl Into<miette::LabeledSpan>) -> Self {
        self.labels.get_or_insert_with(Vec::new).push(label.into());
        self
    }

    /// Append multiple labels to any existing labels.
    pub fn and_labels(
        mut self,
        labels: impl IntoIterator<Item = impl Into<miette::LabeledSpan>>,
    ) -> Self {
        self.labels
            .get_or_insert_with(Vec::new)
            .extend(labels.into_iter().map(Into::into));
        self
    }

    pub fn message(&self) -> &str {
        &self.message
    }
    pub fn severity(&self) -> miette::Severity {
        self.severity
    }
    pub fn code(&self) -> &DiagnosticCode {
        &self.code
    }
    pub fn labels(&self) -> Option<&[miette::LabeledSpan]> {
        self.labels.as_deref()
    }
    pub fn help(&self) -> Option<&str> {
        self.help.as_deref()
    }

    pub fn is_error(&self) -> bool {
        self.severity == miette::Severity::Error
    }

    /// Enriches the diagnostic with an existing source context, converting it into a [`miette::Error`].
    pub fn enrich_with_source(self, source: Arc<NamedSource<String>>) -> Error {
        self.with_source_code(source)
    }

    /// Enriches the diagnostic with source context, converting it into a [`miette::Error`].
    pub fn enrich(self, source: DiagnosticSource<'_>) -> Error {
        let display_path = source.display_name();
        let named_source = NamedSource::new(display_path, source.source_text.to_owned());
        self.enrich_with_source(Arc::new(named_source))
    }
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.message.fmt(f)
    }
}

impl std::error::Error for Diagnostic {}

impl miette::Diagnostic for Diagnostic {
    fn severity(&self) -> Option<miette::Severity> {
        Some(self.severity)
    }

    fn code<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        self.code
            .is_some()
            .then(|| Box::new(&self.code) as Box<dyn std::fmt::Display>)
    }

    fn help<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        self.help
            .as_ref()
            .map(|h| Box::new(h) as Box<dyn std::fmt::Display>)
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = miette::LabeledSpan> + '_>> {
        self.labels
            .as_ref()
            .map(|ls| Box::new(ls.iter().cloned()) as Box<dyn Iterator<Item = miette::LabeledSpan>>)
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DiagnosticCode {
    pub scope: Option<CowStr>,
    pub number: Option<CowStr>,
}

impl DiagnosticCode {
    pub fn is_some(&self) -> bool {
        self.scope.is_some() || self.number.is_some()
    }
}

impl std::fmt::Display for DiagnosticCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (&self.scope, &self.number) {
            (Some(scope), Some(number)) => write!(f, "{scope}({number})"),
            (Some(scope), None) => scope.fmt(f),
            (None, Some(number)) => number.fmt(f),
            (None, None) => Ok(()),
        }
    }
}

/// An interface for collecting or immediately emitting [`Diagnostic`] events.
///
/// Compilers often need to switch between buffering errors (e.g., during Language Server
/// validation to return all errors at once) and immediate printing (e.g., CLI usage).
/// Any struct implementing `Reporter` manages this lifecycle.
pub trait Reporter {
    fn report(&mut self, diagnostic: Diagnostic);

    fn has_errors(&self) -> bool;
}

impl<T: Reporter + ?Sized> Reporter for &mut T {
    fn report(&mut self, diagnostic: Diagnostic) {
        (**self).report(diagnostic);
    }

    fn has_errors(&self) -> bool {
        (**self).has_errors()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    #[test]
    fn error_is_error() {
        let d = Diagnostic::error("something went wrong");
        assert!(d.is_error());
        assert_eq!(d.severity(), miette::Severity::Error);
    }

    #[test]
    fn warn_is_not_error() {
        let d = Diagnostic::warn("something looks odd");
        assert!(!d.is_error());
    }

    #[test]
    fn builder_chain() {
        let d = Diagnostic::error("bad input")
            .with_code("aes::lexer", "E001")
            .with_help("try this instead")
            .with_label(miette::LabeledSpan::at((0usize, 1usize), "here"));

        assert_eq!(d.code().to_string(), "aes::lexer(E001)");
        assert_eq!(d.help(), Some("try this instead"));
        assert_eq!(d.labels().unwrap().len(), 1);
    }

    #[test]
    fn and_label_appends() {
        let d = Diagnostic::error("bad input")
            .with_label(miette::LabeledSpan::at((0usize, 1usize), "first"))
            .and_label(miette::LabeledSpan::at((2usize, 1usize), "second"));

        assert_eq!(d.labels().unwrap().len(), 2);
    }

    #[test]
    fn with_source_code_converts_to_miette_error() {
        let d = Diagnostic::error("unexpected character")
            .with_label(miette::LabeledSpan::at((0usize, 1usize), "here"));

        let src = Arc::new("@type user {}".to_string());
        let error = d.with_source_code(miette::NamedSource::new("schema.aes", src));

        // miette::Error is renderable
        let mut out = String::new();
        miette::GraphicalReportHandler::new()
            .render_report(&mut out, error.as_ref())
            .unwrap();

        assert!(out.contains("unexpected character"));
    }

    #[test]
    fn code_display_formats() {
        let mut code = DiagnosticCode::default();
        assert!(!code.is_some());
        assert_eq!(code.to_string(), "");

        code.scope = Some("aes::lexer".into());
        code.number = Some("E001".into());
        assert_eq!(code.to_string(), "aes::lexer(E001)");

        let scope_only = DiagnosticCode {
            scope: Some("aes".into()),
            number: None,
        };
        assert_eq!(scope_only.to_string(), "aes");
    }

    #[test]
    fn enrichment_normalizes_paths() {
        let d = Diagnostic::error("test");
        let source = DiagnosticSource {
            cwd: None,
            path: Path::new("path\\to\\file.aes"),
            source_text: "test",
        };
        let enriched = d.enrich(source);
        // We can't easily inspect the internal NamedSource name without rendering,
        // but we can at least check that it doesn't panic.
        assert!(enriched.to_string().contains("test"));
    }
}
