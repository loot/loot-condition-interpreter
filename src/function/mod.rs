use std::path::PathBuf;

use regex::Regex;
use unicase::eq;

pub mod eval;
pub mod parse;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
    FileRegex(PathBuf, Regex),
    ActivePath(PathBuf),
    ActiveRegex(Regex),
    Many(PathBuf, Regex),
    ManyActive(Regex),
    Checksum(PathBuf, u32),
    Version(PathBuf, String, ComparisonOperator),
}

impl PartialEq for Function {
    fn eq(&self, other: &Function) -> bool {
        use Function::*;
        match (self, other) {
            (FilePath(p1), FilePath(p2)) => eq(&p1.to_string_lossy(), &p2.to_string_lossy()),
            (FileRegex(p1, r1), FileRegex(p2, r2)) => {
                eq(r1.as_str(), r2.as_str()) && eq(&p1.to_string_lossy(), &p2.to_string_lossy())
            }
            (ActivePath(p1), ActivePath(p2)) => eq(&p1.to_string_lossy(), &p2.to_string_lossy()),
            (ActiveRegex(r1), ActiveRegex(r2)) => eq(r1.as_str(), r2.as_str()),
            (Many(p1, r1), Many(p2, r2)) => {
                eq(r1.as_str(), r2.as_str()) && eq(&p1.to_string_lossy(), &p2.to_string_lossy())
            }
            (ManyActive(r1), ManyActive(r2)) => eq(r1.as_str(), r2.as_str()),
            (Checksum(p1, c1), Checksum(p2, c2)) => {
                c1 == c2 && eq(&p1.to_string_lossy(), &p2.to_string_lossy())
            }
            (Version(p1, v1, c1), Version(p2, v2, c2)) => {
                c1 == c2 && eq(&v1, &v2) && eq(&p1.to_string_lossy(), &p2.to_string_lossy())
            }
            _ => false,
        }
    }
}

impl Eq for Function {}

#[cfg(test)]
mod tests {
    use super::*;

    fn regex(string: &str) -> Regex {
        Regex::new(string).unwrap()
    }

    #[test]
    fn function_eq_for_file_path_should_check_pathbuf() {
        assert_eq!(
            Function::FilePath("Blank.esm".into()),
            Function::FilePath("Blank.esm".into())
        );

        assert_ne!(
            Function::FilePath("Blank.esp".into()),
            Function::FilePath("Blank.esm".into())
        );
    }

    #[test]
    fn function_eq_for_file_path_should_be_case_insensitive_on_pathbuf() {
        assert_eq!(
            Function::FilePath("Blank.esm".into()),
            Function::FilePath("blank.esm".into())
        );
    }

    #[test]
    fn function_eq_for_file_regex_should_check_pathbuf_and_regex() {
        assert_eq!(
            Function::FileRegex("subdir".into(), regex("blank.*")),
            Function::FileRegex("subdir".into(), regex("blank.*"))
        );

        assert_ne!(
            Function::FileRegex("subdir".into(), regex("blank.*")),
            Function::FileRegex("other".into(), regex("blank.*"))
        );
        assert_ne!(
            Function::FileRegex("subdir".into(), regex("blank.*")),
            Function::FileRegex("subdir".into(), regex(".*"))
        );
    }

    #[test]
    fn function_eq_for_file_regex_should_be_case_insensitive_on_pathbuf_and_regex() {
        assert_eq!(
            Function::FileRegex("subdir".into(), regex("blank.*")),
            Function::FileRegex("Subdir".into(), regex("Blank.*"))
        );
    }

    #[test]
    fn function_eq_for_active_path_should_check_pathbuf() {
        assert_eq!(
            Function::ActivePath("Blank.esm".into()),
            Function::ActivePath("Blank.esm".into())
        );

        assert_ne!(
            Function::ActivePath("Blank.esp".into()),
            Function::ActivePath("Blank.esm".into())
        );
    }

    #[test]
    fn function_eq_for_active_path_should_be_case_insensitive_on_pathbuf() {
        assert_eq!(
            Function::ActivePath("Blank.esm".into()),
            Function::ActivePath("blank.esm".into())
        );
    }

    #[test]
    fn function_eq_active_path_should_not_be_equal_to_file_path_with_same_pathbuf() {
        assert_ne!(
            Function::ActivePath("Blank.esm".into()),
            Function::FilePath("Blank.esm".into())
        );
    }

    #[test]
    fn function_eq_for_active_regex_should_check_regex() {
        assert_eq!(
            Function::ActiveRegex(regex("blank.*")),
            Function::ActiveRegex(regex("blank.*"))
        );

        assert_ne!(
            Function::ActiveRegex(regex("blank.*")),
            Function::ActiveRegex(regex(".*"))
        );
    }

    #[test]
    fn function_eq_for_active_regex_should_be_case_insensitive_on_regex() {
        assert_eq!(
            Function::ActiveRegex(regex("blank.*")),
            Function::ActiveRegex(regex("Blank.*"))
        );
    }

    #[test]
    fn function_eq_for_many_should_check_pathbuf_and_regex() {
        assert_eq!(
            Function::Many("subdir".into(), regex("blank.*")),
            Function::Many("subdir".into(), regex("blank.*"))
        );

        assert_ne!(
            Function::Many("subdir".into(), regex("blank.*")),
            Function::Many("subdir".into(), regex(".*"))
        );
        assert_ne!(
            Function::Many("subdir".into(), regex("blank.*")),
            Function::Many("other".into(), regex("blank.*"))
        );
    }

    #[test]
    fn function_eq_for_many_should_be_case_insensitive_on_pathbuf_and_regex() {
        assert_eq!(
            Function::FileRegex("subdir".into(), regex("blank.*")),
            Function::FileRegex("Subdir".into(), regex("Blank.*"))
        );
    }

    #[test]
    fn function_eq_many_should_not_be_equal_to_file_regex_with_same_pathbuf_and_regex() {
        assert_ne!(
            Function::Many("subdir".into(), regex("blank.*")),
            Function::FileRegex("subdir".into(), regex("blank.*"))
        );
    }

    #[test]
    fn function_eq_for_many_active_should_check_regex() {
        assert_eq!(
            Function::ManyActive(regex("blank.*")),
            Function::ManyActive(regex("blank.*"))
        );

        assert_ne!(
            Function::ManyActive(regex("blank.*")),
            Function::ManyActive(regex(".*"))
        );
    }

    #[test]
    fn function_eq_for_many_active_should_be_case_insensitive_on_regex() {
        assert_eq!(
            Function::ManyActive(regex("blank.*")),
            Function::ManyActive(regex("Blank.*"))
        );
    }

    #[test]
    fn function_eq_many_active_should_not_be_equal_to_active_regex_with_same_regex() {
        assert_ne!(
            Function::ManyActive(regex("blank.*")),
            Function::ActiveRegex(regex("blank.*"))
        );
    }

    #[test]
    fn function_eq_for_checksum_should_check_pathbuf_and_crc() {
        assert_eq!(
            Function::Checksum("Blank.esm".into(), 1),
            Function::Checksum("Blank.esm".into(), 1)
        );

        assert_ne!(
            Function::Checksum("Blank.esm".into(), 1),
            Function::Checksum("Blank.esm".into(), 2)
        );
        assert_ne!(
            Function::Checksum("Blank.esm".into(), 1),
            Function::Checksum("Blank.esp".into(), 1)
        );
    }

    #[test]
    fn function_eq_for_checksum_should_be_case_insensitive_on_pathbuf() {
        assert_eq!(
            Function::Checksum("Blank.esm".into(), 1),
            Function::Checksum("blank.esm".into(), 1)
        );
    }

    #[test]
    fn function_eq_for_version_should_check_pathbuf_version_and_comparator() {
        assert_eq!(
            Function::Version("Blank.esm".into(), "1".into(), ComparisonOperator::Equal),
            Function::Version("Blank.esm".into(), "1".into(), ComparisonOperator::Equal)
        );

        assert_ne!(
            Function::Version("Blank.esm".into(), "1".into(), ComparisonOperator::Equal),
            Function::Version("Blank.esp".into(), "1".into(), ComparisonOperator::Equal)
        );
        assert_ne!(
            Function::Version("Blank.esm".into(), "1".into(), ComparisonOperator::Equal),
            Function::Version("Blank.esm".into(), "2".into(), ComparisonOperator::Equal)
        );
        assert_ne!(
            Function::Version("Blank.esm".into(), "1".into(), ComparisonOperator::Equal),
            Function::Version("Blank.esm".into(), "1".into(), ComparisonOperator::NotEqual)
        );
    }

    #[test]
    fn function_eq_for_version_should_be_case_insensitive_on_pathbuf_and_version() {
        assert_eq!(
            Function::Version("Blank.esm".into(), "A".into(), ComparisonOperator::Equal),
            Function::Version("blank.esm".into(), "a".into(), ComparisonOperator::Equal)
        );
    }
}
