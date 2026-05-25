use std::path::Path;

use aes_foundation::Diagnostic;

pub fn failed_to_read_file(path: &Path) -> Diagnostic {
    Diagnostic::error(format!("failed to read file: {}", path.display()))
        .with_help("Try checking the file path and permissions.")
}

pub fn failed_to_connect_backend(server: &str, err: impl std::fmt::Display) -> Diagnostic {
    Diagnostic::error(format!("failed to connect to Aegis server: {server}")).with_help(format!(
        "Ensure the server is running and the endpoint is correct. Details: {err}"
    ))
}

pub fn failed_to_publish_schema(err: impl std::fmt::Display) -> Diagnostic {
    Diagnostic::error("failed to publish schema to Aegis server")
        .with_help(format!("The server rejected the schema. Details: {err}"))
}
