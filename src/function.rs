
use std::path::{Path, PathBuf};
use std::str;

use regex::Regex;
use nom::{IError, IResult};

use super::Error;

#[derive(Debug)]
pub enum ComparisonOperator {
    Equal,
    NotEqual,
    LessThan,
    GreaterThan,
    LessThanOrEqual,
    GreaterThanOrEqual,
}

#[derive(Debug)]
pub enum Function {
    FilePath(PathBuf),
    FileRegex(Regex),
    ActivePath(PathBuf),
    ActiveRegex(Regex),
    Many(Regex),
    ManyActive(Regex),
    Checksum(PathBuf, u32),
    Version(PathBuf, String, ComparisonOperator),
}

const INVALID_PATH_CHARS: &str = "\":*?<>|\\"; // \ is treated as invalid to distinguish regex strings.
const INVALID_REGEX_PATH_CHARS: &str = "\"<>";

impl Function {
    pub fn eval(&self) -> Result<bool, Error> {
        // TODO: Handle all variants.
        // TODO: Paths may not lead outside game directory.
        match *self {
            Function::FilePath(ref f) => Ok(f.exists()),
            _ => Ok(false),
        }
    }

    pub fn parse(input: &str) -> IResult<&str, Function> {
        // TODO: Handle all variants.
        // TODO: Paths may not contain :*?"<>|
        do_parse!(
            input,
            function: alt!(
                delimited!(tag!("file(\""), is_not!(INVALID_PATH_CHARS), tag!("\")")) => {
                    |path| Function::FilePath(PathBuf::from(path))
                } |
                delimited!(tag!("active(\""), is_not!(INVALID_PATH_CHARS), tag!("\")")) => {
                    |path| Function::ActivePath(PathBuf::from(path))
                }
            ) >> (function)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn function_parse_should_parse_a_file_path_function() {
        let result = Function::parse("file(\"Cargo.toml\")").to_result().unwrap();

        match result {
            Function::FilePath(f) => assert_eq!(PathBuf::from("Cargo.toml"), f),
            _ => panic!("Expected a file path function"),
        }
    }

    #[test]
    fn function_parse_should_parse_an_active_function() {
        let result = Function::parse("active(\"Cargo.toml\")").to_result().unwrap();

        match result {
            Function::ActivePath(f) => assert_eq!(PathBuf::from("Cargo.toml"), f),
            _ => panic!("Expected an active path function"),
        }
    }

    #[test]
    fn function_file_path_eval_should_return_true_if_the_file_exists_relative_to_the_data_path() {
        let function = Function::FilePath(PathBuf::from("Cargo.toml"));

        assert!(function.eval().unwrap());

        unimplemented!("not yet any way to actually specify the data path");
    }

    #[test]
    fn function_file_path_eval_should_return_true_if_given_a_plugin_that_is_ghosted() {
        let function = Function::FilePath(PathBuf::from("test.esp"));

        assert!(function.eval().unwrap());

        unimplemented!("need to add tempdir and create a test.esp.ghost");
    }

    #[test]
    fn function_file_path_eval_should_error_if_the_path_is_outside_game_directory() {
        unimplemented!("to do");
    }

    #[test]
    fn function_file_path_eval_should_return_false_if_the_file_does_not_exist() {
        let function = Function::FilePath(PathBuf::from("missing"));

        assert!(!function.eval().unwrap());
    }
}
