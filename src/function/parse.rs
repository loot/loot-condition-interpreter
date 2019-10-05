use std::path::{Component, Path, PathBuf};
use std::str;

use nom::branch::alt;
use nom::bytes::complete::{is_not, tag};
use nom::character::complete::hex_digit1;
use nom::combinator::{map, map_parser, value};
use nom::sequence::{delimited, tuple};
use nom::{Err, IResult};
use regex::{Regex, RegexBuilder};

use super::{ComparisonOperator, Function};
use crate::error::ParsingErrorKind;
use crate::{map_err, whitespace, ParsingError, ParsingResult};

impl ComparisonOperator {
    pub fn parse(input: &str) -> IResult<&str, ComparisonOperator> {
        alt((
            value(ComparisonOperator::Equal, tag("==")),
            value(ComparisonOperator::NotEqual, tag("!=")),
            value(ComparisonOperator::LessThanOrEqual, tag("<=")),
            value(ComparisonOperator::GreaterThanOrEqual, tag(">=")),
            value(ComparisonOperator::LessThan, tag("<")),
            value(ComparisonOperator::GreaterThan, tag(">")),
        ))(input)
    }
}

const INVALID_PATH_CHARS: &str = "\":*?<>|";
const INVALID_NON_REGEX_PATH_CHARS: &str = "\":*?<>|\\"; // \ is treated as invalid to distinguish regex strings.
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
fn parse_regex(input: &str) -> ParsingResult<Regex> {
    RegexBuilder::new(&format!("^{}$", input))
        .case_insensitive(true)
        .build()
        .map(|r| ("", r))
        .map_err(|e| Err::Failure(ParsingErrorKind::from(e).at(input)))
}

fn not_in_game_directory(input: &str, path: PathBuf) -> Err<ParsingError<&str>> {
    Err::Failure(ParsingErrorKind::PathIsNotInGameDirectory(path).at(input))
}

fn parse_path(input: &str) -> IResult<&str, PathBuf> {
    map(
        delimited(tag("\""), is_not(INVALID_PATH_CHARS), tag("\"")),
        PathBuf::from,
    )(input)
}

fn parse_version_args(input: &str) -> ParsingResult<(PathBuf, String, ComparisonOperator)> {
    let version_parser = map(
        delimited(tag("\""), is_not("\""), tag("\"")),
        |version: &str| version.to_string(),
    );

    let parser = tuple((
        parse_path,
        whitespace(tag(",")),
        version_parser,
        whitespace(tag(",")),
        ComparisonOperator::parse,
    ));

    let (remaining_input, (path, _, version, _, comparator)) = map_err(parser)(input)?;

    if is_in_game_path(&path) {
        Ok((remaining_input, (path, version, comparator)))
    } else {
        Err(not_in_game_directory(input, path))
    }
}

fn parse_crc(input: &str) -> ParsingResult<u32> {
    u32::from_str_radix(input, 16)
        .map(|c| ("", c))
        .map_err(|e| Err::Failure(ParsingErrorKind::from(e).at(input)))
}

fn parse_checksum_args(input: &str) -> ParsingResult<(PathBuf, u32)> {
    let parser = tuple((
        map_err(parse_path),
        map_err(whitespace(tag(","))),
        map_parser(hex_digit1, parse_crc),
    ));

    let (remaining_input, (path, _, crc)) = parser(input)?;

    if is_in_game_path(&path) {
        Ok((remaining_input, (path, crc)))
    } else {
        Err(not_in_game_directory(input, path))
    }
}

fn parse_non_regex_path(input: &str) -> ParsingResult<PathBuf> {
    let (remaining_input, path) = map(is_not(INVALID_NON_REGEX_PATH_CHARS), |path: &str| {
        PathBuf::from(path)
    })(input)?;

    if is_in_game_path(&path) {
        Ok((remaining_input, path))
    } else {
        Err(not_in_game_directory(input, path))
    }
}

/// Parse a string that is a path where the last component is a regex string
/// that may contain characters that are invalid in paths but valid in regex.
fn parse_regex_path(input: &str) -> ParsingResult<(PathBuf, Regex)> {
    let (remaining_input, string) = is_not(INVALID_REGEX_PATH_CHARS)(input)?;

    if string.ends_with('/') {
        return Err(Err::Failure(
            ParsingErrorKind::PathEndsInADirectorySeparator(string.into()).at(input),
        ));
    }

    let (parent_path_slice, regex_slice) = string
        .rfind('/')
        .map(|i| (&string[..i], &string[i + 1..]))
        .unwrap_or_else(|| (".", &string));

    let parent_path = PathBuf::from(parent_path_slice);

    if !is_in_game_path(&parent_path) {
        return Err(not_in_game_directory(input, parent_path));
    }

    let regex = parse_regex(regex_slice)?.1;

    Ok((remaining_input, (parent_path, regex)))
}

fn parse_regex_filename(input: &str) -> ParsingResult<Regex> {
    map_parser(is_not(INVALID_REGEX_PATH_CHARS), parse_regex)(input)
}

impl Function {
    pub fn parse(input: &str) -> ParsingResult<Function> {
        alt((
            map(
                delimited(
                    map_err(tag("file(\"")),
                    parse_non_regex_path,
                    map_err(tag("\")")),
                ),
                Function::FilePath,
            ),
            map(
                delimited(
                    map_err(tag("file(\"")),
                    parse_regex_path,
                    map_err(tag("\")")),
                ),
                |(path, regex)| Function::FileRegex(path, regex),
            ),
            map(
                delimited(
                    map_err(tag("active(\"")),
                    parse_non_regex_path,
                    map_err(tag("\")")),
                ),
                Function::ActivePath,
            ),
            map(
                delimited(
                    map_err(tag("active(\"")),
                    parse_regex_filename,
                    map_err(tag("\")")),
                ),
                Function::ActiveRegex,
            ),
            map(
                delimited(
                    map_err(tag("is_master(\"")),
                    parse_non_regex_path,
                    map_err(tag("\")")),
                ),
                Function::IsMaster,
            ),
            map(
                delimited(
                    map_err(tag("many(\"")),
                    parse_regex_path,
                    map_err(tag("\")")),
                ),
                |(path, regex)| Function::Many(path, regex),
            ),
            map(
                delimited(
                    map_err(tag("many_active(\"")),
                    parse_regex_filename,
                    map_err(tag("\")")),
                ),
                Function::ManyActive,
            ),
            map(
                delimited(
                    map_err(tag("version(")),
                    parse_version_args,
                    map_err(tag(")")),
                ),
                |(path, version, comparator)| Function::Version(path, version, comparator),
            ),
            map(
                delimited(
                    map_err(tag("product_version(")),
                    parse_version_args,
                    map_err(tag(")")),
                ),
                |(path, version, comparator)| Function::ProductVersion(path, version, comparator),
            ),
            map(
                delimited(
                    map_err(tag("checksum(")),
                    parse_checksum_args,
                    map_err(tag(")")),
                ),
                |(path, crc)| Function::Checksum(path, crc),
            ),
        ))(input)
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
    fn parse_regex_should_produce_a_regex_that_does_not_partially_match() {
        let (_, regex) = parse_regex("cargo.".into()).unwrap();

        assert!(!regex.is_match("Cargo.toml"));
    }

    #[test]
    fn function_parse_should_parse_a_file_path_function() {
        let output = Function::parse("file(\"Cargo.toml\")".into()).unwrap();

        assert!(output.0.is_empty());
        match output.1 {
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
        let output = Function::parse("file(\"Cargo.*\")".into()).unwrap();

        assert!(output.0.is_empty());
        match output.1 {
            Function::FileRegex(p, r) => {
                assert_eq!(PathBuf::from("."), p);
                assert_eq!(Regex::new("^Cargo.*$").unwrap().as_str(), r.as_str());
            }
            _ => panic!("Expected a file regex function"),
        }
    }

    #[test]
    fn function_parse_should_parse_a_file_regex_function_with_a_parent_path() {
        let output = Function::parse("file(\"subdir/Cargo.*\")".into()).unwrap();

        assert!(output.0.is_empty());
        match output.1 {
            Function::FileRegex(p, r) => {
                assert_eq!(PathBuf::from("subdir"), p);
                assert_eq!(Regex::new("^Cargo.*$").unwrap().as_str(), r.as_str());
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
        let output = Function::parse("active(\"Cargo.toml\")".into()).unwrap();

        assert!(output.0.is_empty());
        match output.1 {
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
        let output = Function::parse("active(\"Cargo.*\")".into()).unwrap();

        assert!(output.0.is_empty());
        match output.1 {
            Function::ActiveRegex(r) => {
                assert_eq!(Regex::new("^Cargo.*$").unwrap().as_str(), r.as_str())
            }
            _ => panic!("Expected an active regex function"),
        }
    }

    #[test]
    fn function_parse_should_parse_an_is_master_function() {
        let output = Function::parse("is_master(\"Blank.esm\")".into()).unwrap();

        assert!(output.0.is_empty());
        match output.1 {
            Function::IsMaster(f) => assert_eq!(Path::new("Blank.esm"), f),
            _ => panic!("Expected an is master function"),
        }
    }

    #[test]
    fn function_parse_should_error_if_the_is_master_path_is_outside_the_game_directory() {
        // Trying to check if a path that isn't a plugin in the data folder is
        // active is pointless, but it's not worth having a more specific check.
        assert!(Function::parse("is_master(\"../../Blank.esm\")".into()).is_err());
    }

    #[test]
    fn function_parse_should_parse_a_many_function_with_no_parent_path() {
        let output = Function::parse("many(\"Cargo.*\")".into()).unwrap();

        assert!(output.0.is_empty());
        match output.1 {
            Function::Many(p, r) => {
                assert_eq!(PathBuf::from("."), p);
                assert_eq!(Regex::new("^Cargo.*$").unwrap().as_str(), r.as_str());
            }
            _ => panic!("Expected a many function"),
        }
    }

    #[test]
    fn function_parse_should_parse_a_many_function_with_a_parent_path() {
        let output = Function::parse("many(\"subdir/Cargo.*\")".into()).unwrap();

        assert!(output.0.is_empty());
        match output.1 {
            Function::Many(p, r) => {
                assert_eq!(PathBuf::from("subdir"), p);
                assert_eq!(Regex::new("^Cargo.*$").unwrap().as_str(), r.as_str());
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
        let output = Function::parse("many_active(\"Cargo.*\")".into()).unwrap();

        assert!(output.0.is_empty());
        match output.1 {
            Function::ManyActive(r) => {
                assert_eq!(Regex::new("^Cargo.*$").unwrap().as_str(), r.as_str())
            }
            _ => panic!("Expected a many active function"),
        }
    }

    #[test]
    fn function_parse_should_parse_a_checksum_function() {
        let output = Function::parse("checksum(\"Cargo.toml\", DEADBEEF)".into()).unwrap();

        assert!(output.0.is_empty());
        match output.1 {
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
        let output = Function::parse("version(\"Cargo.toml\", \"1.2\", ==)".into()).unwrap();

        assert!(output.0.is_empty());
        match output.1 {
            Function::Version(path, version, comparator) => {
                assert_eq!(Path::new("Cargo.toml"), path);
                assert_eq!("1.2", version);
                assert_eq!(ComparisonOperator::Equal, comparator);
            }
            _ => panic!("Expected a version function"),
        }
    }

    #[test]
    fn function_parse_should_parse_a_version_not_equals_function() {
        let output = Function::parse("version(\"Cargo.toml\", \"1.2\", !=)".into()).unwrap();

        assert!(output.0.is_empty());
        match output.1 {
            Function::Version(path, version, comparator) => {
                assert_eq!(Path::new("Cargo.toml"), path);
                assert_eq!("1.2", version);
                assert_eq!(ComparisonOperator::NotEqual, comparator);
            }
            _ => panic!("Expected a version function"),
        }
    }

    #[test]
    fn function_parse_should_parse_a_version_less_than_function() {
        let output = Function::parse("version(\"Cargo.toml\", \"1.2\", <)".into()).unwrap();

        assert!(output.0.is_empty());
        match output.1 {
            Function::Version(path, version, comparator) => {
                assert_eq!(Path::new("Cargo.toml"), path);
                assert_eq!("1.2", version);
                assert_eq!(ComparisonOperator::LessThan, comparator);
            }
            _ => panic!("Expected a version function"),
        }
    }

    #[test]
    fn function_parse_should_parse_a_version_greater_than_function() {
        let output = Function::parse("version(\"Cargo.toml\", \"1.2\", >)".into()).unwrap();

        assert!(output.0.is_empty());
        match output.1 {
            Function::Version(path, version, comparator) => {
                assert_eq!(Path::new("Cargo.toml"), path);
                assert_eq!("1.2", version);
                assert_eq!(ComparisonOperator::GreaterThan, comparator);
            }
            _ => panic!("Expected a version function"),
        }
    }

    #[test]
    fn function_parse_should_parse_a_version_less_than_or_equal_to_function() {
        let output = Function::parse("version(\"Cargo.toml\", \"1.2\", <=)".into()).unwrap();

        assert!(output.0.is_empty());
        match output.1 {
            Function::Version(path, version, comparator) => {
                assert_eq!(Path::new("Cargo.toml"), path);
                assert_eq!("1.2", version);
                assert_eq!(ComparisonOperator::LessThanOrEqual, comparator);
            }
            _ => panic!("Expected a version function"),
        }
    }

    #[test]
    fn function_parse_should_parse_a_version_greater_than_or_equal_to_function() {
        let output = Function::parse("version(\"Cargo.toml\", \"1.2\", >=)".into()).unwrap();

        assert!(output.0.is_empty());
        match output.1 {
            Function::Version(path, version, comparator) => {
                assert_eq!(Path::new("Cargo.toml"), path);
                assert_eq!("1.2", version);
                assert_eq!(ComparisonOperator::GreaterThanOrEqual, comparator);
            }
            _ => panic!("Expected a version function"),
        }
    }

    #[test]
    fn function_parse_should_parse_a_version_with_a_path_containing_backslashes() {
        let output = Function::parse("version(\"..\\Cargo.toml\", \"1.2\", ==)".into()).unwrap();

        assert!(output.0.is_empty());
        match output.1 {
            Function::Version(path, version, comparator) => {
                assert_eq!(Path::new("..\\Cargo.toml"), path);
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
        let output =
            Function::parse("product_version(\"Cargo.toml\", \"1.2\", ==)".into()).unwrap();

        assert!(output.0.is_empty());
        match output.1 {
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
