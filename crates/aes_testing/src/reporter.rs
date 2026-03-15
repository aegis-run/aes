use aes_foundation::Diagnostic;

#[derive(Default)]
pub struct Reporter {
    pub diagnostics: Vec<Diagnostic>,
}

impl aes_foundation::Reporter for Reporter {
    fn report(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }
    fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|d| d.is_error())
    }
}

impl Reporter {
    pub fn messages(&self) -> Vec<&str> {
        self.diagnostics.iter().map(|d| d.message()).collect()
    }
    pub fn is_clean(&self) -> bool {
        self.diagnostics.is_empty()
    }
}
