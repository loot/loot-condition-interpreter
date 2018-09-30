use std::path::PathBuf;
use std::str;

use nom::{hex_digit, Context, Err, ErrorKind, IResult};
use regex::Regex;

use super::Error;

#[derive(Debug, PartialEq, Eq)]
pub enum ComparisonOperator {
    Equal,
    NotEqual,
    LessThan,
    GreaterThan,
    LessThanOrEqual,
    GreaterThanOrEqual,
}

impl ComparisonOperator {
    pub fn parse(input: &str) -> IResult<&str, ComparisonOperator> {
        do_parse!(
            input,
            operator:
                alt!(
                tag!("==") => { |_| ComparisonOperator::Equal } |
                tag!("!=") => { |_| ComparisonOperator::NotEqual } |
                tag!("<") => { |_| ComparisonOperator::LessThan } |
                tag!(">") => { |_| ComparisonOperator::GreaterThan } |
                tag!("<=") => { |_| ComparisonOperator::LessThanOrEqual } |
                tag!(">=") => { |_| ComparisonOperator::GreaterThanOrEqual }
            ) >> (operator)
        )
    }
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

const PARSE_REGEX_ERROR: ErrorKind = ErrorKind::Custom(1);
const PARSE_CRC_ERROR: ErrorKind = ErrorKind::Custom(2);

fn parse_regex(input: &str) -> IResult<&str, Regex> {
    Regex::new(input)
        .map(|r| ("", r))
        .map_err(|_| Err::Failure(Context::Code(input, PARSE_REGEX_ERROR)))
}

fn parse_version_args(input: &str) -> IResult<&str, (PathBuf, &str, ComparisonOperator)> {
    do_parse!(
        input,
        tag!("\"")
            >> path: is_not!(INVALID_PATH_CHARS)
            >> tag!("\"")
            >> ws!(tag!(","))
            >> tag!("\"")
            >> version: is_not!("\"")
            >> tag!("\"")
            >> ws!(tag!(","))
            >> operator: call!(ComparisonOperator::parse)
            >> ((PathBuf::from(path), version, operator))
    )
}

fn parse_crc(input: &str) -> IResult<&str, u32> {
    u32::from_str_radix(input, 16)
        .map(|c| ("", c))
        .map_err(|_| Err::Failure(Context::Code(input, PARSE_CRC_ERROR)))
}

fn parse_checksum_args(input: &str) -> IResult<&str, (PathBuf, u32)> {
    do_parse!(
        input,
        tag!("\"")
            >> path: is_not!(INVALID_PATH_CHARS)
            >> tag!("\"")
            >> ws!(tag!(","))
            >> crc: flat_map!(call!(hex_digit), parse_crc)
            >> ((PathBuf::from(path), crc))
    )
}

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
        do_parse!(
            input,
            function:
                alt!(
                delimited!(tag!("file(\""), is_not!(INVALID_PATH_CHARS), tag!("\")")) => {
                    |path| Function::FilePath(PathBuf::from(path))
                } |
                delimited!(tag!("file(\""), flat_map!(is_not!(INVALID_REGEX_PATH_CHARS), parse_regex), tag!("\"")) => {
                    |r| Function::FileRegex(r)
                } |
                delimited!(tag!("active(\""), is_not!(INVALID_PATH_CHARS), tag!("\")")) => {
                    |path| Function::ActivePath(PathBuf::from(path))
                } |
                delimited!(tag!("active(\""), flat_map!(is_not!(INVALID_REGEX_PATH_CHARS), parse_regex), tag!("\"")) => {
                    |r| Function::ActiveRegex(r)
                } |
                delimited!(tag!("many(\""), flat_map!(is_not!(INVALID_REGEX_PATH_CHARS), parse_regex), tag!("\"")) => {
                    |r| Function::Many(r)
                } |
                delimited!(tag!("many_active(\""), flat_map!(is_not!(INVALID_REGEX_PATH_CHARS), parse_regex), tag!("\"")) => {
                    |r| Function::ManyActive(r)
                } |
                delimited!(tag!("version("), call!(parse_version_args), tag!(")")) => {
                    |(path, version, comparator): (PathBuf, &str, ComparisonOperator)| {
                        Function::Version(path, version.to_string(), comparator)
                    }
                } |
                delimited!(tag!("checksum("), call!(parse_checksum_args), tag!(")")) => {
                    |(path, crc)| Function::Checksum(path, crc)
                }
            ) >> (function)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::path::Path;

    #[test]
    fn function_parse_should_parse_a_file_path_function() {
        let result = Function::parse("file(\"Cargo.toml\")").unwrap().1;

        match result {
            Function::FilePath(f) => assert_eq!(Path::new("Cargo.toml"), f),
            _ => panic!("Expected a file path function"),
        }
    }

    #[test]
    fn function_parse_should_parse_a_file_regex_function() {
        let result = Function::parse("file(\"Cargo.*\")").unwrap().1;

        match result {
            Function::FileRegex(r) => {
                assert_eq!(Regex::new("Cargo.*").unwrap().as_str(), r.as_str())
            }
            _ => panic!("Expected a file regex function"),
        }
    }

    #[test]
    fn function_parse_should_parse_an_active_path_function() {
        let result = Function::parse("active(\"Cargo.toml\")").unwrap().1;

        match result {
            Function::ActivePath(f) => assert_eq!(Path::new("Cargo.toml"), f),
            _ => panic!("Expected an active path function"),
        }
    }

    #[test]
    fn function_parse_should_parse_an_active_regex_function() {
        let result = Function::parse("active(\"Cargo.*\")").unwrap().1;

        match result {
            Function::ActiveRegex(r) => {
                assert_eq!(Regex::new("Cargo.*").unwrap().as_str(), r.as_str())
            }
            _ => panic!("Expected an active regex function"),
        }
    }

    #[test]
    fn function_parse_should_parse_a_many_function() {
        let result = Function::parse("many(\"Cargo.*\")").unwrap().1;

        match result {
            Function::Many(r) => {
                assert_eq!(Regex::new("Cargo.*").unwrap().as_str(), r.as_str())
            },
            _ => panic!("Expected a many function"),
        }
    }

    #[test]
    fn function_parse_should_parse_a_many_active_function() {
        let result = Function::parse("many_active(\"Cargo.*\")").unwrap().1;

        match result {
            Function::ManyActive(r) => {
                assert_eq!(Regex::new("Cargo.*").unwrap().as_str(), r.as_str())
            }
            _ => panic!("Expected a many active function"),
        }
    }

    #[test]
    fn function_parse_should_parse_a_checksum_function() {
        let result = Function::parse("checksum(\"Cargo.toml\", DEADBEEF)")
            .unwrap()
            .1;

        match result {
            Function::Checksum(path, crc) => {
                assert_eq!(Path::new("Cargo.toml"), path);
                assert_eq!(0xDEADBEEF, crc);
            }
            _ => panic!("Expected a checksum function"),
        }
    }

    #[test]
    fn function_parse_should_parse_a_version_equals_function() {
        let result = Function::parse("version(\"Cargo.toml\", \"1.2\", ==)")
            .unwrap()
            .1;

        match result {
            Function::Version(path, version, comparator) => {
                assert_eq!(Path::new("Cargo.toml"), path);
                assert_eq!("1.2", version);
                assert_eq!(ComparisonOperator::Equal, comparator);
            }
            _ => panic!("Expected a checksum function"),
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
    fn function_file_path_eval_should_be_true_if_given_LOOT() {
        unimplemented!();
    }

    #[test]
    fn function_file_path_eval_should_not_check_for_ghosted_non_plugin_file() {
        unimplemented!();
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

    #[test]
    fn function_file_regex_eval_should_error_if_the_path_is_outside_game_directory() {
        unimplemented!();
    }

    #[test]
    fn function_file_regex_eval_should_be_false_if_no_directory_entries_match() {
        unimplemented!();
    }

    #[test]
    fn function_file_regex_eval_should_be_false_if_the_parent_path_part_is_not_a_directory() {
        unimplemented!();
    }

    #[test]
    fn function_file_regex_eval_should_be_true_if_a_directory_entry_matches() {
        unimplemented!();
    }

    #[test]
    fn function_active_path_eval_should_be_false_if

}
