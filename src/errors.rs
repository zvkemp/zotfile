use std::convert::From;

#[derive(Debug)]
pub enum ErrorKind {
    Base,
    Git(String),
    Io(std::io::ErrorKind),
    Toml(Option<(usize, usize)>)
}

#[derive(Debug)] pub struct Error {
    kind: ErrorKind,
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<git2::Error> for self::Error {
    fn from(error: git2::Error) -> Self {
        Error { kind: ErrorKind::Git(error.message().to_owned()) }
    }
}

impl From<std::io::Error> for self::Error {
    fn from(error: std::io::Error) -> Self {
        Error { kind: ErrorKind::Io(error.kind()) }
    }
}

impl From<toml::de::Error> for self::Error {
    fn from(error: toml::de::Error) -> Self {
        Error { kind: ErrorKind::Toml(error.line_col()) }
    }
}
