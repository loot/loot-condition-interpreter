use std::path::{Component, Path, PathBuf};
use std::str;

use nom::{hex_digit, Context, Err, ErrorKind, IResult};
use regex::Regex;

use super::{ComparisonOperator, Function};

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

const INVALID_PATH_CHARS: &str = "\":*?<>|\\"; // \ is treated as invalid to distinguish regex strings.
const INVALID_REGEX_PATH_CHARS: &str = "\"<>";

const PARSE_REGEX_ERROR: ErrorKind = ErrorKind::Custom(1);
const PARSE_CRC_ERROR: ErrorKind = ErrorKind::Custom(2);
const PARSE_PATH_ERROR: ErrorKind = ErrorKind::Custom(3);

fn is_in_game_path(path: &Path) -> bool {
    let mut previous_component = Component::CurDir;
    for component in path.components() {
        match (component, previous_component) {
            (Component::Prefix(_), _) => return false,
            (Component::RootDir, _) => return false,
            (Component::ParentDir, Component::ParentDir) => return false,
            (Component::CurDir, _) => continue,
            _ => previous_component = component,
        }
    }

    true
}

fn parse_regex(input: &str) -> IResult<&str, Regex> {
    Regex::new(input)
        .map(|r| ("", r))
        .map_err(|_| Err::Failure(Context::Code(input, PARSE_REGEX_ERROR)))
}

fn parse_version_args(input: &str) -> IResult<&str, (PathBuf, &str, ComparisonOperator)> {
    let (remaining_input, (path, version, comparator)) = try_parse!(
        input,
        do_parse!(
            tag!("\"")
                >> path: is_not!(INVALID_PATH_CHARS)
                >> tag!("\"")
                >> ws!(tag!(","))
                >> tag!("\"")
                >> version: is_not!("\"")
                >> tag!("\"")
                >> ws!(tag!(","))
                >> operator: call!(ComparisonOperator::parse)
                >> (PathBuf::from(path), version, operator)
        )
    );

    if is_in_game_path(&path) {
        Ok((remaining_input, (path, version, comparator)))
    } else {
        Err(Err::Failure(Context::Code(input, PARSE_PATH_ERROR)))
    }
}

fn parse_crc(input: &str) -> IResult<&str, u32> {
    u32::from_str_radix(input, 16)
        .map(|c| ("", c))
        .map_err(|_| Err::Failure(Context::Code(input, PARSE_CRC_ERROR)))
}

fn parse_checksum_args(input: &str) -> IResult<&str, (PathBuf, u32)> {
    let (remaining_input, (path, crc)) = try_parse!(
        input,
        do_parse!(
            tag!("\"")
                >> path: is_not!(INVALID_PATH_CHARS)
                >> tag!("\"")
                >> ws!(tag!(","))
                >> crc: flat_map!(call!(hex_digit), parse_crc)
                >> (PathBuf::from(path), crc)
        )
    );

    if is_in_game_path(&path) {
        Ok((remaining_input, (path, crc)))
    } else {
        Err(Err::Failure(Context::Code(input, PARSE_PATH_ERROR)))
    }
}

fn parse_path(input: &str) -> IResult<&str, PathBuf> {
    let (remaining_input, path) =
        try_parse!(input, map!(is_not!(INVALID_PATH_CHARS), PathBuf::from));

    if is_in_game_path(&path) {
        Ok((remaining_input, path))
    } else {
        Err(Err::Failure(Context::Code(input, PARSE_PATH_ERROR)))
    }
}

/// Parse a string that is a path where the last component is a regex string
/// that may contain characters that are invalid in paths but valid in regex.
fn parse_regex_path(input: &str) -> IResult<&str, (PathBuf, Regex)> {
    let (remaining_input, string) = try_parse!(input, is_not!(INVALID_REGEX_PATH_CHARS));

    if string.ends_with('/') {
        return Err(Err::Failure(Context::Code(input, PARSE_PATH_ERROR)));
    }

    let (parent_path_slice, regex_slice) = string
        .rfind('/')
        .map(|i| (&string[..i], &string[i + 1..]))
        .unwrap_or_else(|| (".", string));

    let parent_path = PathBuf::from(parent_path_slice);

    if !is_in_game_path(&parent_path) {
        return Err(Err::Failure(Context::Code(input, PARSE_PATH_ERROR)));
    }

    let regex = parse_regex(regex_slice)?.1;

    Ok((remaining_input, (parent_path, regex)))
}

impl Function {
    pub fn parse(input: &str) -> IResult<&str, Function> {
        do_parse!(
            input,
            function:
                alt!(
                delimited!(tag!("file(\""), call!(parse_path), tag!("\")")) => {
                    |path| Function::FilePath(path)
                } |
                delimited!(tag!("file(\""), call!(parse_regex_path), tag!("\"")) => {
                    |(p, r)| Function::FileRegex(p, r)
                } |
                delimited!(tag!("active(\""), call!(parse_path), tag!("\")")) => {
                    |path| Function::ActivePath(path)
                } |
                delimited!(tag!("active(\""), flat_map!(is_not!(INVALID_REGEX_PATH_CHARS), parse_regex), tag!("\"")) => {
                    |r| Function::ActiveRegex(r)
                } |
                delimited!(tag!("many(\""), call!(parse_regex_path), tag!("\"")) => {
                    |(p, r)| Function::Many(p, r)
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
    fn function_parse_should_error_if_the_file_path_is_outside_the_game_directory() {
        assert!(Function::parse("file(\"../../Cargo.toml\")").is_err());
    }

    #[test]
    fn function_parse_should_parse_a_file_regex_function_with_no_parent_path() {
        let result = Function::parse("file(\"Cargo.*\")").unwrap().1;

        match result {
            Function::FileRegex(p, r) => {
                assert_eq!(PathBuf::from("."), p);
                assert_eq!(Regex::new("Cargo.*").unwrap().as_str(), r.as_str());
            }
            _ => panic!("Expected a file regex function"),
        }
    }

    #[test]
    fn function_parse_should_parse_a_file_regex_function_with_a_parent_path() {
        let result = Function::parse("file(\"subdir/Cargo.*\")").unwrap().1;

        match result {
            Function::FileRegex(p, r) => {
                assert_eq!(PathBuf::from("subdir"), p);
                assert_eq!(Regex::new("Cargo.*").unwrap().as_str(), r.as_str());
            }
            _ => panic!("Expected a file regex function"),
        }
    }

    #[test]
    fn function_parse_should_error_if_given_a_file_regex_function_ending_in_a_forward_slash() {
        assert!(Function::parse("file(\"sub\\dir/\")").is_err());
    }

    #[test]
    fn function_parse_should_error_if_the_file_regex_parent_path_is_outside_the_game_directory() {
        assert!(Function::parse("file(\"../../Cargo.*\")").is_err());
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
    fn function_parse_should_error_if_the_active_path_is_outside_the_game_directory() {
        // Trying to check if a path that isn't a plugin in the data folder is
        // active is pointless, but it's not worth having a more specific check.
        assert!(Function::parse("active(\"../../Cargo.toml\")").is_err());
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
    fn function_parse_should_parse_a_many_function_with_no_parent_path() {
        let result = Function::parse("many(\"Cargo.*\")").unwrap().1;

        match result {
            Function::Many(p, r) => {
                assert_eq!(PathBuf::from("."), p);
                assert_eq!(Regex::new("Cargo.*").unwrap().as_str(), r.as_str());
            }
            _ => panic!("Expected a many function"),
        }
    }

    #[test]
    fn function_parse_should_parse_a_many_function_with_a_parent_path() {
        let result = Function::parse("many(\"subdir/Cargo.*\")").unwrap().1;

        match result {
            Function::Many(p, r) => {
                assert_eq!(PathBuf::from("subdir"), p);
                assert_eq!(Regex::new("Cargo.*").unwrap().as_str(), r.as_str());
            }
            _ => panic!("Expected a many function"),
        }
    }

    #[test]
    fn function_parse_should_error_if_given_a_many_function_ending_in_a_forward_slash() {
        assert!(Function::parse("many(\"subdir/\")").is_err());
    }

    #[test]
    fn function_parse_should_error_if_the_many_parent_path_is_outside_the_game_directory() {
        assert!(Function::parse("file(\"../../Cargo.*\")").is_err());
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
    fn function_parse_should_error_if_the_checksum_path_is_outside_the_game_directory() {
        assert!(Function::parse("checksum(\"../../Cargo.toml\", DEADBEEF)").is_err());
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
    fn function_parse_should_error_if_the_version_path_is_outside_the_game_directory() {
        assert!(Function::parse("version(\"../../Cargo.toml\", \"1.2\", ==)").is_err());
    }
}
