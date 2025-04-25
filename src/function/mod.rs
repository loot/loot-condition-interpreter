use std::fmt;
use std::hash::{Hash, Hasher};
use std::mem::discriminant;
use std::path::PathBuf;

use regex::Regex;
use unicase::eq;

pub mod eval;
pub mod parse;
mod path;
mod version;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ComparisonOperator {
    Equal,
    NotEqual,
    LessThan,
    GreaterThan,
    LessThanOrEqual,
    GreaterThanOrEqual,
}

impl fmt::Display for ComparisonOperator {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::ComparisonOperator::*;
        match self {
            Equal => write!(f, "=="),
            NotEqual => write!(f, "!="),
            LessThan => write!(f, "<"),
            GreaterThan => write!(f, ">"),
            LessThanOrEqual => write!(f, "<="),
            GreaterThanOrEqual => write!(f, ">="),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Function {
    FilePath(PathBuf),
    FileRegex(PathBuf, Regex),
    FileSize(PathBuf, u64),
    Readable(PathBuf),
    IsExecutable(PathBuf),
    ActivePath(PathBuf),
    ActiveRegex(Regex),
    IsMaster(PathBuf),
    Many(PathBuf, Regex),
    ManyActive(Regex),
    Checksum(PathBuf, u32),
    Version(PathBuf, String, ComparisonOperator),
    ProductVersion(PathBuf, String, ComparisonOperator),
    FilenameVersion(PathBuf, Regex, String, ComparisonOperator),
    DescriptionContains(PathBuf, Regex),
}

impl fmt::Display for Function {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Function::*;
        match self {
            FilePath(p) => write!(f, "file(\"{}\")", p.display()),
            FileRegex(p, r) => write!(f, "file(\"{}/{}\")", p.display(), r),
            FileSize(p, s) => write!(f, "file_size(\"{}\", {})", p.display(), s),
            Readable(p) => write!(f, "readable(\"{}\")", p.display()),
            IsExecutable(p) => write!(f, "is_executable(\"{}\")", p.display()),
            ActivePath(p) => write!(f, "active(\"{}\")", p.display()),
            ActiveRegex(r) => write!(f, "active(\"{}\")", r),
            IsMaster(p) => write!(f, "is_master(\"{}\")", p.display()),
            Many(p, r) => write!(f, "many(\"{}/{}\")", p.display(), r),
            ManyActive(r) => write!(f, "many_active(\"{}\")", r),
            Checksum(p, c) => write!(f, "checksum(\"{}\", {:02X?})", p.display(), c),
            Version(p, v, c) => write!(f, "version(\"{}\", \"{}\", {})", p.display(), v, c),
            ProductVersion(p, v, c) => {
                write!(f, "product_version(\"{}\", \"{}\", {})", p.display(), v, c)
            }
            FilenameVersion(p, r, v, c) => {
                write!(
                    f,
                    "filename_version(\"{}/{}\", \"{}\", {})",
                    p.display(),
                    r,
                    v,
                    c
                )
            }
            DescriptionContains(p, r) => {
                write!(f, "description_contains(\"{}\", \"{}\")", p.display(), r)
            }
        }
    }
}

impl PartialEq for Function {
    fn eq(&self, other: &Function) -> bool {
        use Function::*;
        match (self, other) {
            (FilePath(p1), FilePath(p2)) => eq(&p1.to_string_lossy(), &p2.to_string_lossy()),
            (FileRegex(p1, r1), FileRegex(p2, r2)) => {
                eq(r1.as_str(), r2.as_str()) && eq(&p1.to_string_lossy(), &p2.to_string_lossy())
            }
            (FileSize(p1, s1), FileSize(p2, s2)) => {
                s1 == s2 && eq(&p1.to_string_lossy(), &p2.to_string_lossy())
            }
            (Readable(p1), Readable(p2)) => eq(&p1.to_string_lossy(), &p2.to_string_lossy()),
            (IsExecutable(p1), IsExecutable(p2)) => {
                eq(&p1.to_string_lossy(), &p2.to_string_lossy())
            }
            (ActivePath(p1), ActivePath(p2)) => eq(&p1.to_string_lossy(), &p2.to_string_lossy()),
            (ActiveRegex(r1), ActiveRegex(r2)) => eq(r1.as_str(), r2.as_str()),
            (IsMaster(p1), IsMaster(p2)) => eq(&p1.to_string_lossy(), &p2.to_string_lossy()),
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
            (ProductVersion(p1, v1, c1), ProductVersion(p2, v2, c2)) => {
                c1 == c2 && eq(&v1, &v2) && eq(&p1.to_string_lossy(), &p2.to_string_lossy())
            }
            (FilenameVersion(p1, r1, v1, c1), FilenameVersion(p2, r2, v2, c2)) => {
                c1 == c2
                    && eq(&v1, &v2)
                    && eq(r1.as_str(), r2.as_str())
                    && eq(&p1.to_string_lossy(), &p2.to_string_lossy())
            }
            (DescriptionContains(p1, r1), DescriptionContains(p2, r2)) => {
                eq(r1.as_str(), r2.as_str()) && eq(&p1.to_string_lossy(), &p2.to_string_lossy())
            }
            _ => false,
        }
    }
}

impl Eq for Function {}

impl Hash for Function {
    fn hash<H: Hasher>(&self, state: &mut H) {
        use Function::*;
        match self {
            FilePath(p) => {
                p.to_string_lossy().to_lowercase().hash(state);
            }
            FileRegex(p, r) => {
                p.to_string_lossy().to_lowercase().hash(state);
                r.as_str().to_lowercase().hash(state);
            }
            FileSize(p, s) => {
                p.to_string_lossy().to_lowercase().hash(state);
                s.hash(state);
            }
            Readable(p) => {
                p.to_string_lossy().to_lowercase().hash(state);
            }
            IsExecutable(p) => {
                p.to_string_lossy().to_lowercase().hash(state);
            }
            ActivePath(p) => {
                p.to_string_lossy().to_lowercase().hash(state);
            }
            ActiveRegex(r) => {
                r.as_str().to_lowercase().hash(state);
            }
            IsMaster(p) => {
                p.to_string_lossy().to_lowercase().hash(state);
            }
            Many(p, r) => {
                p.to_string_lossy().to_lowercase().hash(state);
                r.as_str().to_lowercase().hash(state);
            }
            ManyActive(r) => {
                r.as_str().to_lowercase().hash(state);
            }
            Checksum(p, c) => {
                p.to_string_lossy().to_lowercase().hash(state);
                c.hash(state);
            }
            Version(p, v, c) => {
                p.to_string_lossy().to_lowercase().hash(state);
                v.to_lowercase().hash(state);
                c.hash(state);
            }
            ProductVersion(p, v, c) => {
                p.to_string_lossy().to_lowercase().hash(state);
                v.to_lowercase().hash(state);
                c.hash(state);
            }
            FilenameVersion(p, r, v, c) => {
                p.to_string_lossy().to_lowercase().hash(state);
                r.as_str().to_lowercase().hash(state);
                v.to_lowercase().hash(state);
                c.hash(state);
            }
            DescriptionContains(p, r) => {
                p.to_string_lossy().to_lowercase().hash(state);
                r.as_str().to_lowercase().hash(state);
            }
        }

        discriminant(self).hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn regex(string: &str) -> Regex {
        Regex::new(string).unwrap()
    }

    mod fmt {
        use super::*;

        #[test]
        fn function_fmt_for_file_path_should_format_correctly() {
            let function = Function::FilePath("subdir/Blank.esm".into());

            assert_eq!("file(\"subdir/Blank.esm\")", &format!("{}", function));
        }

        #[test]
        fn function_fmt_for_file_regex_should_format_correctly() {
            let function = Function::FileRegex("subdir".into(), regex("Blank.*"));

            assert_eq!("file(\"subdir/Blank.*\")", &format!("{}", function));
        }

        #[test]
        fn function_fmt_for_file_size_should_format_correctly() {
            let function = Function::FileSize("subdir/Blank.esm".into(), 12345678);

            assert_eq!(
                "file_size(\"subdir/Blank.esm\", 12345678)",
                &format!("{}", function)
            );
        }

        #[test]
        fn function_fmt_for_readable_should_format_correctly() {
            let function = Function::Readable("subdir/Blank.esm".into());

            assert_eq!("readable(\"subdir/Blank.esm\")", &format!("{}", function));
        }

        #[test]
        fn function_fmt_for_is_executable_should_format_correctly() {
            let function = Function::IsExecutable("subdir/Blank.esm".into());

            assert_eq!(
                "is_executable(\"subdir/Blank.esm\")",
                &format!("{}", function)
            );
        }

        #[test]
        fn function_fmt_for_active_path_should_format_correctly() {
            let function = Function::ActivePath("Blank.esm".into());

            assert_eq!("active(\"Blank.esm\")", &format!("{}", function));
        }

        #[test]
        fn function_fmt_for_active_regex_should_format_correctly() {
            let function = Function::ActiveRegex(regex("Blank.*"));

            assert_eq!("active(\"Blank.*\")", &format!("{}", function));
        }

        #[test]
        fn function_fmt_for_is_master_should_format_correctly() {
            let function = Function::IsMaster("Blank.esm".into());

            assert_eq!("is_master(\"Blank.esm\")", &format!("{}", function));
        }

        #[test]
        fn function_fmt_for_many_should_format_correctly() {
            let function = Function::Many("subdir".into(), regex("Blank.*"));

            assert_eq!("many(\"subdir/Blank.*\")", &format!("{}", function));
        }

        #[test]
        fn function_fmt_for_many_active_should_format_correctly() {
            let function = Function::ManyActive(regex("Blank.*"));

            assert_eq!("many_active(\"Blank.*\")", &format!("{}", function));
        }

        #[test]
        fn function_fmt_for_checksum_should_format_correctly() {
            let function = Function::Checksum("subdir/Blank.esm".into(), 0xDEADBEEF);

            assert_eq!(
                "checksum(\"subdir/Blank.esm\", DEADBEEF)",
                &format!("{}", function)
            );
        }

        #[test]
        fn function_fmt_for_version_should_format_correctly() {
            let function = Function::Version(
                "subdir/Blank.esm".into(),
                "1.2a".into(),
                ComparisonOperator::Equal,
            );

            assert_eq!(
                "version(\"subdir/Blank.esm\", \"1.2a\", ==)",
                &format!("{}", function)
            );
        }

        #[test]
        fn function_fmt_for_product_version_should_format_correctly() {
            let function = Function::ProductVersion(
                "../TESV.exe".into(),
                "1.2a".into(),
                ComparisonOperator::Equal,
            );

            assert_eq!(
                "product_version(\"../TESV.exe\", \"1.2a\", ==)",
                &format!("{}", function)
            );
        }

        #[test]
        fn function_fmt_for_filename_version_should_format_correctly() {
            let function = Function::FilenameVersion(
                "subdir".into(),
                regex(r"filename (\d+(?:[_.-]?\d+)*[a-z]?)\.esp"),
                "1.2a".into(),
                ComparisonOperator::Equal,
            );

            assert_eq!(
                "filename_version(\"subdir/filename (\\d+(?:[_.-]?\\d+)*[a-z]?)\\.esp\", \"1.2a\", ==)",
                &format!("{}", function)
            );
        }

        #[test]
        fn function_fmt_for_description_contains_should_format_correctly() {
            let function = Function::DescriptionContains("Blank.esp".into(), regex("€ƒ."));

            assert_eq!(
                "description_contains(\"Blank.esp\", \"€ƒ.\")",
                &format!("{}", function)
            );
        }
    }

    mod eq {
        use super::*;

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
        fn function_eq_for_file_size_should_check_pathbuf_and_size() {
            assert_eq!(
                Function::FileSize("subdir".into(), 1),
                Function::FileSize("subdir".into(), 1)
            );

            assert_ne!(
                Function::FileSize("subdir".into(), 1),
                Function::FileSize("other".into(), 1)
            );
            assert_ne!(
                Function::FileSize("subdir".into(), 1),
                Function::FileSize("subdir".into(), 2)
            );
        }

        #[test]
        fn function_eq_for_file_size_should_be_case_insensitive_on_pathbuf() {
            assert_eq!(
                Function::FileSize("subdir".into(), 1),
                Function::FileSize("Subdir".into(), 1)
            );
        }

        #[test]
        fn function_eq_for_readable_should_check_pathbuf() {
            assert_eq!(
                Function::Readable("Blank.esm".into()),
                Function::Readable("Blank.esm".into())
            );

            assert_ne!(
                Function::Readable("Blank.esp".into()),
                Function::Readable("Blank.esm".into())
            );
        }

        #[test]
        fn function_eq_for_readable_should_be_case_insensitive_on_pathbuf() {
            assert_eq!(
                Function::Readable("Blank.esm".into()),
                Function::Readable("blank.esm".into())
            );
        }

        #[test]
        fn function_eq_for_readable_should_not_be_equal_to_file_path_with_same_pathbuf() {
            assert_ne!(
                Function::Readable("Blank.esm".into()),
                Function::FilePath("Blank.esm".into())
            );
        }

        #[test]
        fn function_eq_for_is_executable_should_check_pathbuf() {
            assert_eq!(
                Function::IsExecutable("Blank.esm".into()),
                Function::IsExecutable("Blank.esm".into())
            );

            assert_ne!(
                Function::IsExecutable("Blank.esp".into()),
                Function::IsExecutable("Blank.esm".into())
            );
        }

        #[test]
        fn function_eq_for_is_executable_should_be_case_insensitive_on_pathbuf() {
            assert_eq!(
                Function::IsExecutable("Blank.esm".into()),
                Function::IsExecutable("blank.esm".into())
            );
        }

        #[test]
        fn function_eq_for_is_executable_should_not_be_equal_to_file_path_or_is_executable_with_same_pathbuf(
        ) {
            assert_ne!(
                Function::IsExecutable("Blank.esm".into()),
                Function::FilePath("Blank.esm".into())
            );
            assert_ne!(
                Function::IsExecutable("Blank.esm".into()),
                Function::Readable("Blank.esm".into())
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
        fn function_eq_for_active_path_should_not_be_equal_to_file_path_with_same_pathbuf() {
            assert_ne!(
                Function::ActivePath("Blank.esm".into()),
                Function::FilePath("Blank.esm".into())
            );
        }

        #[test]
        fn function_eq_for_active_path_should_not_be_equal_to_readable_with_same_pathbuf() {
            assert_ne!(
                Function::ActivePath("Blank.esm".into()),
                Function::Readable("Blank.esm".into())
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
        fn function_eq_for_is_master_should_check_pathbuf() {
            assert_eq!(
                Function::IsMaster("Blank.esm".into()),
                Function::IsMaster("Blank.esm".into())
            );

            assert_ne!(
                Function::IsMaster("Blank.esp".into()),
                Function::IsMaster("Blank.esm".into())
            );
        }

        #[test]
        fn function_eq_for_is_master_should_be_case_insensitive_on_pathbuf() {
            assert_eq!(
                Function::IsMaster("Blank.esm".into()),
                Function::IsMaster("blank.esm".into())
            );
        }

        #[test]
        fn function_eq_for_is_master_should_not_be_equal_to_file_path_with_same_pathbuf() {
            assert_ne!(
                Function::IsMaster("Blank.esm".into()),
                Function::FilePath("Blank.esm".into())
            );
        }

        #[test]
        fn function_eq_for_is_master_should_not_be_equal_to_readable_with_same_pathbuf() {
            assert_ne!(
                Function::IsMaster("Blank.esm".into()),
                Function::Readable("Blank.esm".into())
            );
        }

        #[test]
        fn function_eq_for_is_master_should_not_be_equal_to_active_path_with_same_pathbuf() {
            assert_ne!(
                Function::IsMaster("Blank.esm".into()),
                Function::ActivePath("Blank.esm".into())
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
                Function::Many("subdir".into(), regex("blank.*")),
                Function::Many("Subdir".into(), regex("Blank.*"))
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

        #[test]
        fn function_eq_for_product_version_should_check_pathbuf_version_and_comparator() {
            assert_eq!(
                Function::ProductVersion("Blank.esm".into(), "1".into(), ComparisonOperator::Equal),
                Function::ProductVersion("Blank.esm".into(), "1".into(), ComparisonOperator::Equal)
            );

            assert_ne!(
                Function::ProductVersion("Blank.esm".into(), "1".into(), ComparisonOperator::Equal),
                Function::ProductVersion("Blank.esp".into(), "1".into(), ComparisonOperator::Equal)
            );
            assert_ne!(
                Function::ProductVersion("Blank.esm".into(), "1".into(), ComparisonOperator::Equal),
                Function::ProductVersion("Blank.esm".into(), "2".into(), ComparisonOperator::Equal)
            );
            assert_ne!(
                Function::ProductVersion("Blank.esm".into(), "1".into(), ComparisonOperator::Equal),
                Function::ProductVersion(
                    "Blank.esm".into(),
                    "1".into(),
                    ComparisonOperator::NotEqual
                )
            );
        }

        #[test]
        fn function_eq_for_product_version_should_be_case_insensitive_on_pathbuf_and_version() {
            assert_eq!(
                Function::ProductVersion("Blank.esm".into(), "A".into(), ComparisonOperator::Equal),
                Function::ProductVersion("blank.esm".into(), "a".into(), ComparisonOperator::Equal)
            );
        }

        #[test]
        fn function_eq_for_filename_version_should_check_pathbuf_regex_version_and_comparator() {
            assert_eq!(
                Function::FilenameVersion(
                    "subdir".into(),
                    regex("Blank\\.esm"),
                    "1".into(),
                    ComparisonOperator::Equal
                ),
                Function::FilenameVersion(
                    "subdir".into(),
                    regex("Blank\\.esm"),
                    "1".into(),
                    ComparisonOperator::Equal
                )
            );

            assert_ne!(
                Function::FilenameVersion(
                    "subdir1".into(),
                    regex("Blank\\.esm"),
                    "1".into(),
                    ComparisonOperator::Equal
                ),
                Function::FilenameVersion(
                    "subdir2".into(),
                    regex("Blank\\.esm"),
                    "1".into(),
                    ComparisonOperator::Equal
                )
            );
            assert_ne!(
                Function::FilenameVersion(
                    "subdir".into(),
                    regex("Blank\\.esm"),
                    "1".into(),
                    ComparisonOperator::Equal
                ),
                Function::FilenameVersion(
                    "subdir".into(),
                    regex("Blank.esp"),
                    "1".into(),
                    ComparisonOperator::Equal
                )
            );
            assert_ne!(
                Function::FilenameVersion(
                    "subdir".into(),
                    regex("Blank\\.esm"),
                    "1".into(),
                    ComparisonOperator::Equal
                ),
                Function::FilenameVersion(
                    "subdir".into(),
                    regex("Blank\\.esm"),
                    "2".into(),
                    ComparisonOperator::Equal
                )
            );
            assert_ne!(
                Function::FilenameVersion(
                    "subdir".into(),
                    regex("Blank\\.esm"),
                    "1".into(),
                    ComparisonOperator::Equal
                ),
                Function::FilenameVersion(
                    "subdir".into(),
                    regex("Blank\\.esm"),
                    "1".into(),
                    ComparisonOperator::NotEqual
                )
            );
        }

        #[test]
        fn function_eq_for_filename_version_should_be_case_insensitive_on_pathbuf_and_version() {
            assert_eq!(
                Function::FilenameVersion(
                    "subdir".into(),
                    regex("Blank\\.esm"),
                    "A".into(),
                    ComparisonOperator::Equal
                ),
                Function::FilenameVersion(
                    "Subdir".into(),
                    regex("Blank\\.esm"),
                    "a".into(),
                    ComparisonOperator::Equal
                )
            );
        }

        #[test]
        fn function_eq_for_description_contains_should_check_pathbuf_and_regex() {
            assert_eq!(
                Function::DescriptionContains("Blank.esp".into(), regex("€ƒ.")),
                Function::DescriptionContains("Blank.esp".into(), regex("€ƒ."))
            );

            assert_ne!(
                Function::DescriptionContains("Blank.esp".into(), regex("€ƒ.")),
                Function::DescriptionContains("Blank.esp".into(), regex(".*"))
            );
            assert_ne!(
                Function::DescriptionContains("Blank.esp".into(), regex("€ƒ.")),
                Function::DescriptionContains("other".into(), regex("€ƒ."))
            );
        }

        #[test]
        fn function_eq_for_description_contains_should_be_case_insensitive_on_pathbuf_and_regex() {
            assert_eq!(
                Function::DescriptionContains("Blank.esp".into(), regex("€ƒ.")),
                Function::DescriptionContains("blank.esp".into(), regex("€Ƒ."))
            );
        }

        #[test]
        fn function_eq_description_contains_should_not_be_equal_to_file_regex_with_same_pathbuf_and_regex(
        ) {
            assert_ne!(
                Function::DescriptionContains("Blank.esp".into(), regex("€ƒ.")),
                Function::FileRegex("Blank.esp".into(), regex("€ƒ."))
            );
        }
    }

    mod hash {
        use super::*;

        use std::collections::hash_map::DefaultHasher;

        fn hash(function: Function) -> u64 {
            let mut hasher = DefaultHasher::new();
            function.hash(&mut hasher);
            hasher.finish()
        }

        #[test]
        fn function_hash_file_path_should_hash_pathbuf() {
            let function1 = Function::FilePath("Blank.esm".into());
            let function2 = Function::FilePath("Blank.esm".into());

            assert_eq!(hash(function1), hash(function2));

            let function1 = Function::FilePath("Blank.esm".into());
            let function2 = Function::FilePath("Blank.esp".into());

            assert_ne!(hash(function1), hash(function2));
        }

        #[test]
        fn function_hash_file_path_should_be_case_insensitive() {
            let function1 = Function::FilePath("Blank.esm".into());
            let function2 = Function::FilePath("blank.esm".into());

            assert_eq!(hash(function1), hash(function2));
        }

        #[test]
        fn function_hash_file_regex_should_hash_pathbuf_and_regex() {
            let function1 = Function::FileRegex("subdir".into(), regex(".*"));
            let function2 = Function::FileRegex("subdir".into(), regex(".*"));

            assert_eq!(hash(function1), hash(function2));

            let function1 = Function::FileRegex("subdir".into(), regex(".*"));
            let function2 = Function::FileRegex("other".into(), regex(".*"));

            assert_ne!(hash(function1), hash(function2));

            let function1 = Function::FileRegex("subdir".into(), regex(".*"));
            let function2 = Function::FileRegex("subdir".into(), regex("Blank.*"));

            assert_ne!(hash(function1), hash(function2));
        }

        #[test]
        fn function_hash_file_regex_should_be_case_insensitive() {
            let function1 = Function::FileRegex("Subdir".into(), regex("Blank.*"));
            let function2 = Function::FileRegex("subdir".into(), regex("blank.*"));

            assert_eq!(hash(function1), hash(function2));
        }

        #[test]
        fn function_hash_file_size_should_hash_pathbuf_and_size() {
            let function1 = Function::FileSize("subdir".into(), 1);
            let function2 = Function::FileSize("subdir".into(), 1);

            assert_eq!(hash(function1), hash(function2));

            let function1 = Function::FileSize("subdir".into(), 1);
            let function2 = Function::FileSize("other".into(), 1);

            assert_ne!(hash(function1), hash(function2));

            let function1 = Function::FileSize("subdir".into(), 1);
            let function2 = Function::FileSize("subdir".into(), 2);

            assert_ne!(hash(function1), hash(function2));
        }

        #[test]
        fn function_hash_file_size_should_be_case_insensitive() {
            let function1 = Function::FileSize("Subdir".into(), 1);
            let function2 = Function::FileSize("subdir".into(), 1);

            assert_eq!(hash(function1), hash(function2));
        }

        #[test]
        fn function_hash_readable_should_hash_pathbuf() {
            let function1 = Function::Readable("Blank.esm".into());
            let function2 = Function::Readable("Blank.esm".into());

            assert_eq!(hash(function1), hash(function2));

            let function1 = Function::Readable("Blank.esm".into());
            let function2 = Function::Readable("Blank.esp".into());

            assert_ne!(hash(function1), hash(function2));
        }

        #[test]
        fn function_hash_readable_should_be_case_insensitive() {
            let function1 = Function::Readable("Blank.esm".into());
            let function2 = Function::Readable("blank.esm".into());

            assert_eq!(hash(function1), hash(function2));
        }

        #[test]
        fn function_hash_file_path_and_readable_should_not_have_equal_hashes() {
            let function1 = Function::FilePath("Blank.esm".into());
            let function2 = Function::Readable("Blank.esm".into());

            assert_ne!(hash(function1), hash(function2));
        }

        #[test]
        fn function_hash_is_executable_should_hash_pathbuf() {
            let function1 = Function::IsExecutable("Blank.esm".into());
            let function2 = Function::IsExecutable("Blank.esm".into());

            assert_eq!(hash(function1), hash(function2));

            let function1 = Function::IsExecutable("Blank.esm".into());
            let function2 = Function::IsExecutable("Blank.esp".into());

            assert_ne!(hash(function1), hash(function2));
        }

        #[test]
        fn function_hash_is_executable_should_be_case_insensitive() {
            let function1 = Function::IsExecutable("Blank.esm".into());
            let function2 = Function::IsExecutable("blank.esm".into());

            assert_eq!(hash(function1), hash(function2));
        }

        #[test]
        fn function_hash_file_path_and_readable_and_is_executable_should_not_have_equal_hashes() {
            let function1 = Function::FilePath("Blank.esm".into());
            let function2 = Function::Readable("Blank.esm".into());
            let function3 = Function::IsExecutable("Blank.esm".into());

            assert_ne!(hash(function1.clone()), hash(function2.clone()));
            assert_ne!(hash(function3.clone()), hash(function1));
            assert_ne!(hash(function3), hash(function2));
        }

        #[test]
        fn function_hash_active_path_should_hash_pathbuf() {
            let function1 = Function::ActivePath("Blank.esm".into());
            let function2 = Function::ActivePath("Blank.esm".into());

            assert_eq!(hash(function1), hash(function2));

            let function1 = Function::ActivePath("Blank.esm".into());
            let function2 = Function::ActivePath("Blank.esp".into());

            assert_ne!(hash(function1), hash(function2));
        }

        #[test]
        fn function_hash_active_path_should_be_case_insensitive() {
            let function1 = Function::ActivePath("Blank.esm".into());
            let function2 = Function::ActivePath("blank.esm".into());

            assert_eq!(hash(function1), hash(function2));
        }

        #[test]
        fn function_hash_file_path_and_active_path_should_not_have_equal_hashes() {
            let function1 = Function::FilePath("Blank.esm".into());
            let function2 = Function::ActivePath("Blank.esm".into());

            assert_ne!(hash(function1), hash(function2));
        }

        #[test]
        fn function_hash_readable_and_active_path_should_not_have_equal_hashes() {
            let function1 = Function::Readable("Blank.esm".into());
            let function2 = Function::ActivePath("Blank.esm".into());

            assert_ne!(hash(function1), hash(function2));
        }

        #[test]
        fn function_hash_active_regex_should_hash_pathbuf_and_regex() {
            let function1 = Function::ActiveRegex(regex(".*"));
            let function2 = Function::ActiveRegex(regex(".*"));

            assert_eq!(hash(function1), hash(function2));

            let function1 = Function::ActiveRegex(regex(".*"));
            let function2 = Function::ActiveRegex(regex("Blank.*"));

            assert_ne!(hash(function1), hash(function2));
        }

        #[test]
        fn function_hash_active_regex_should_be_case_insensitive() {
            let function1 = Function::ActiveRegex(regex("Blank.*"));
            let function2 = Function::ActiveRegex(regex("blank.*"));

            assert_eq!(hash(function1), hash(function2));
        }

        #[test]
        fn function_hash_is_master_should_hash_pathbuf() {
            let function1 = Function::IsMaster("Blank.esm".into());
            let function2 = Function::IsMaster("Blank.esm".into());

            assert_eq!(hash(function1), hash(function2));

            let function1 = Function::IsMaster("Blank.esm".into());
            let function2 = Function::IsMaster("Blank.esp".into());

            assert_ne!(hash(function1), hash(function2));
        }

        #[test]
        fn function_hash_is_master_should_be_case_insensitive() {
            let function1 = Function::IsMaster("Blank.esm".into());
            let function2 = Function::IsMaster("blank.esm".into());

            assert_eq!(hash(function1), hash(function2));
        }

        #[test]
        fn function_hash_file_path_and_is_master_should_not_have_equal_hashes() {
            let function1 = Function::FilePath("Blank.esm".into());
            let function2 = Function::IsMaster("Blank.esm".into());

            assert_ne!(hash(function1), hash(function2));
        }

        #[test]
        fn function_hash_readable_and_is_master_should_not_have_equal_hashes() {
            let function1 = Function::Readable("Blank.esm".into());
            let function2 = Function::IsMaster("Blank.esm".into());

            assert_ne!(hash(function1), hash(function2));
        }

        #[test]
        fn function_hash_active_path_and_is_master_should_not_have_equal_hashes() {
            let function1 = Function::ActivePath("Blank.esm".into());
            let function2 = Function::IsMaster("Blank.esm".into());

            assert_ne!(hash(function1), hash(function2));
        }

        #[test]
        fn function_hash_many_should_hash_pathbuf_and_regex() {
            let function1 = Function::Many("subdir".into(), regex(".*"));
            let function2 = Function::Many("subdir".into(), regex(".*"));

            assert_eq!(hash(function1), hash(function2));

            let function1 = Function::Many("subdir".into(), regex(".*"));
            let function2 = Function::Many("other".into(), regex(".*"));

            assert_ne!(hash(function1), hash(function2));

            let function1 = Function::Many("subdir".into(), regex(".*"));
            let function2 = Function::Many("subdir".into(), regex("Blank.*"));

            assert_ne!(hash(function1), hash(function2));
        }

        #[test]
        fn function_hash_many_should_be_case_insensitive() {
            let function1 = Function::Many("Subdir".into(), regex("Blank.*"));
            let function2 = Function::Many("subdir".into(), regex("blank.*"));

            assert_eq!(hash(function1), hash(function2));
        }

        #[test]
        fn function_hash_file_regex_and_many_should_not_have_equal_hashes() {
            let function1 = Function::FileRegex("subdir".into(), regex(".*"));
            let function2 = Function::Many("subdir".into(), regex(".*"));

            assert_ne!(hash(function1), hash(function2));
        }

        #[test]
        fn function_many_active_should_hash_pathbuf_and_regex() {
            let function1 = Function::ManyActive(regex(".*"));
            let function2 = Function::ManyActive(regex(".*"));

            assert_eq!(hash(function1), hash(function2));

            let function1 = Function::ManyActive(regex(".*"));
            let function2 = Function::ManyActive(regex("Blank.*"));

            assert_ne!(hash(function1), hash(function2));
        }

        #[test]
        fn function_many_active_should_be_case_insensitive() {
            let function1 = Function::ManyActive(regex("Blank.*"));
            let function2 = Function::ManyActive(regex("blank.*"));

            assert_eq!(hash(function1), hash(function2));
        }

        #[test]
        fn function_hash_active_regex_and_many_active_should_not_have_equal_hashes() {
            let function1 = Function::ActiveRegex(regex(".*"));
            let function2 = Function::ManyActive(regex(".*"));

            assert_ne!(hash(function1), hash(function2));
        }

        #[test]
        fn function_hash_checksum_should_hash_pathbuf_and_regex() {
            let function1 = Function::Checksum("subdir".into(), 1);
            let function2 = Function::Checksum("subdir".into(), 1);

            assert_eq!(hash(function1), hash(function2));

            let function1 = Function::Checksum("subdir".into(), 1);
            let function2 = Function::Checksum("other".into(), 1);

            assert_ne!(hash(function1), hash(function2));

            let function1 = Function::Checksum("subdir".into(), 1);
            let function2 = Function::Checksum("subdir".into(), 2);

            assert_ne!(hash(function1), hash(function2));
        }

        #[test]
        fn function_hash_checksum_should_be_case_insensitive() {
            let function1 = Function::Checksum("Blank.esm".into(), 1);
            let function2 = Function::Checksum("Blank.esm".into(), 1);

            assert_eq!(hash(function1), hash(function2));
        }

        #[test]
        fn function_hash_version_should_hash_pathbuf_and_version_and_comparator() {
            let function1 =
                Function::Version("Blank.esm".into(), "1.2a".into(), ComparisonOperator::Equal);
            let function2 =
                Function::Version("Blank.esm".into(), "1.2a".into(), ComparisonOperator::Equal);

            assert_eq!(hash(function1), hash(function2));

            let function1 =
                Function::Version("Blank.esm".into(), "1".into(), ComparisonOperator::Equal);
            let function2 =
                Function::Version("Blank.esp".into(), "1".into(), ComparisonOperator::Equal);

            assert_ne!(hash(function1), hash(function2));

            let function1 =
                Function::Version("Blank.esm".into(), "1".into(), ComparisonOperator::Equal);
            let function2 =
                Function::Version("Blank.esm".into(), "2".into(), ComparisonOperator::Equal);

            assert_ne!(hash(function1), hash(function2));

            let function1 =
                Function::Version("Blank.esm".into(), "1".into(), ComparisonOperator::Equal);
            let function2 =
                Function::Version("Blank.esm".into(), "1".into(), ComparisonOperator::NotEqual);

            assert_ne!(hash(function1), hash(function2));
        }

        #[test]
        fn function_hash_version_should_be_case_insensitive() {
            let function1 =
                Function::Version("Blank.esm".into(), "1.2a".into(), ComparisonOperator::Equal);
            let function2 =
                Function::Version("Blank.esm".into(), "1.2A".into(), ComparisonOperator::Equal);

            assert_eq!(hash(function1), hash(function2));
        }

        #[test]
        fn function_hash_product_version_should_hash_pathbuf_and_version_and_comparator() {
            let function1 = Function::ProductVersion(
                "Blank.esm".into(),
                "1.2a".into(),
                ComparisonOperator::Equal,
            );
            let function2 = Function::ProductVersion(
                "Blank.esm".into(),
                "1.2a".into(),
                ComparisonOperator::Equal,
            );

            assert_eq!(hash(function1), hash(function2));

            let function1 =
                Function::ProductVersion("Blank.esm".into(), "1".into(), ComparisonOperator::Equal);
            let function2 =
                Function::ProductVersion("Blank.esp".into(), "1".into(), ComparisonOperator::Equal);

            assert_ne!(hash(function1), hash(function2));

            let function1 =
                Function::ProductVersion("Blank.esm".into(), "1".into(), ComparisonOperator::Equal);
            let function2 =
                Function::ProductVersion("Blank.esm".into(), "2".into(), ComparisonOperator::Equal);

            assert_ne!(hash(function1), hash(function2));

            let function1 =
                Function::ProductVersion("Blank.esm".into(), "1".into(), ComparisonOperator::Equal);
            let function2 = Function::ProductVersion(
                "Blank.esm".into(),
                "1".into(),
                ComparisonOperator::NotEqual,
            );

            assert_ne!(hash(function1), hash(function2));
        }

        #[test]
        fn function_hash_product_version_should_be_case_insensitive() {
            let function1 = Function::ProductVersion(
                "Blank.esm".into(),
                "1.2a".into(),
                ComparisonOperator::Equal,
            );
            let function2 = Function::ProductVersion(
                "Blank.esm".into(),
                "1.2A".into(),
                ComparisonOperator::Equal,
            );

            assert_eq!(hash(function1), hash(function2));
        }

        #[test]
        fn function_hash_filename_version_should_hash_pathbuf_regex_and_version_and_comparator() {
            let function1 = Function::FilenameVersion(
                "subdir".into(),
                regex("Blank\\.esm"),
                "1.2a".into(),
                ComparisonOperator::Equal,
            );
            let function2 = Function::FilenameVersion(
                "subdir".into(),
                regex("Blank\\.esm"),
                "1.2a".into(),
                ComparisonOperator::Equal,
            );

            assert_eq!(hash(function1), hash(function2));

            let function1 = Function::FilenameVersion(
                "subdir1".into(),
                regex("Blank\\.esm"),
                "1".into(),
                ComparisonOperator::Equal,
            );
            let function2 = Function::FilenameVersion(
                "subdir2".into(),
                regex("Blank\\.esp"),
                "1".into(),
                ComparisonOperator::Equal,
            );

            assert_ne!(hash(function1), hash(function2));

            let function1 = Function::FilenameVersion(
                "subdir".into(),
                regex("Blank\\.esm"),
                "1".into(),
                ComparisonOperator::Equal,
            );
            let function2 = Function::FilenameVersion(
                "subdir".into(),
                regex("Blank\\.esp"),
                "1".into(),
                ComparisonOperator::Equal,
            );

            assert_ne!(hash(function1), hash(function2));

            let function1 = Function::FilenameVersion(
                "subdir".into(),
                regex("Blank\\.esm"),
                "1".into(),
                ComparisonOperator::Equal,
            );
            let function2 = Function::FilenameVersion(
                "subdir".into(),
                regex("Blank\\.esm"),
                "2".into(),
                ComparisonOperator::Equal,
            );

            assert_ne!(hash(function1), hash(function2));

            let function1 = Function::FilenameVersion(
                "subdir".into(),
                regex("Blank\\.esm"),
                "1".into(),
                ComparisonOperator::Equal,
            );
            let function2 = Function::FilenameVersion(
                "subdir".into(),
                regex("Blank\\.esm"),
                "1".into(),
                ComparisonOperator::NotEqual,
            );

            assert_ne!(hash(function1), hash(function2));
        }

        #[test]
        fn function_hash_filename_version_should_be_case_insensitive() {
            let function1 = Function::FilenameVersion(
                "subdir".into(),
                regex("Blank\\.esm"),
                "1.2a".into(),
                ComparisonOperator::Equal,
            );
            let function2 = Function::FilenameVersion(
                "Subdir".into(),
                regex("Blank\\.esm"),
                "1.2A".into(),
                ComparisonOperator::Equal,
            );

            assert_eq!(hash(function1), hash(function2));
        }

        #[test]
        fn function_hash_description_contains_should_hash_pathbuf_and_regex() {
            let function1 = Function::DescriptionContains("Blank.esp".into(), regex("€ƒ."));
            let function2 = Function::DescriptionContains("Blank.esp".into(), regex("€ƒ."));

            assert_eq!(hash(function1), hash(function2));

            let function1 = Function::DescriptionContains("Blank.esp".into(), regex("€ƒ."));
            let function2 = Function::DescriptionContains("other".into(), regex("€ƒ."));

            assert_ne!(hash(function1), hash(function2));

            let function1 = Function::DescriptionContains("Blank.esp".into(), regex("€ƒ."));
            let function2 = Function::DescriptionContains("Blank.esp".into(), regex(".*"));

            assert_ne!(hash(function1), hash(function2));
        }

        #[test]
        fn function_hash_description_contains_should_be_case_insensitive() {
            let function1 = Function::DescriptionContains("blank.esp".into(), regex("€Ƒ."));
            let function2 = Function::DescriptionContains("Blank.esp".into(), regex("€ƒ."));

            assert_eq!(hash(function1), hash(function2));
        }

        #[test]
        fn function_hash_file_regex_and_description_contains_should_not_have_equal_hashes() {
            let function1 = Function::FileRegex("Blank.esp".into(), regex("€ƒ."));
            let function2 = Function::DescriptionContains("Blank.esp".into(), regex("€ƒ."));

            assert_ne!(hash(function1), hash(function2));
        }
    }
}
