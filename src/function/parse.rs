use std::path::PathBuf;
use std::str;

use nom::branch::alt;
use nom::bytes::complete::{is_not, tag};
use nom::character::complete::digit1;
use nom::character::complete::hex_digit1;
use nom::combinator::{map, map_parser, value};
use nom::sequence::delimited;
use nom::{Err, IResult, Parser};
use regex::{Regex, RegexBuilder};

use super::{ComparisonOperator, Function};
use crate::error::ParsingErrorKind;
use crate::{map_err, whitespace, ParsingResult};

impl ComparisonOperator {
    pub fn parse(input: &str) -> IResult<&str, ComparisonOperator> {
        alt((
            value(ComparisonOperator::Equal, tag("==")),
            value(ComparisonOperator::NotEqual, tag("!=")),
            value(ComparisonOperator::LessThanOrEqual, tag("<=")),
            value(ComparisonOperator::GreaterThanOrEqual, tag(">=")),
            value(ComparisonOperator::LessThan, tag("<")),
            value(ComparisonOperator::GreaterThan, tag(">")),
        ))
        .parse(input)
    }
}

const INVALID_PATH_CHARS: &str = "\":*?<>|";
const INVALID_NON_REGEX_PATH_CHARS: &str = "\":*?<>|\\"; // \ is treated as invalid to distinguish regex strings.
const INVALID_REGEX_PATH_CHARS: &str = "\"<>";

fn build_regex(input: &str) -> Result<(&'static str, Regex), regex::Error> {
    RegexBuilder::new(input)
        .case_insensitive(true)
        .build()
        .map(|r| ("", r))
}

fn parse_regex(input: &str) -> ParsingResult<Regex> {
    build_regex(input).map_err(|e| Err::Failure(ParsingErrorKind::from(e).at(input)))
}

fn parse_anchored_regex(input: &str) -> ParsingResult<Regex> {
    build_regex(&format!("^{input}$"))
        .map_err(|e| Err::Failure(ParsingErrorKind::from(e).at(input)))
}

fn parse_path(input: &str) -> IResult<&str, PathBuf> {
    map(
        delimited(tag("\""), is_not(INVALID_PATH_CHARS), tag("\"")),
        PathBuf::from,
    )
    .parse(input)
}

fn parse_size(input: &str) -> ParsingResult<u64> {
    str::parse(input)
        .map(|c| ("", c))
        .map_err(|e| Err::Failure(ParsingErrorKind::from(e).at(input)))
}

fn parse_file_size_args(input: &str) -> ParsingResult<(PathBuf, u64)> {
    let mut parser = (
        map_err(parse_path),
        map_err(whitespace(tag(","))),
        map_parser(digit1, parse_size),
    );

    let (remaining_input, (path, _, size)) = parser.parse(input)?;

    Ok((remaining_input, (path, size)))
}

fn parse_version(input: &str) -> IResult<&str, String> {
    map(
        delimited(tag("\""), is_not("\""), tag("\"")),
        |version: &str| version.to_owned(),
    )
    .parse(input)
}

fn parse_version_args(input: &str) -> ParsingResult<(PathBuf, String, ComparisonOperator)> {
    let parser = (
        parse_path,
        whitespace(tag(",")),
        parse_version,
        whitespace(tag(",")),
        ComparisonOperator::parse,
    );

    let (remaining_input, (path, _, version, _, comparator)) = map_err(parser).parse(input)?;

    Ok((remaining_input, (path, version, comparator)))
}

fn parse_filename_version_args(
    input: &str,
) -> ParsingResult<(PathBuf, Regex, String, ComparisonOperator)> {
    let mut parser = (
        delimited(map_err(tag("\"")), parse_regex_path, map_err(tag("\""))),
        map_err(whitespace(tag(","))),
        map_err(parse_version),
        map_err(whitespace(tag(","))),
        map_err(ComparisonOperator::parse),
    );

    let (remaining_input, ((path, regex), _, version, _, comparator)) = parser.parse(input)?;

    if regex.captures_len() != 2 {
        return Err(Err::Failure(
            ParsingErrorKind::InvalidRegexUnknown.at(input),
        ));
    }

    Ok((remaining_input, (path, regex, version, comparator)))
}

fn parse_description_contains_args(input: &str) -> ParsingResult<(PathBuf, Regex)> {
    let mut parser = (
        map_err(parse_path),
        map_err(whitespace(tag(","))),
        delimited(
            map_err(tag("\"")),
            map_parser(is_not("\""), parse_regex),
            map_err(tag("\"")),
        ),
    );

    let (remaining_input, (path, _, regex)) = parser.parse(input)?;

    Ok((remaining_input, (path, regex)))
}

fn parse_crc(input: &str) -> ParsingResult<u32> {
    u32::from_str_radix(input, 16)
        .map(|c| ("", c))
        .map_err(|e| Err::Failure(ParsingErrorKind::from(e).at(input)))
}

fn parse_checksum_args(input: &str) -> ParsingResult<(PathBuf, u32)> {
    let mut parser = (
        map_err(parse_path),
        map_err(whitespace(tag(","))),
        map_parser(hex_digit1, parse_crc),
    );

    let (remaining_input, (path, _, crc)) = parser.parse(input)?;

    Ok((remaining_input, (path, crc)))
}

fn parse_non_regex_path(input: &str) -> ParsingResult<PathBuf> {
    let (remaining_input, path) = map(is_not(INVALID_NON_REGEX_PATH_CHARS), |path: &str| {
        PathBuf::from(path)
    })
    .parse(input)?;

    Ok((remaining_input, path))
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

    let (parent_path_slice, regex_slice) = string.rsplit_once('/').unwrap_or((".", string));

    let parent_path = PathBuf::from(parent_path_slice);

    let regex = parse_anchored_regex(regex_slice)?.1;

    Ok((remaining_input, (parent_path, regex)))
}

fn parse_regex_filename(input: &str) -> ParsingResult<Regex> {
    map_parser(is_not(INVALID_REGEX_PATH_CHARS), parse_anchored_regex).parse(input)
}

impl Function {
    #[expect(clippy::too_many_lines)]
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
                    map_err(tag("file_size(")),
                    parse_file_size_args,
                    map_err(tag(")")),
                ),
                |(path, size)| Function::FileSize(path, size),
            ),
            map(
                delimited(
                    map_err(tag("readable(\"")),
                    parse_non_regex_path,
                    map_err(tag("\")")),
                ),
                Function::Readable,
            ),
            map(
                delimited(
                    map_err(tag("is_executable(\"")),
                    parse_non_regex_path,
                    map_err(tag("\")")),
                ),
                Function::IsExecutable,
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
                    map_err(tag("filename_version(")),
                    parse_filename_version_args,
                    map_err(tag(")")),
                ),
                |(path, regex, version, comparator)| {
                    Function::FilenameVersion(path, regex, version, comparator)
                },
            ),
            map(
                delimited(
                    map_err(tag("checksum(")),
                    parse_checksum_args,
                    map_err(tag(")")),
                ),
                |(path, crc)| Function::Checksum(path, crc),
            ),
            map(
                delimited(
                    map_err(tag("description_contains(")),
                    parse_description_contains_args,
                    map_err(tag(")")),
                ),
                |(path, regex)| Function::DescriptionContains(path, regex),
            ),
        ))
        .parse(input)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn parse_regex_should_produce_case_insensitive_regex() {
        let (_, regex) = parse_regex("cargo.*").unwrap();

        assert!(regex.is_match("Cargo.toml"));
    }

    #[test]
    fn parse_regex_should_produce_a_regex_that_does_partially_match() {
        let (_, regex) = parse_regex("argo.").unwrap();

        assert!(regex.is_match("Cargo.toml"));
    }

    #[test]
    fn parse_anchored_regex_should_produce_case_insensitive_regex() {
        let (_, regex) = parse_anchored_regex("cargo.*").unwrap();

        assert!(regex.is_match("Cargo.toml"));
    }

    #[test]
    fn parse_anchored_regex_should_produce_a_regex_that_does_not_partially_match() {
        let (_, regex) = parse_anchored_regex("cargo.").unwrap();

        assert!(!regex.is_match("Cargo.toml"));
    }

    #[test]
    fn function_parse_should_parse_a_file_path_function() {
        let output = Function::parse("file(\"Cargo.toml\")").unwrap();

        assert!(output.0.is_empty());
        match output.1 {
            Function::FilePath(f) => assert_eq!(Path::new("Cargo.toml"), f),
            _ => panic!("Expected a file path function"),
        }
    }

    #[test]
    fn function_parse_should_parse_a_file_regex_function_with_no_parent_path() {
        let output = Function::parse("file(\"Cargo.*\")").unwrap();

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
        let output = Function::parse("file(\"subdir/Cargo.*\")").unwrap();

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
        assert!(Function::parse("file(\"sub\\dir/\")").is_err());
    }

    #[test]
    fn function_parse_should_parse_a_file_size_function() {
        let output = Function::parse("file_size(\"Cargo.toml\", 1234)").unwrap();

        assert!(output.0.is_empty());
        match output.1 {
            Function::FileSize(f, s) => {
                assert_eq!(Path::new("Cargo.toml"), f);
                assert_eq!(1234, s);
            }
            _ => panic!("Expected a file size function"),
        }
    }

    #[test]
    fn function_parse_should_parse_a_readable_function() {
        let output = Function::parse("readable(\"Cargo.toml\")").unwrap();

        assert!(output.0.is_empty());
        match output.1 {
            Function::Readable(f) => assert_eq!(Path::new("Cargo.toml"), f),
            _ => panic!("Expected a readable function"),
        }
    }

    #[test]
    fn function_parse_should_parse_an_is_executable_function() {
        let output = Function::parse("is_executable(\"Cargo.toml\")").unwrap();

        assert!(output.0.is_empty());
        match output.1 {
            Function::IsExecutable(f) => assert_eq!(Path::new("Cargo.toml"), f),
            _ => panic!("Expected an is_executable function"),
        }
    }

    #[test]
    fn function_parse_should_parse_an_active_path_function() {
        let output = Function::parse("active(\"Cargo.toml\")").unwrap();

        assert!(output.0.is_empty());
        match output.1 {
            Function::ActivePath(f) => assert_eq!(Path::new("Cargo.toml"), f),
            _ => panic!("Expected an active path function"),
        }
    }

    #[test]
    fn function_parse_should_parse_an_active_regex_function() {
        let output = Function::parse("active(\"Cargo.*\")").unwrap();

        assert!(output.0.is_empty());
        match output.1 {
            Function::ActiveRegex(r) => {
                assert_eq!(Regex::new("^Cargo.*$").unwrap().as_str(), r.as_str());
            }
            _ => panic!("Expected an active regex function"),
        }
    }

    #[test]
    fn function_parse_should_parse_an_is_master_function() {
        let output = Function::parse("is_master(\"Blank.esm\")").unwrap();

        assert!(output.0.is_empty());
        match output.1 {
            Function::IsMaster(f) => assert_eq!(Path::new("Blank.esm"), f),
            _ => panic!("Expected an is master function"),
        }
    }

    #[test]
    fn function_parse_should_parse_a_many_function_with_no_parent_path() {
        let output = Function::parse("many(\"Cargo.*\")").unwrap();

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
        let output = Function::parse("many(\"subdir/Cargo.*\")").unwrap();

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
        assert!(Function::parse("many(\"subdir/\")").is_err());
    }

    #[test]
    fn function_parse_should_parse_a_many_active_function() {
        let output = Function::parse("many_active(\"Cargo.*\")").unwrap();

        assert!(output.0.is_empty());
        match output.1 {
            Function::ManyActive(r) => {
                assert_eq!(Regex::new("^Cargo.*$").unwrap().as_str(), r.as_str());
            }
            _ => panic!("Expected a many active function"),
        }
    }

    #[test]
    fn function_parse_should_parse_a_checksum_function() {
        let output = Function::parse("checksum(\"Cargo.toml\", DEADBEEF)").unwrap();

        assert!(output.0.is_empty());
        match output.1 {
            Function::Checksum(path, crc) => {
                assert_eq!(Path::new("Cargo.toml"), path);
                assert_eq!(0xDEAD_BEEF, crc);
            }
            _ => panic!("Expected a checksum function"),
        }
    }

    #[test]
    fn function_parse_should_parse_a_version_equals_function() {
        let output = Function::parse("version(\"Cargo.toml\", \"1.2\", ==)").unwrap();

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
        let output = Function::parse("version(\"Cargo.toml\", \"1.2\", !=)").unwrap();

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
        let output = Function::parse("version(\"Cargo.toml\", \"1.2\", <)").unwrap();

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
        let output = Function::parse("version(\"Cargo.toml\", \"1.2\", >)").unwrap();

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
        let output = Function::parse("version(\"Cargo.toml\", \"1.2\", <=)").unwrap();

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
        let output = Function::parse("version(\"Cargo.toml\", \"1.2\", >=)").unwrap();

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
        let output = Function::parse("version(\"..\\Cargo.toml\", \"1.2\", ==)").unwrap();

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
    fn function_parse_should_parse_a_product_version_equals_function() {
        let output = Function::parse("product_version(\"Cargo.toml\", \"1.2\", ==)").unwrap();

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
    fn function_parse_should_parse_a_filename_version_equals_function() {
        let output =
            Function::parse("filename_version(\"subdir/Cargo (.+).toml\", \"1.2\", ==)").unwrap();

        assert!(output.0.is_empty());
        match output.1 {
            Function::FilenameVersion(path, regex, version, comparator) => {
                assert_eq!(PathBuf::from("subdir"), path);
                assert_eq!(
                    Regex::new("^Cargo (.+).toml$").unwrap().as_str(),
                    regex.as_str()
                );
                assert_eq!("1.2", version);
                assert_eq!(ComparisonOperator::Equal, comparator);
            }
            _ => panic!("Expected a filename version function"),
        }
    }

    #[test]
    fn function_parse_should_error_if_the_filename_version_regex_does_not_contain_an_explicit_capture_group(
    ) {
        assert!(
            Function::parse("filename_version(\"subdir/Cargo .+.toml\", \"1.2\", ==)").is_err()
        );
    }

    #[test]
    fn function_parse_should_parse_a_description_contains_function() {
        let lowercase_non_ascii = "\u{20ac}\u{192}.";
        let function = format!("description_contains(\"Blank.esp\", \"{lowercase_non_ascii}\")");
        let output = Function::parse(&function).unwrap();

        assert!(output.0.is_empty());
        match output.1 {
            Function::DescriptionContains(p, r) => {
                assert_eq!(PathBuf::from("Blank.esp"), p);
                assert_eq!(
                    Regex::new(lowercase_non_ascii).unwrap().as_str(),
                    r.as_str()
                );
            }
            _ => panic!("Expected a description_contains function"),
        }
    }
}
