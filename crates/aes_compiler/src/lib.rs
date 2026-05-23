use aes_foundation::{Reporter, vfs};

mod export;

/// The high-level entry point for the Aegis compiler pipeline.
///
/// `Compiler` manages the Virtual File System (VFS) and provides methods to
/// validate and export schemas. It orchestrates the flow from raw source
/// strings to analyzed semantic models and exported IR.
#[derive(Default)]
pub struct Compiler {
    vfs: vfs::Vfs,
}

impl Compiler {
    /// Adds a new file to the compiler's VFS and returns its unique identifier.
    pub fn add_file(&mut self, path: String, source: String) -> vfs::FileId {
        self.vfs.add(path, source)
    }

    /// Performs semantic analysis on the specified file.
    ///
    /// Returns a [`Schema`] if the file is semantically valid, or `None` if
    /// parsing or analysis errors occur. All diagnostics are reported via
    /// the provided [`Reporter`].
    pub fn check(
        &self,
        id: vfs::FileId,
        reporter: &mut impl Reporter,
    ) -> Option<aes_semantic::Schema<'_>> {
        let file = self.vfs.get(id)?;
        let ast = self.parse_ast(file, reporter)?;
        aes_semantic::analyze(file, &ast, reporter)
    }

    /// Compiles and exports the specified file to the Aegis IR format.
    ///
    /// This method performs full validation before exporting. If the schema is
    /// valid, it returns the exported [`ir::v1::Schema`].
    pub fn export_schema(
        &self,
        id: vfs::FileId,
        reporter: &mut impl Reporter,
    ) -> Option<aes_ir::v1::Schema> {
        let file = self.vfs.get(id)?;
        let ast = self.parse_ast(file, reporter)?;
        let _ = aes_semantic::analyze(file, &ast, reporter)?;
        export::Exporter::new(&ast, file).export_schema().into()
    }

    fn parse_ast<'f>(
        &self,
        file: vfs::FileRef<'f>,
        reporter: &mut impl Reporter,
    ) -> Option<aes_ast::Ast<'f>> {
        let ast = aes_parser::Parser::new(file, &mut *reporter).parse();
        if reporter.has_errors() {
            return None;
        }

        Some(ast)
    }
}
