use std::{env, ffi::OsString, io, path};

pub trait ProcessInit {
    type Stdout: io::Write + Send;
    type Stderr: io::Write + Send + 'static;

    fn args(&self) -> &[OsString];
    fn cwd(&self) -> &path::Path;
    fn streams(&mut self) -> (&mut Self::Stdout, &mut Self::Stderr);

    /// Consumes the initialization object and returns the owned output streams.
    fn take_streams(self) -> (Self::Stdout, Self::Stderr);
}

pub struct EnvProcessInit {
    args: Vec<OsString>,
    cwd: path::PathBuf,
    stdout: io::BufWriter<io::Stdout>,
    stderr: io::BufWriter<io::Stderr>,
}

impl EnvProcessInit {
    /// Constructs an `EnvProcessInit` with active standard OS streams and environment variables.
    pub fn new() -> Result<Self, io::Error> {
        Ok(Self {
            args: env::args_os().collect(),
            cwd: env::current_dir()?,
            stdout: io::BufWriter::new(io::stdout()),
            stderr: io::BufWriter::new(io::stderr()),
        })
    }
}

impl ProcessInit for EnvProcessInit {
    type Stdout = io::BufWriter<io::Stdout>;
    type Stderr = io::BufWriter<io::Stderr>;

    fn args(&self) -> &[OsString] {
        &self.args
    }

    fn cwd(&self) -> &path::Path {
        self.cwd.as_path()
    }

    fn streams(&mut self) -> (&mut Self::Stdout, &mut Self::Stderr) {
        (&mut self.stdout, &mut self.stderr)
    }

    fn take_streams(self) -> (Self::Stdout, Self::Stderr) {
        (self.stdout, self.stderr)
    }
}
