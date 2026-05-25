use std::{
    io,
    path::{Path, PathBuf},
};

pub fn read(path: &Path) -> io::Result<String> {
    let bytes = std::fs::read(path)?;
    let Ok(_) = simdutf8::basic::from_utf8(&bytes) else {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid utf-8"));
    };
    Ok(unsafe { String::from_utf8_unchecked(bytes) })
}

pub fn resolve_path(path: &Path, cwd: &Path) -> PathBuf {
    if path.is_relative() {
        cwd.join(path)
    } else {
        path.to_path_buf()
    }
}
