use std::borrow::Cow;

pub use miette::{GraphicalReportHandler, GraphicalTheme, LabeledSpan, NamedSource};
pub type Result<T> = std::result::Result<T, Diagnostic>;

type CowStr = Cow<'static, str>;

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
}
