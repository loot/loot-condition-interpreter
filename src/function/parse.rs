use std::path::{Component, Path, PathBuf};
use std::str;

use nom::types::CompleteStr;
use nom::{hex_digit, Context, Err, ErrorKind, IResult};
use regex::{Regex, RegexBuilder};

use super::{ComparisonOperator, Function};
use ParsingError;
use ParsingResult;

impl ComparisonOperator {
    pub fn parse(input: CompleteStr) -> IResult<CompleteStr, ComparisonOperator> {
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

fn parse_regex(input: CompleteStr) -> ParsingResult<Regex> {
    RegexBuilder::new(input.as_ref())
        .case_insensitive(true)
        .build()
        .map(|r| (CompleteStr(""), r))
        .map_err(|e| {
            Err::Failure(Context::Code(
                input,
                ErrorKind::Custom(ParsingError::from(e)),
            ))
        })
}

fn not_in_game_directory(input: CompleteStr, path: PathBuf) -> Err<CompleteStr, ParsingError> {
    Err::Failure(Context::Code(
        input,
        ErrorKind::Custom(ParsingError::PathIsNotInGameDirectory(path)),
    ))
}

fn parse_version_args(input: CompleteStr) -> ParsingResult<(PathBuf, String, ComparisonOperator)> {
    let (remaining_input, (path, version, comparator)) = try_parse!(
        input,
        fix_error!(
            ParsingError,
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
                    >> (PathBuf::from(path.as_ref()), version.to_string(), operator)
            )
        )
    );

    if is_in_game_path(&path) {
        Ok((remaining_input, (path, version, comparator)))
    } else {
        Err(not_in_game_directory(input, path))
    }
}

fn parse_crc(input: CompleteStr) -> ParsingResult<u32> {
    u32::from_str_radix(input.as_ref(), 16)
        .map(|c| (CompleteStr(""), c))
        .map_err(|e| {
            Err::Failure(Context::Code(
                input,
                ErrorKind::Custom(ParsingError::from(e)),
            ))
        })
}

fn parse_checksum_args(input: CompleteStr) -> ParsingResult<(PathBuf, u32)> {
    let (remaining_input, (path, crc)) = try_parse!(
        input,
        do_parse!(
            fix_error!(ParsingError, tag!("\""))
                >> path: fix_error!(ParsingError, is_not!(INVALID_PATH_CHARS))
                >> fix_error!(ParsingError, tag!("\""))
                >> fix_error!(ParsingError, ws!(tag!(",")))
                >> crc: flat_map!(fix_error!(ParsingError, hex_digit), parse_crc)
                >> (PathBuf::from(path.as_ref()), crc)
        )
    );

    if is_in_game_path(&path) {
        Ok((remaining_input, (path, crc)))
    } else {
        Err(not_in_game_directory(input, path))
    }
}

fn parse_path(input: CompleteStr) -> ParsingResult<PathBuf> {
    let (remaining_input, path) = try_parse!(
        input,
        fix_error!(
            ParsingError,
            map!(is_not!(INVALID_PATH_CHARS), |s| PathBuf::from(s.as_ref()))
        )
    );

    if is_in_game_path(&path) {
        Ok((remaining_input, path))
    } else {
        Err(not_in_game_directory(input, path))
    }
}

/// Parse a string that is a path where the last component is a regex string
/// that may contain characters that are invalid in paths but valid in regex.
fn parse_regex_path(input: CompleteStr) -> ParsingResult<(PathBuf, Regex)> {
    let (remaining_input, string) = try_parse!(
        input,
        fix_error!(ParsingError, is_not!(INVALID_REGEX_PATH_CHARS))
    );

    if string.ends_with('/') {
        return Err(Err::Failure(Context::Code(
            input,
            ErrorKind::Custom(ParsingError::PathEndsInADirectorySeparator(
                string.as_ref().into(),
            )),
        )));
    }

    let (parent_path_slice, regex_slice) = string
        .rfind('/')
        .map(|i| (&string[..i], &string[i + 1..]))
        .unwrap_or_else(|| (".", &string));

    let parent_path = PathBuf::from(parent_path_slice);

    if !is_in_game_path(&parent_path) {
        return Err(not_in_game_directory(input, parent_path));
    }

    let regex = parse_regex(CompleteStr(regex_slice))?.1;

    Ok((remaining_input, (parent_path, regex)))
}

impl Function {
    pub fn parse(input: CompleteStr) -> ParsingResult<Function> {
        do_parse!(
            input,
            function:
                alt!(
                delimited!(
                    fix_error!(ParsingError, tag!("file(\"")),
                    call!(parse_path),
                    fix_error!(ParsingError, tag!("\")"))
                ) => {
                    |path| Function::FilePath(path)
                } |
                delimited!(
                    fix_error!(ParsingError, tag!("file(\"")),
                    call!(parse_regex_path),
                    fix_error!(ParsingError, tag!("\""))
                ) => {
                    |(p, r)| Function::FileRegex(p, r)
                } |
                delimited!(
                    fix_error!(ParsingError, tag!("active(\"")),
                    call!(parse_path),
                    fix_error!(ParsingError, tag!("\")"))
                ) => {
                    |path| Function::ActivePath(path)
                } |
                delimited!(
                    fix_error!(ParsingError, tag!("active(\"")),
                    flat_map!(fix_error!(ParsingError, is_not!(INVALID_REGEX_PATH_CHARS)), parse_regex),
                    fix_error!(ParsingError, tag!("\""))
                ) => {
                    |r| Function::ActiveRegex(r)
                } |
                delimited!(
                    fix_error!(ParsingError, tag!("many(\"")),
                    call!(parse_regex_path),
                    fix_error!(ParsingError, tag!("\""))
                ) => {
                    |(p, r)| Function::Many(p, r)
                } |
                delimited!(
                    fix_error!(ParsingError, tag!("many_active(\"")),
                    flat_map!(fix_error!(ParsingError, is_not!(INVALID_REGEX_PATH_CHARS)), parse_regex),
                    fix_error!(ParsingError, tag!("\""))
                ) => {
                    |r| Function::ManyActive(r)
                } |
                delimited!(
                    fix_error!(ParsingError, tag!("version(")),
                    call!(parse_version_args),
                    fix_error!(ParsingError, tag!(")"))
                ) => {
                    |(path, version, comparator)| {
                        Function::Version(path, version, comparator)
                    }
                } |
                delimited!(
                    fix_error!(ParsingError, tag!("product_version(")),
                    call!(parse_version_args),
                    fix_error!(ParsingError, tag!(")"))
                ) => {
                    |(path, version, comparator)| {
                        Function::ProductVersion(path, version, comparator)
                    }
                } |
                delimited!(
                    fix_error!(ParsingError, tag!("checksum(")),
                    call!(parse_checksum_args),
                    fix_error!(ParsingError, tag!(")"))
                ) => {
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
    fn parse_regex_should_produce_case_insensitive_regex() {
        let (_, regex) = parse_regex("cargo.*".into()).unwrap();

        assert!(regex.is_match("Cargo.toml"));
    }

    #[test]
    fn function_parse_should_parse_a_file_path_function() {
        let result = Function::parse("file(\"Cargo.toml\")".into()).unwrap().1;

        match result {
            Function::FilePath(f) => assert_eq!(Path::new("Cargo.toml"), f),
            _ => panic!("Expected a file path function"),
        }
    }

    #[test]
    fn function_parse_should_error_if_the_file_path_is_outside_the_game_directory() {
        assert!(Function::parse("file(\"../../Cargo.toml\")".into()).is_err());
    }

    #[test]
    fn function_parse_should_parse_a_file_regex_function_with_no_parent_path() {
        let result = Function::parse("file(\"Cargo.*\")".into()).unwrap().1;

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
        let result = Function::parse("file(\"subdir/Cargo.*\")".into())
            .unwrap()
            .1;

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
        assert!(Function::parse("file(\"sub\\dir/\")".into()).is_err());
    }

    #[test]
    fn function_parse_should_error_if_the_file_regex_parent_path_is_outside_the_game_directory() {
        assert!(Function::parse("file(\"../../Cargo.*\")".into()).is_err());
    }

    #[test]
    fn function_parse_should_parse_an_active_path_function() {
        let result = Function::parse("active(\"Cargo.toml\")".into()).unwrap().1;

        match result {
            Function::ActivePath(f) => assert_eq!(Path::new("Cargo.toml"), f),
            _ => panic!("Expected an active path function"),
        }
    }

    #[test]
    fn function_parse_should_error_if_the_active_path_is_outside_the_game_directory() {
        // Trying to check if a path that isn't a plugin in the data folder is
        // active is pointless, but it's not worth having a more specific check.
        assert!(Function::parse("active(\"../../Cargo.toml\")".into()).is_err());
    }

    #[test]
    fn function_parse_should_parse_an_active_regex_function() {
        let result = Function::parse("active(\"Cargo.*\")".into()).unwrap().1;

        match result {
            Function::ActiveRegex(r) => {
                assert_eq!(Regex::new("Cargo.*").unwrap().as_str(), r.as_str())
            }
            _ => panic!("Expected an active regex function"),
        }
    }

    #[test]
    fn function_parse_should_parse_a_many_function_with_no_parent_path() {
        let result = Function::parse("many(\"Cargo.*\")".into()).unwrap().1;

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
        let result = Function::parse("many(\"subdir/Cargo.*\")".into())
            .unwrap()
            .1;

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
        assert!(Function::parse("many(\"subdir/\")".into()).is_err());
    }

    #[test]
    fn function_parse_should_error_if_the_many_parent_path_is_outside_the_game_directory() {
        assert!(Function::parse("file(\"../../Cargo.*\")".into()).is_err());
    }

    #[test]
    fn function_parse_should_parse_a_many_active_function() {
        let result = Function::parse("many_active(\"Cargo.*\")".into())
            .unwrap()
            .1;

        match result {
            Function::ManyActive(r) => {
                assert_eq!(Regex::new("Cargo.*").unwrap().as_str(), r.as_str())
            }
            _ => panic!("Expected a many active function"),
        }
    }

    #[test]
    fn function_parse_should_parse_a_checksum_function() {
        let result = Function::parse("checksum(\"Cargo.toml\", DEADBEEF)".into())
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
        assert!(Function::parse("checksum(\"../../Cargo.toml\", DEADBEEF)".into()).is_err());
    }

    #[test]
    fn function_parse_should_parse_a_version_equals_function() {
        let result = Function::parse("version(\"Cargo.toml\", \"1.2\", ==)".into())
            .unwrap()
            .1;

        match result {
            Function::Version(path, version, comparator) => {
                assert_eq!(Path::new("Cargo.toml"), path);
                assert_eq!("1.2", version);
                assert_eq!(ComparisonOperator::Equal, comparator);
            }
            _ => panic!("Expected a version function"),
        }
    }

    #[test]
    fn function_parse_should_error_if_the_version_path_is_outside_the_game_directory() {
        assert!(Function::parse("version(\"../../Cargo.toml\", \"1.2\", ==)".into()).is_err());
    }

    #[test]
    fn function_parse_should_parse_a_product_version_equals_function() {
        let result = Function::parse("product_version(\"Cargo.toml\", \"1.2\", ==)".into())
            .unwrap()
            .1;

        match result {
            Function::ProductVersion(path, version, comparator) => {
                assert_eq!(Path::new("Cargo.toml"), path);
                assert_eq!("1.2", version);
                assert_eq!(ComparisonOperator::Equal, comparator);
            }
            _ => panic!("Expected a product version function"),
        }
    }

    #[test]
    fn function_parse_should_error_if_the_product_version_path_is_outside_the_game_directory() {
        assert!(
            Function::parse("product_version(\"../../Cargo.toml\", \"1.2\", ==)".into()).is_err()
        );
    }
}
