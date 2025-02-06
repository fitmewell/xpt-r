use std::fmt::Display;
use std::string::FromUtf8Error;

#[derive(Debug)]
pub enum XPTError {
    DecodeError(String),
    ParseError(String),
}

impl Display for XPTError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            XPTError::DecodeError(a) => a.to_string(),
            XPTError::ParseError(a) => a.to_string(),
        };
        write!(f, "{}", str)
    }
}

impl From<FromUtf8Error> for XPTError {
    fn from(err: FromUtf8Error) -> XPTError {
        XPTError::DecodeError(err.to_string())
    }
}

impl From<std::io::Error> for XPTError {
    fn from(err: std::io::Error) -> XPTError {
        XPTError::DecodeError(err.to_string())
    }
}
