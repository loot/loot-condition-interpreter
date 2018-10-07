use std::error;
use std::fmt;
use std::io;
use std::num::ParseIntError;
use std::path::PathBuf;

use nom::{Context, Err, ErrorKind};
use regex;

#[derive(Debug)]
pub enum Error {
    ParsingIncomplete,
    /// The first string is the expression parsed, the second is a tag describing the parser that failed.
    GenericParsingError(String, String),
    /// The string is the expression parsed.
    CustomParsingError(String, ParsingError),
    PeParsingError(PathBuf, Box<error::Error>),
    IoError(PathBuf, io::Error),
}

fn escape<I: fmt::Display>(input: I) -> String {
    input.to_string().replace("\"", "\\\"")
}

impl<I: fmt::Debug + fmt::Display> From<Err<I, ParsingError>> for Error {
    fn from(error: Err<I, ParsingError>) -> Self {
        match error {
            Err::Incomplete(_) => Error::ParsingIncomplete,
            Err::Error(Context::Code(i, ErrorKind::Custom(e)))
            | Err::Failure(Context::Code(i, ErrorKind::Custom(e))) => {
                Error::CustomParsingError(escape(i), e)
            }
            Err::Error(Context::Code(i, e)) | Err::Failure(Context::Code(i, e)) => {
                Error::GenericParsingError(escape(i), format!("{:?}", e))
            }
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::ParsingIncomplete => write!(f, "More input was expected by the parser"),
            Error::GenericParsingError(i, e) => write!(
                f,
                "An error was encountered in the parser \"{}\" while parsing the expression \"{}\"",
                e, i
            ),
            Error::CustomParsingError(i, e) => write!(
                f,
                "An error was encountered while parsing the expression \"{}\": {}",
                i, e
            ),
            Error::PeParsingError(p, e) => write!(
                f,
                "An error was encountered while reading the file version field of \"{}\": {}",
                p.display(),
                e
            ),
            Error::IoError(p, e) => write!(
                f,
                "An error was encountered while accessing the path \"{}\": {}",
                p.display(),
                e
            ),
        }
    }
}

impl error::Error for Error {
    fn cause(&self) -> Option<&error::Error> {
        match self {
            Error::CustomParsingError(_, e) => Some(e),
            Error::PeParsingError(_, e) => Some(e.as_ref()),
            Error::IoError(_, e) => Some(e),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum ParsingError {
    InvalidRegexSyntax(String),
    InvalidRegexUnknown,
    InvalidCrc(ParseIntError),
    PathEndsInADirectorySeparator(PathBuf),
    PathIsNotInGameDirectory(PathBuf),
    Unknown(u32),
}

impl From<regex::Error> for ParsingError {
    fn from(error: regex::Error) -> Self {
        match error {
            regex::Error::Syntax(s) => ParsingError::InvalidRegexSyntax(s),
            _ => ParsingError::InvalidRegexUnknown,
        }
    }
}

impl From<ParseIntError> for ParsingError {
    fn from(error: ParseIntError) -> Self {
        ParsingError::InvalidCrc(error)
    }
}

impl From<u32> for ParsingError {
    fn from(error: u32) -> Self {
        ParsingError::Unknown(error)
    }
}

impl fmt::Display for ParsingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParsingError::InvalidRegexSyntax(s) => write!(f, "{}", s),
            ParsingError::InvalidRegexUnknown => write!(f, "Unknown regex parsing error"),
            ParsingError::InvalidCrc(e) => e.fmt(f),
            ParsingError::PathEndsInADirectorySeparator(p) => {
                write!(f, "\"{}\" ends in a directory separator", p.display())
            }
            ParsingError::PathIsNotInGameDirectory(p) => {
                write!(f, "\"{}\" is not in the game directory", p.display())
            }
            ParsingError::Unknown(e) => write!(f, "Unknown error code {}", e),
        }
    }
}

impl error::Error for ParsingError {
    fn cause(&self) -> Option<&error::Error> {
        match self {
            ParsingError::InvalidCrc(e) => Some(e),
            _ => None,
        }
    }
}
