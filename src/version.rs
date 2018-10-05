use std::cmp::Ordering;
use std::path::Path;

use Error;

#[derive(Debug, PartialEq, PartialOrd)]
enum Identifier {
    Numeric(u32),
    NonNumeric(String),
}

impl<'a> From<&'a str> for Identifier {
    fn from(string: &'a str) -> Self {
        u32::from_str_radix(string, 10)
            .map(Identifier::Numeric)
            .unwrap_or_else(|_| Identifier::NonNumeric(string.to_lowercase()))
    }
}

#[derive(Debug)]
pub struct Version {
    release_numbers: Vec<Identifier>,
    pre_release_ids: Vec<Identifier>,
}

impl Version {
    pub fn read_file_version(file_path: &Path) -> Result<Self, Error> {
        // TODO: Actually read the file's File Version field.
        Ok(Version::from(format!("{}", file_path.display()).as_str()))
    }
}

fn is_separator(c: char) -> bool {
    c == '-' || c == ' ' || c == ':' || c == '_'
}

fn is_pre_release_separator(c: char) -> bool {
    c == '.' || is_separator(c)
}

impl<'a> From<&'a str> for Version {
    fn from(string: &'a str) -> Self {
        let trimmed = trim_metadata(string);

        let (release, pre_release) = match trimmed.find(is_separator) {
            Some(i) if i + 1 < trimmed.len() => (&trimmed[..i], &trimmed[i + 1..]),
            Some(_) | None => (trimmed, ""),
        };

        Version {
            release_numbers: release.split('.').map(Identifier::from).collect(),
            pre_release_ids: pre_release
                .split(is_pre_release_separator)
                .map(Identifier::from)
                .collect(),
        }
    }
}

fn trim_metadata(version: &str) -> &str {
    if version.is_empty() {
        "0"
    } else if let Some(i) = version.find('+') {
        &version[..i]
    } else {
        version
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Version) -> Option<Ordering> {
        // TODO: Compare with same behaviour as pseudosem.
        match self.release_numbers.partial_cmp(&other.release_numbers) {
            Some(Ordering::Equal) | None => {
                self.pre_release_ids.partial_cmp(&other.pre_release_ids)
            }
            r => r,
        }
    }
}

impl PartialEq for Version {
    fn eq(&self, other: &Version) -> bool {
        // TODO: Compare with same behaviour as pseudosem.
        self.release_numbers == other.release_numbers
            && self.pre_release_ids == other.pre_release_ids
    }
}

#[cfg(test)]
mod tests {
    mod empty {
        use super::super::*;

        #[test]
        fn version_eq_an_empty_string_should_equal_an_empty_string() {
            assert_eq!(Version::from(""), Version::from(""));
        }

        #[test]
        fn version_eq_an_empty_string_should_equal_a_version_of_zero() {
            assert_eq!(Version::from(""), Version::from("0"));
            assert_eq!(Version::from("0"), Version::from(""));
        }

        #[test]
        fn version_eq_an_empty_string_should_not_equal_a_non_zero_version() {
            assert_ne!(Version::from(""), Version::from("5"));
            assert_ne!(Version::from("5"), Version::from(""));
        }

        #[test]
        fn version_partial_cmp_an_empty_string_should_be_less_than_a_non_zero_version() {
            assert!(Version::from("") < Version::from("1"));
            assert!(Version::from("1") > Version::from(""));
        }
    }

    mod numeric {
        use super::super::*;

        #[test]
        fn version_eq_a_non_empty_string_should_equal_itself() {
            assert_eq!(Version::from("5"), Version::from("5"));
        }

        #[test]
        fn version_eq_single_digit_versions_should_compare_digits() {
            assert_eq!(Version::from("5"), Version::from("5"));

            assert_ne!(Version::from("4"), Version::from("5"));
            assert_ne!(Version::from("5"), Version::from("4"));
        }

        #[test]
        fn version_partial_cmp_single_digit_versions_should_compare_digits() {
            assert!(Version::from("4") < Version::from("5"));
            assert!(Version::from("5") > Version::from("4"));
        }

        #[test]
        fn version_eq_numeric_versions_should_compare_numbers() {
            assert_ne!(Version::from("5"), Version::from("10"));
            assert_ne!(Version::from("10"), Version::from("5"));
        }

        #[test]
        fn version_partial_cmp_numeric_versions_should_compare_numbers() {
            assert!(Version::from("5") < Version::from("10"));
            assert!(Version::from("10") > Version::from("5"));
        }
    }

    mod semver {
        use super::super::*;

        #[test]
        fn version_eq_should_compare_patch_numbers() {
            assert_eq!(Version::from("0.0.5"), Version::from("0.0.5"));

            assert_ne!(Version::from("0.0.5"), Version::from("0.0.10"));
            assert_ne!(Version::from("0.0.10"), Version::from("0.0.5"));
        }

        #[test]
        fn version_partial_cmp_should_compare_patch_numbers() {
            assert!(Version::from("0.0.5") < Version::from("0.0.10"));
            assert!(Version::from("0.0.10") > Version::from("0.0.5"));
        }

        #[test]
        fn version_eq_should_compare_minor_numbers() {
            assert_eq!(Version::from("0.5.0"), Version::from("0.5.0"));

            assert_ne!(Version::from("0.5.0"), Version::from("0.10.0"));
            assert_ne!(Version::from("0.10.0"), Version::from("0.5.0"));
        }

        #[test]
        fn version_partial_cmp_should_compare_minor_numbers() {
            assert!(Version::from("0.5.0") < Version::from("0.10.0"));
            assert!(Version::from("0.10.0") > Version::from("0.5.0"));
        }

        #[test]
        fn version_partial_cmp_minor_numbers_should_take_precedence_over_patch_numbers() {
            assert!(Version::from("0.5.10") < Version::from("0.10.5"));
            assert!(Version::from("0.10.5") > Version::from("0.5.10"));
        }

        #[test]
        fn version_eq_should_compare_major_numbers() {
            assert_eq!(Version::from("5.0.0"), Version::from("5.0.0"));

            assert_ne!(Version::from("5.0.0"), Version::from("10.0.0"));
            assert_ne!(Version::from("10.0.0"), Version::from("5.0.0"));
        }

        #[test]
        fn version_partial_cmp_should_compare_major_numbers() {
            assert!(Version::from("5.0.0") < Version::from("10.0.0"));
            assert!(Version::from("10.0.0") > Version::from("5.0.0"));
        }

        #[test]
        fn version_partial_cmp_major_numbers_should_take_precedence_over_minor_numbers() {
            assert!(Version::from("5.10.0") < Version::from("10.5.0"));
            assert!(Version::from("10.5.0") > Version::from("5.10.0"));
        }

        #[test]
        fn version_partial_cmp_major_numbers_should_take_precedence_over_patch_numbers() {
            assert!(Version::from("5.0.10") < Version::from("10.0.5"));
            assert!(Version::from("10.0.5") > Version::from("5.0.10"));
        }

        #[test]
        fn version_eq_should_compare_pre_release_identifiers() {
            assert_eq!(
                Version::from("0.0.5-5.alpha"),
                Version::from("0.0.5-5.alpha")
            );

            assert_ne!(
                Version::from("0.0.5-5.alpha"),
                Version::from("0.0.5-10.beta")
            );
            assert_ne!(
                Version::from("0.0.5-10.beta"),
                Version::from("0.0.5-5.alpha")
            );
        }

        #[test]
        fn version_partial_cmp_should_compare_numeric_pre_release_ids_numerically() {
            assert!(Version::from("0.0.5-5") < Version::from("0.0.5-10"));
            assert!(Version::from("0.0.5-10") > Version::from("0.0.5-5"));
        }

        #[test]
        fn version_partial_cmp_should_compare_non_numeric_pre_release_ids_lexically() {
            assert!(Version::from("0.0.5-a") < Version::from("0.0.5-b"));
            assert!(Version::from("0.0.5-b") > Version::from("0.0.5-a"));
        }

        #[test]
        fn version_partial_cmp_numeric_pre_release_ids_should_be_less_than_than_non_numeric_ids() {
            assert!(Version::from("0.0.5-9") < Version::from("0.0.5-a"));
            assert!(Version::from("0.0.5-a") > Version::from("0.0.5-9"));
        }

        #[test]
        fn version_partial_cmp_earlier_pre_release_ids_should_take_precedence_over_later_ids() {
            assert!(Version::from("0.0.5-5.10") < Version::from("0.0.5-10.5"));
            assert!(Version::from("0.0.5-10.5") > Version::from("0.0.5-5.10"));
        }

        #[test]
        fn version_partial_cmp_a_version_with_more_pre_release_ids_is_greater() {
            assert!(Version::from("0.0.5-5") < Version::from("0.0.5-5.0"));
            assert!(Version::from("0.0.5-5.0") > Version::from("0.0.5-5"));
        }

        #[test]
        fn version_partial_cmp_release_numbers_should_take_precedence_over_pre_release_ids() {
            assert!(Version::from("0.0.5-10") < Version::from("0.0.10-5"));
            assert!(Version::from("0.0.10-5") > Version::from("0.0.5-10"));
        }

        #[test]
        fn version_eq_should_ignore_metadata() {
            assert_eq!(Version::from("0.0.1+alpha"), Version::from("0.0.1+beta"));
        }

        #[test]
        fn version_partial_cmp_should_ignore_metadata() {
            assert!(!(Version::from("0.0.1+alpha") < Version::from("0.0.1+1")));
            assert!(!(Version::from("0.0.1+1") < Version::from("0.0.1+alpha")));

            assert!(!(Version::from("0.0.1+2") < Version::from("0.0.1+1")));
            assert!(!(Version::from("0.0.1+1") < Version::from("0.0.1+2")));
        }
    }

    mod extensions {
        use super::super::*;

        #[test]
        fn version_eq_should_ignore_leading_zeroes_in_major_version_numbers() {
            assert_eq!(Version::from("05.0.0"), Version::from("5.0.0"));
            assert_eq!(Version::from("5.0.0"), Version::from("05.0.0"));
        }

        #[test]
        fn version_partial_cmp_should_ignore_leading_zeroes_in_major_version_numbers() {
            assert!(!(Version::from("05.0.0") < Version::from("5.0.0")));
            assert!(!(Version::from("5.0.0") < Version::from("05.0.0")));
        }

        #[test]
        fn version_eq_should_ignore_leading_zeroes_in_minor_version_numbers() {
            assert_eq!(Version::from("0.05.0"), Version::from("0.5.0"));
            assert_eq!(Version::from("0.5.0"), Version::from("0.05.0"));
        }

        #[test]
        fn version_partial_cmp_should_ignore_leading_zeroes_in_minor_version_numbers() {
            assert!(!(Version::from("0.05.0") < Version::from("0.5.0")));
            assert!(!(Version::from("0.5.0") < Version::from("0.05.0")));
        }

        #[test]
        fn version_eq_should_ignore_leading_zeroes_in_patch_version_numbers() {
            assert_eq!(Version::from("0.0.05"), Version::from("0.0.5"));
            assert_eq!(Version::from("0.0.5"), Version::from("0.0.05"));
        }

        #[test]
        fn version_partial_cmp_should_ignore_leading_zeroes_in_patch_version_numbers() {
            assert!(!(Version::from("0.0.05") < Version::from("0.0.5")));
            assert!(!(Version::from("0.0.5") < Version::from("0.0.05")));
        }

        #[test]
        fn version_eq_should_ignore_leading_zeroes_in_numeric_pre_release_ids() {
            assert_eq!(Version::from("0.0.5-05"), Version::from("0.0.5-5"));
            assert_eq!(Version::from("0.0.5-5"), Version::from("0.0.5-05"));
        }

        #[test]
        fn version_partial_cmp_should_ignore_leading_zeroes_in_numeric_pre_release_ids() {
            assert!(!(Version::from("0.0.5-05") < Version::from("0.0.5-5")));
            assert!(!(Version::from("0.0.5-5") < Version::from("0.0.5-05")));
        }

        #[test]
        fn version_eq_should_compare_an_equal_but_arbitrary_number_of_version_numbers() {
            assert_eq!(Version::from("1.0.0.1.0.0"), Version::from("1.0.0.1.0.0"));

            assert_ne!(Version::from("1.0.0.0.0.0"), Version::from("1.0.0.0.0.1"));
            assert_ne!(Version::from("1.0.0.0.0.1"), Version::from("1.0.0.0.0.0"));
        }

        #[test]
        fn version_partial_cmp_should_compare_an_equal_but_arbitrary_number_of_version_numbers() {
            assert!(!(Version::from("1.0.0.1.0.0") > Version::from("1.0.0.1.0.0")));

            assert!(Version::from("1.0.0.0.0.0") < Version::from("1.0.0.0.0.1"));
            assert!(Version::from("1.0.0.0.0.1") > Version::from("1.0.0.0.0.0"));
        }

        #[test]
        fn version_eq_non_numeric_release_ids_should_be_compared_lexically() {
            assert_eq!(Version::from("1.0.0a"), Version::from("1.0.0a"));

            assert_ne!(Version::from("1.0.0a"), Version::from("1.0.0b"));
            assert_ne!(Version::from("1.0.0b"), Version::from("1.0.0a"));
        }

        #[test]
        fn version_partial_cmp_non_numeric_release_ids_should_be_compared_lexically() {
            assert!(Version::from("1.0.0a") < Version::from("1.0.0b"));
            assert!(Version::from("1.0.0b") > Version::from("1.0.0a"));
        }

        #[test]
        fn version_partial_cmp_non_numeric_release_ids_should_be_greater_than_release_numbers() {
            assert!(Version::from("1.0.0") < Version::from("1.0.0a"));
            assert!(Version::from("1.0.0a") > Version::from("1.0.0"));
        }

        #[test]
        fn version_partial_cmp_any_release_id_may_be_non_numeric() {
            assert!(Version::from("1.0.0alpha.2") < Version::from("1.0.0beta.2"));
            assert!(Version::from("1.0.0beta.2") > Version::from("1.0.0alpha.2"));
        }

        #[test]
        fn version_eq_should_compare_release_ids_case_insensitively() {
            assert_eq!(Version::from("1.0.0A"), Version::from("1.0.0a"));
            assert_eq!(Version::from("1.0.0a"), Version::from("1.0.0A"));
        }

        #[test]
        fn version_partial_cmp_should_compare_release_ids_case_insensitively() {
            assert!(Version::from("1.0.0a") < Version::from("1.0.0B"));
            assert!(Version::from("1.0.0B") > Version::from("1.0.0a"));
        }

        #[test]
        fn version_eq_should_compare_pre_release_ids_case_insensitively() {
            assert_eq!(Version::from("1.0.0-Alpha"), Version::from("1.0.0-alpha"));
            assert_eq!(Version::from("1.0.0-alpha"), Version::from("1.0.0-Alpha"));
        }

        #[test]
        fn version_partial_cmp_should_compare_pre_release_ids_case_insensitively() {
            assert!(Version::from("1.0.0-alpha") < Version::from("1.0.0-Beta"));
            assert!(Version::from("1.0.0-Beta") > Version::from("1.0.0-alpha"));
        }

        #[test]
        fn version_from_should_treat_space_as_separator_between_release_and_pre_release_ids() {
            let version = Version::from("1.0.0 alpha");
            assert_eq!(
                version.release_numbers,
                vec![
                    Identifier::Numeric(1),
                    Identifier::Numeric(0),
                    Identifier::Numeric(0)
                ]
            );
            assert_eq!(
                version.pre_release_ids,
                vec![Identifier::NonNumeric("alpha".into())]
            );
        }

        #[test]
        fn version_from_should_treat_colon_as_separator_between_release_and_pre_release_ids() {
            let version = Version::from("1.0.0:alpha");
            assert_eq!(
                version.release_numbers,
                vec![
                    Identifier::Numeric(1),
                    Identifier::Numeric(0),
                    Identifier::Numeric(0)
                ]
            );
            assert_eq!(
                version.pre_release_ids,
                vec![Identifier::NonNumeric("alpha".into())]
            );
        }

        #[test]
        fn version_from_should_treat_underscore_as_separator_between_release_and_pre_release_ids() {
            let version = Version::from("1.0.0_alpha");
            assert_eq!(
                version.release_numbers,
                vec![
                    Identifier::Numeric(1),
                    Identifier::Numeric(0),
                    Identifier::Numeric(0)
                ]
            );
            assert_eq!(
                version.pre_release_ids,
                vec![Identifier::NonNumeric("alpha".into())]
            );
        }

        #[test]
        fn version_from_should_treat_space_as_separator_between_pre_release_ids() {
            let version = Version::from("1.0.0-alpha 1");
            assert_eq!(
                version.release_numbers,
                vec![
                    Identifier::Numeric(1),
                    Identifier::Numeric(0),
                    Identifier::Numeric(0)
                ]
            );
            assert_eq!(
                version.pre_release_ids,
                vec![
                    Identifier::NonNumeric("alpha".into()),
                    Identifier::Numeric(1)
                ]
            );
        }

        #[test]
        fn version_from_should_treat_colon_as_separator_between_pre_release_ids() {
            let version = Version::from("1.0.0-alpha:1");
            assert_eq!(
                version.release_numbers,
                vec![
                    Identifier::Numeric(1),
                    Identifier::Numeric(0),
                    Identifier::Numeric(0)
                ]
            );
            assert_eq!(
                version.pre_release_ids,
                vec![
                    Identifier::NonNumeric("alpha".into()),
                    Identifier::Numeric(1)
                ]
            );
        }

        #[test]
        fn version_from_should_treat_underscore_as_separator_between_pre_release_ids() {
            let version = Version::from("1.0.0-alpha_1");
            assert_eq!(
                version.release_numbers,
                vec![
                    Identifier::Numeric(1),
                    Identifier::Numeric(0),
                    Identifier::Numeric(0)
                ]
            );
            assert_eq!(
                version.pre_release_ids,
                vec![
                    Identifier::NonNumeric("alpha".into()),
                    Identifier::Numeric(1)
                ]
            );
        }

        #[test]
        fn version_from_should_treat_dash_as_separator_between_pre_release_ids() {
            let version = Version::from("1.0.0-alpha-1");
            assert_eq!(
                version.release_numbers,
                vec![
                    Identifier::Numeric(1),
                    Identifier::Numeric(0),
                    Identifier::Numeric(0)
                ]
            );
            assert_eq!(
                version.pre_release_ids,
                vec![
                    Identifier::NonNumeric("alpha".into()),
                    Identifier::Numeric(1)
                ]
            );
        }
    }
}
