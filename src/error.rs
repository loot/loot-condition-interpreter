use std::error;
use std::fmt;
use std::io;
use std::num::ParseIntError;
use std::path::PathBuf;

use nom::error::ErrorKind;
use nom::Err;
use regex;

#[derive(Debug)]
pub enum Error {
    ParsingIncomplete,
    // The string is the input that was not parsed.
    UnconsumedInput(String),
    /// The string is the input at which the error was encountered.
    ParsingError(String, ParsingErrorKind),
    PeParsingError(PathBuf, Box<dyn error::Error>),
    IoError(PathBuf, io::Error),
}

fn escape<I: fmt::Display>(input: I) -> String {
    input.to_string().replace("\"", "\\\"")
}

impl<I: fmt::Debug + fmt::Display> From<Err<ParsingError<I>>> for Error {
    fn from(error: Err<ParsingError<I>>) -> Self {
        match error {
            Err::Incomplete(_) => Error::ParsingIncomplete,
            Err::Error(e) | Err::Failure(e) => Error::ParsingError(escape(e.input), e.kind),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::ParsingIncomplete => write!(f, "More input was expected by the parser"),
            Error::UnconsumedInput(i) => write!(
                f,
                "The parser did not consume the following input: \"{}\"",
                i
            ),
            Error::ParsingError(i, e) => write!(
                f,
                "An error was encountered while parsing the expression \"{}\": {}",
                i, e
            ),
            Error::PeParsingError(p, e) => write!(
                f,
                "An error was encountered while reading the version fields of \"{}\": {}",
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
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::ParsingError(_, e) => Some(e),
            Error::PeParsingError(_, e) => Some(e.as_ref()),
            Error::IoError(_, e) => Some(e),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct ParsingError<I: fmt::Debug + fmt::Display> {
    input: I,
    kind: ParsingErrorKind,
}

impl<I: fmt::Debug + fmt::Display> From<(I, ErrorKind)> for ParsingError<I> {
    fn from((input, kind): (I, ErrorKind)) -> Self {
        use nom::error::ParseError;
        ParsingError::from_error_kind(input, kind)
    }
}

impl<I: fmt::Debug + fmt::Display> fmt::Display for ParsingError<I> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "An error was encountered while parsing the expression \"{}\": {}",
            self.input, self.kind
        )
    }
}

impl<I: fmt::Debug + fmt::Display> error::Error for ParsingError<I> {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        self.kind.source()
    }
}

impl<I: fmt::Debug + fmt::Display> nom::error::ParseError<I> for ParsingError<I> {
    fn from_error_kind(input: I, kind: ErrorKind) -> Self {
        ParsingError {
            input,
            kind: ParsingErrorKind::GenericParserError(kind.description().to_string()),
        }
    }

    fn append(_: I, _: ErrorKind, other: Self) -> Self {
        other
    }
}

#[derive(Debug)]
pub enum ParsingErrorKind {
    InvalidRegexSyntax(String),
    InvalidRegexUnknown,
    InvalidCrc(ParseIntError),
    PathEndsInADirectorySeparator(PathBuf),
    PathIsNotInGameDirectory(PathBuf),
    GenericParserError(String),
}

impl ParsingErrorKind {
    pub fn at<I: fmt::Debug + fmt::Display>(self, input: I) -> ParsingError<I> {
        ParsingError { input, kind: self }
    }
}

impl From<regex::Error> for ParsingErrorKind {
    fn from(error: regex::Error) -> Self {
        match error {
            regex::Error::Syntax(s) => ParsingErrorKind::InvalidRegexSyntax(s),
            _ => ParsingErrorKind::InvalidRegexUnknown,
        }
    }
}

impl From<ParseIntError> for ParsingErrorKind {
    fn from(error: ParseIntError) -> Self {
        ParsingErrorKind::InvalidCrc(error)
    }
}

impl error::Error for ParsingErrorKind {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            ParsingErrorKind::InvalidCrc(e) => Some(e),
            _ => None,
        }
    }
}

impl fmt::Display for ParsingErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParsingErrorKind::InvalidRegexSyntax(s) => write!(f, "{}", s),
            ParsingErrorKind::InvalidRegexUnknown => write!(f, "Unknown regex parsing error"),
            ParsingErrorKind::InvalidCrc(e) => e.fmt(f),
            ParsingErrorKind::PathEndsInADirectorySeparator(p) => {
                write!(f, "\"{}\" ends in a directory separator", p.display())
            }
            ParsingErrorKind::PathIsNotInGameDirectory(p) => {
                write!(f, "\"{}\" is not in the game directory", p.display())
            }
            ParsingErrorKind::GenericParserError(e) => write!(f, "Error in parser: {}", e),
        }
    }
}
