use std::fmt;

#[derive(Debug)]
pub struct Error {
    kind: Kind,
    message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Usage,
    Fail,
}

impl Error {
    pub fn usage(message: impl Into<String>) -> Self {
        Self {
            kind: Kind::Usage,
            message: message.into(),
        }
    }

    pub fn fail(message: impl Into<String>) -> Self {
        Self {
            kind: Kind::Fail,
            message: message.into(),
        }
    }

    pub fn exit_code(&self) -> i32 {
        match self.kind {
            Kind::Usage => 2,
            Kind::Fail => 1,
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Error {}

impl From<rusqlite::Error> for Error {
    fn from(err: rusqlite::Error) -> Self {
        Self::fail(err.to_string())
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::fail(err.to_string())
    }
}
