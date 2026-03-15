use std::sync::Arc;

use aes_foundation::{GraphicalReportHandler, GraphicalTheme, NamedSource};

pub fn render_diagnostics(source: &str, errors: &[aes_foundation::Diagnostic]) -> String {
    let handler = GraphicalReportHandler::new_themed(GraphicalTheme::unicode_nocolor());
    let named_src = Arc::new(NamedSource::new("test.aes", source.to_owned()));

    let mut out = String::new();
    for (i, diag) in errors.iter().enumerate() {
        if i > 0 {
            out.push_str("\n---\n\n");
        }
        let error = diag.clone().with_source_code(named_src.clone());
        handler.render_report(&mut out, error.as_ref()).ok();
    }
    out
}

pub trait URN {
    fn urn(&self) -> String;
}

impl URN for aes_foundation::Diagnostic {
    fn urn(&self) -> String {
        let scope = self.code().scope.as_ref().unwrap();
        let number = self.code().number.as_ref().unwrap();
        format!("{scope}({number})")
    }
}

#[track_caller]
pub fn assert_code(r: &crate::Reporter, urn: &str) {
    use aes_foundation::Reporter;
    assert!(r.has_errors());
    assert_eq!(r.diagnostics.len(), 1);
    assert_eq!(r.diagnostics[0].urn(), urn);
}
