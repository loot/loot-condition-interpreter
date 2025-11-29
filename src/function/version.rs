mod pe;

use std::cmp::Ordering;
use std::path::Path;

use crate::error::Error;
use pe::{read_file_version, read_pe_version, read_product_version};

#[derive(Clone, Debug)]
enum ReleaseId {
    Numeric(u32),
    NonNumeric(String),
}

impl<'a> From<&'a str> for ReleaseId {
    fn from(string: &'a str) -> Self {
        string.trim().parse().map_or_else(
            |_| ReleaseId::NonNumeric(string.to_lowercase()),
            ReleaseId::Numeric,
        )
    }
}

fn are_numeric_values_equal(n: u32, s: &str) -> bool {
    // The values can only be equal if the trimmed string can be wholly
    // converted to the same u32 value.
    match s.trim().parse() {
        Ok(n2) => n == n2,
        Err(_) => false,
    }
}

impl PartialEq for ReleaseId {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Numeric(n1), Self::Numeric(n2)) => n1 == n2,
            (Self::NonNumeric(s1), Self::NonNumeric(s2)) => s1 == s2,
            (Self::Numeric(n), Self::NonNumeric(s)) | (Self::NonNumeric(s), Self::Numeric(n)) => {
                are_numeric_values_equal(*n, s)
            }
        }
    }
}

// This is like u32::from_str_radix(), but stops instead of erroring when it
// encounters a non-digit character. It also doesn't support signs.
fn u32_from_str(id: &str) -> (Option<u32>, usize) {
    // Conversion can fail even with only ASCII digits because of overflow, so
    // take that into account.
    if let Some((digits, remainder)) = id.split_once(|c: char| !c.is_ascii_digit()) {
        if digits.is_empty() {
            (None, id.len())
        } else {
            (digits.trim().parse().ok(), remainder.len() + 1)
        }
    } else {
        (id.trim().parse().ok(), 0)
    }
}

fn compare_heterogeneous_ids(lhs_number: u32, rhs_string: &str) -> Option<Ordering> {
    match u32_from_str(rhs_string) {
        (Some(rhs_number), remaining_slice_length) => {
            match lhs_number.partial_cmp(&rhs_number) {
                // If not all bytes were digits, treat the non-numeric ID as
                // greater.
                Some(Ordering::Equal) if remaining_slice_length > 0 => Some(Ordering::Less),
                order => order,
            }
        }
        // If there are no digits to compare, numeric values are
        // always less than non-numeric values.
        (None, _) => Some(Ordering::Less),
    }
}

impl PartialOrd for ReleaseId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Self::Numeric(n1), Self::Numeric(n2)) => n1.partial_cmp(n2),
            (Self::NonNumeric(s1), Self::NonNumeric(s2)) => s1.partial_cmp(s2),
            (Self::Numeric(n), Self::NonNumeric(s)) => compare_heterogeneous_ids(*n, s),
            (Self::NonNumeric(s), Self::Numeric(n)) => {
                compare_heterogeneous_ids(*n, s).map(Ordering::reverse)
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, PartialOrd)]
enum PreReleaseId {
    Numeric(u32),
    NonNumeric(String),
}

impl<'a> From<&'a str> for PreReleaseId {
    fn from(string: &'a str) -> Self {
        string.trim().parse().map_or_else(
            |_| PreReleaseId::NonNumeric(string.to_lowercase()),
            PreReleaseId::Numeric,
        )
    }
}

#[derive(Debug)]
pub(super) struct Version {
    release_ids: Vec<ReleaseId>,
    pre_release_ids: Vec<PreReleaseId>,
}

impl Version {
    pub(super) fn read_file_version(file_path: &Path) -> Result<Option<Self>, Error> {
        read_pe_version(file_path, read_file_version)
    }

    pub(super) fn read_product_version(file_path: &Path) -> Result<Option<Self>, Error> {
        read_pe_version(file_path, read_product_version)
    }

    pub(super) fn is_readable(file_path: &Path) -> bool {
        read_pe_version(file_path, |_| Ok(None)).is_ok()
    }
}

fn is_separator(c: char) -> bool {
    c == '-' || c == ' ' || c == ':' || c == '_'
}

fn is_pre_release_separator(c: char) -> bool {
    c == '.' || is_separator(c)
}

fn split_version_string(string: &str) -> (&str, &str) {
    // Special case for strings of the form "0, 1, 2, 3", which are used in
    // OBSE and SKSE, and which should be interpreted as "0.1.2.3".
    if let Ok(regex) = regex::Regex::new("\\d+, \\d+, \\d+, \\d+") {
        if regex.is_match(string) {
            return (string, "");
        }
    }

    string.split_once(is_separator).unwrap_or((string, ""))
}

impl<T: AsRef<str>> From<T> for Version {
    fn from(string: T) -> Self {
        let (release, pre_release) = split_version_string(trim_metadata(string.as_ref()));

        Version {
            release_ids: release.split(['.', ',']).map(ReleaseId::from).collect(),
            pre_release_ids: pre_release
                .split_terminator(is_pre_release_separator)
                .map(PreReleaseId::from)
                .collect(),
        }
    }
}

fn trim_metadata(version: &str) -> &str {
    if version.is_empty() {
        "0"
    } else if let Some((prefix, _)) = version.split_once('+') {
        prefix
    } else {
        version
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Version) -> Option<Ordering> {
        let (self_release_ids, other_release_ids) =
            pad_release_ids(&self.release_ids, &other.release_ids);

        match self_release_ids.partial_cmp(&other_release_ids) {
            Some(Ordering::Equal) | None => {
                match (
                    self.pre_release_ids.is_empty(),
                    other.pre_release_ids.is_empty(),
                ) {
                    (true, false) => Some(Ordering::Greater),
                    (false, true) => Some(Ordering::Less),
                    _ => self.pre_release_ids.partial_cmp(&other.pre_release_ids),
                }
            }
            r => r,
        }
    }
}

impl PartialEq for Version {
    fn eq(&self, other: &Version) -> bool {
        let (self_release_ids, other_release_ids) =
            pad_release_ids(&self.release_ids, &other.release_ids);

        self_release_ids == other_release_ids && self.pre_release_ids == other.pre_release_ids
    }
}

fn pad_release_ids(ids1: &[ReleaseId], ids2: &[ReleaseId]) -> (Vec<ReleaseId>, Vec<ReleaseId>) {
    let mut ids1 = ids1.to_vec();
    let mut ids2 = ids2.to_vec();

    match ids1.len().cmp(&ids2.len()) {
        Ordering::Less => ids1.resize(ids2.len(), ReleaseId::Numeric(0)),
        Ordering::Greater => ids2.resize(ids1.len(), ReleaseId::Numeric(0)),
        Ordering::Equal => {}
    }

    (ids1, ids2)
}

#[cfg(test)]
mod tests {
    fn is_cmp_eq(lhs: &super::Version, rhs: &super::Version) -> bool {
        lhs.partial_cmp(rhs).unwrap().is_eq()
    }

    mod release_ids {
        use super::super::*;

        #[test]
        fn eq_should_compare_equality_of_u32_values() {
            assert_eq!(ReleaseId::Numeric(1), ReleaseId::Numeric(1));
            assert_ne!(ReleaseId::Numeric(1), ReleaseId::Numeric(0));
        }

        #[test]
        fn eq_should_compare_equality_of_string_values() {
            assert_eq!(
                ReleaseId::NonNumeric("abcd".into()),
                ReleaseId::NonNumeric("abcd".into())
            );
            assert_ne!(
                ReleaseId::NonNumeric("abcd".into()),
                ReleaseId::NonNumeric("abce".into())
            );
        }

        #[test]
        fn eq_should_convert_string_values_to_u32_before_comparing_against_a_u32_value() {
            assert_eq!(ReleaseId::Numeric(123), ReleaseId::NonNumeric("123".into()));
            assert_eq!(
                ReleaseId::Numeric(123),
                ReleaseId::NonNumeric(" 123 ".into())
            );

            assert_ne!(
                ReleaseId::Numeric(123),
                ReleaseId::NonNumeric("1two3".into())
            );

            assert_eq!(ReleaseId::NonNumeric("123".into()), ReleaseId::Numeric(123));
            assert_eq!(
                ReleaseId::NonNumeric(" 123 ".into()),
                ReleaseId::Numeric(123)
            );

            assert_ne!(
                ReleaseId::NonNumeric("1two3".into()),
                ReleaseId::Numeric(123)
            );
        }

        #[test]
        fn cmp_should_compare_u32_values() {
            let cmp = ReleaseId::Numeric(1).partial_cmp(&ReleaseId::Numeric(1));
            assert_eq!(Some(Ordering::Equal), cmp);

            let cmp = ReleaseId::Numeric(1).partial_cmp(&ReleaseId::Numeric(2));
            assert_eq!(Some(Ordering::Less), cmp);

            let cmp = ReleaseId::Numeric(2).partial_cmp(&ReleaseId::Numeric(1));
            assert_eq!(Some(Ordering::Greater), cmp);
        }

        #[test]
        fn cmp_should_compare_string_values() {
            let cmp = ReleaseId::NonNumeric("alpha".into())
                .partial_cmp(&ReleaseId::NonNumeric("alpha".into()));
            assert_eq!(Some(Ordering::Equal), cmp);

            let cmp = ReleaseId::NonNumeric("alpha".into())
                .partial_cmp(&ReleaseId::NonNumeric("beta".into()));
            assert_eq!(Some(Ordering::Less), cmp);

            let cmp = ReleaseId::NonNumeric("beta".into())
                .partial_cmp(&ReleaseId::NonNumeric("alpha".into()));
            assert_eq!(Some(Ordering::Greater), cmp);
        }

        #[test]
        fn cmp_should_treat_strings_with_no_leading_digits_as_always_greater_than_u32s() {
            let cmp = ReleaseId::Numeric(123).partial_cmp(&ReleaseId::NonNumeric("one23".into()));
            assert_eq!(Some(Ordering::Less), cmp);

            let cmp = ReleaseId::NonNumeric("one23".into()).partial_cmp(&ReleaseId::Numeric(123));
            assert_eq!(Some(Ordering::Greater), cmp);
        }

        #[test]
        fn cmp_should_compare_leading_digits_in_strings_against_u32s_and_use_the_result_if_it_is_not_equal(
        ) {
            let cmp = ReleaseId::Numeric(86).partial_cmp(&ReleaseId::NonNumeric("78b".into()));
            assert_eq!(Some(Ordering::Greater), cmp);

            let cmp = ReleaseId::NonNumeric("78b".into()).partial_cmp(&ReleaseId::Numeric(86));
            assert_eq!(Some(Ordering::Less), cmp);
        }

        #[test]
        fn cmp_should_compare_leading_digits_in_strings_against_u32s_and_use_the_result_if_it_is_equal_and_there_are_no_non_digit_characters(
        ) {
            let cmp = ReleaseId::Numeric(86).partial_cmp(&ReleaseId::NonNumeric("86".into()));
            assert_eq!(Some(Ordering::Equal), cmp);

            let cmp = ReleaseId::NonNumeric("86".into()).partial_cmp(&ReleaseId::Numeric(86));
            assert_eq!(Some(Ordering::Equal), cmp);
        }

        #[test]
        fn cmp_should_compare_leading_digits_in_strings_against_u32s_and_treat_the_u32_as_less_if_the_result_is_equal_and_there_are_non_digit_characters(
        ) {
            let cmp = ReleaseId::Numeric(86).partial_cmp(&ReleaseId::NonNumeric("86b".into()));
            assert_eq!(Some(Ordering::Less), cmp);

            let cmp = ReleaseId::NonNumeric("86b".into()).partial_cmp(&ReleaseId::Numeric(86));
            assert_eq!(Some(Ordering::Greater), cmp);
        }
    }

    mod constructors {
        use super::super::*;

        #[test]
        fn version_read_file_version_should_read_the_file_version_field_of_a_32_bit_executable() {
            let version = Version::read_file_version(Path::new("tests/libloot_win32/loot.dll"))
                .unwrap()
                .unwrap();

            assert_eq!(
                version.release_ids,
                vec![
                    ReleaseId::Numeric(0),
                    ReleaseId::Numeric(18),
                    ReleaseId::Numeric(2),
                    ReleaseId::Numeric(0),
                ]
            );
            assert!(version.pre_release_ids.is_empty());
        }

        #[test]
        fn version_read_file_version_should_read_the_file_version_field_of_a_64_bit_executable() {
            let version = Version::read_file_version(Path::new("tests/libloot_win64/loot.dll"))
                .unwrap()
                .unwrap();

            assert_eq!(
                version.release_ids,
                vec![
                    ReleaseId::Numeric(0),
                    ReleaseId::Numeric(18),
                    ReleaseId::Numeric(2),
                    ReleaseId::Numeric(0),
                ]
            );
            assert!(version.pre_release_ids.is_empty());
        }

        #[test]
        fn version_read_file_version_should_error_with_path_if_path_does_not_exist() {
            let error = Version::read_file_version(Path::new("missing")).unwrap_err();

            assert!(error
                .to_string()
                .starts_with("An error was encountered while accessing the path \"missing\":"));
        }

        #[test]
        fn version_read_file_version_should_error_with_path_if_the_file_is_not_an_executable() {
            let error = Version::read_file_version(Path::new("Cargo.toml")).unwrap_err();

            assert_eq!("An error was encountered while reading the version fields of \"Cargo.toml\": Unknown file magic", error.to_string());
        }

        #[test]
        fn version_read_file_version_should_return_none_if_there_is_no_version_info() {
            let version =
                Version::read_file_version(Path::new("tests/loot_api_python/loot_api.pyd"))
                    .unwrap();

            assert!(version.is_none());
        }

        #[test]
        fn version_read_product_version_should_read_the_file_version_field_of_a_32_bit_executable()
        {
            let version = Version::read_product_version(Path::new("tests/libloot_win32/loot.dll"))
                .unwrap()
                .unwrap();

            assert_eq!(
                version.release_ids,
                vec![
                    ReleaseId::Numeric(0),
                    ReleaseId::Numeric(18),
                    ReleaseId::Numeric(2)
                ]
            );
            assert!(version.pre_release_ids.is_empty());
        }

        #[test]
        fn version_read_product_version_should_read_the_file_version_field_of_a_64_bit_executable()
        {
            let version = Version::read_product_version(Path::new("tests/libloot_win64/loot.dll"))
                .unwrap()
                .unwrap();

            assert_eq!(
                version.release_ids,
                vec![
                    ReleaseId::Numeric(0),
                    ReleaseId::Numeric(18),
                    ReleaseId::Numeric(2),
                ]
            );
            assert!(version.pre_release_ids.is_empty());
        }

        #[test]
        fn version_read_product_version_should_find_non_us_english_version_strings() {
            let tmp_dir = tempfile::tempdir().unwrap();
            let dll_path = tmp_dir.path().join("loot.ru.dll");

            let mut dll_bytes = std::fs::read("tests/libloot_win32/loot.dll").unwrap();

            // Set the version info block's language code to 1049 (Russian).
            dll_bytes[0x0053_204A] = b'1'; // This changes VersionInfo.strings.Language.lang_id
            dll_bytes[0x0053_216C] = 0x19; // This changes VersionInfo.langs.Language.lang_id

            std::fs::write(&dll_path, dll_bytes).unwrap();

            let version = Version::read_product_version(&dll_path).unwrap().unwrap();

            assert_eq!(
                version.release_ids,
                vec![
                    ReleaseId::Numeric(0),
                    ReleaseId::Numeric(18),
                    ReleaseId::Numeric(2)
                ]
            );
            assert!(version.pre_release_ids.is_empty());
        }

        #[test]
        fn version_read_product_version_should_error_with_path_if_path_does_not_exist() {
            let error = Version::read_product_version(Path::new("missing")).unwrap_err();

            assert!(error
                .to_string()
                .starts_with("An error was encountered while accessing the path \"missing\":"));
        }

        #[test]
        fn version_read_product_version_should_error_with_path_if_the_file_is_not_an_executable() {
            let error = Version::read_product_version(Path::new("Cargo.toml")).unwrap_err();

            assert_eq!("An error was encountered while reading the version fields of \"Cargo.toml\": Unknown file magic", error.to_string());
        }

        #[test]
        fn version_read_product_version_should_return_none_if_there_is_no_version_info() {
            let version =
                Version::read_product_version(Path::new("tests/loot_api_python/loot_api.pyd"))
                    .unwrap();

            assert!(version.is_none());
        }
    }

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
        use super::is_cmp_eq;

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
        fn version_eq_should_consider_versions_that_differ_by_the_presence_of_a_pre_release_id_to_be_not_equal(
        ) {
            assert_ne!(Version::from("1.0.0"), Version::from("1.0.0-alpha"));
        }

        #[test]
        fn version_partial_cmp_should_treat_the_absence_of_a_pre_release_id_as_greater_than_its_presence(
        ) {
            assert!(Version::from("1.0.0-alpha") < Version::from("1.0.0"));
            assert!(Version::from("1.0.0") > Version::from("1.0.0-alpha"));
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

            assert!(Version::from("0.0.5-86") < Version::from("0.0.5-78b"));
            assert!(Version::from("0.0.5-78b") > Version::from("0.0.5-86"));
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
        fn version_partial_cmp_release_ids_should_take_precedence_over_pre_release_ids() {
            assert!(Version::from("0.0.5-10") < Version::from("0.0.10-5"));
            assert!(Version::from("0.0.10-5") > Version::from("0.0.5-10"));
        }

        #[test]
        fn version_eq_should_ignore_metadata() {
            assert_eq!(Version::from("0.0.1+alpha"), Version::from("0.0.1+beta"));
        }

        #[test]
        fn version_partial_cmp_should_ignore_metadata() {
            assert!(is_cmp_eq(
                &Version::from("0.0.1+alpha"),
                &Version::from("0.0.1+1")
            ));
            assert!(is_cmp_eq(
                &Version::from("0.0.1+1"),
                &Version::from("0.0.1+alpha")
            ));

            assert!(is_cmp_eq(
                &Version::from("0.0.1+2"),
                &Version::from("0.0.1+1")
            ));
            assert!(is_cmp_eq(
                &Version::from("0.0.1+1"),
                &Version::from("0.0.1+2")
            ));
        }
    }

    mod extensions {
        use super::super::*;
        use super::is_cmp_eq;

        #[test]
        fn version_from_should_parse_comma_separated_versions() {
            // OBSE and SKSE use version string fields of the form "0, 2, 0, 12".
            let version = Version::from("0, 2, 0, 12");

            assert_eq!(
                version.release_ids,
                vec![
                    ReleaseId::Numeric(0),
                    ReleaseId::Numeric(2),
                    ReleaseId::Numeric(0),
                    ReleaseId::Numeric(12),
                ]
            );
            assert!(version.pre_release_ids.is_empty());
        }

        #[test]
        fn version_eq_should_ignore_leading_zeroes_in_major_version_numbers() {
            assert_eq!(Version::from("05.0.0"), Version::from("5.0.0"));
            assert_eq!(Version::from("5.0.0"), Version::from("05.0.0"));
        }

        #[test]
        fn version_partial_cmp_should_ignore_leading_zeroes_in_major_version_numbers() {
            assert!(is_cmp_eq(&Version::from("05.0.0"), &Version::from("5.0.0")));
            assert!(is_cmp_eq(&Version::from("5.0.0"), &Version::from("05.0.0")));
        }

        #[test]
        fn version_eq_should_ignore_leading_zeroes_in_minor_version_numbers() {
            assert_eq!(Version::from("0.05.0"), Version::from("0.5.0"));
            assert_eq!(Version::from("0.5.0"), Version::from("0.05.0"));
        }

        #[test]
        fn version_partial_cmp_should_ignore_leading_zeroes_in_minor_version_numbers() {
            assert!(is_cmp_eq(&Version::from("0.05.0"), &Version::from("0.5.0")));
            assert!(is_cmp_eq(&Version::from("0.5.0"), &Version::from("0.05.0")));
        }

        #[test]
        fn version_eq_should_ignore_leading_zeroes_in_patch_version_numbers() {
            assert_eq!(Version::from("0.0.05"), Version::from("0.0.5"));
            assert_eq!(Version::from("0.0.5"), Version::from("0.0.05"));
        }

        #[test]
        fn version_partial_cmp_should_ignore_leading_zeroes_in_patch_version_numbers() {
            assert!(is_cmp_eq(&Version::from("0.0.05"), &Version::from("0.0.5")));
            assert!(is_cmp_eq(&Version::from("0.0.5"), &Version::from("0.0.05")));
        }

        #[test]
        fn version_eq_should_ignore_leading_zeroes_in_numeric_pre_release_ids() {
            assert_eq!(Version::from("0.0.5-05"), Version::from("0.0.5-5"));
            assert_eq!(Version::from("0.0.5-5"), Version::from("0.0.5-05"));
        }

        #[test]
        fn version_partial_cmp_should_ignore_leading_zeroes_in_numeric_pre_release_ids() {
            assert!(is_cmp_eq(
                &Version::from("0.0.5-05"),
                &Version::from("0.0.5-5")
            ));
            assert!(is_cmp_eq(
                &Version::from("0.0.5-5"),
                &Version::from("0.0.5-05")
            ));
        }

        #[test]
        fn version_eq_should_compare_an_equal_but_arbitrary_number_of_version_numbers() {
            assert_eq!(Version::from("1.0.0.1.0.0"), Version::from("1.0.0.1.0.0"));

            assert_ne!(Version::from("1.0.0.0.0.0"), Version::from("1.0.0.0.0.1"));
            assert_ne!(Version::from("1.0.0.0.0.1"), Version::from("1.0.0.0.0.0"));
        }

        #[test]
        fn version_partial_cmp_should_compare_an_equal_but_arbitrary_number_of_version_numbers() {
            assert!(is_cmp_eq(
                &Version::from("1.0.0.1.0.0"),
                &Version::from("1.0.0.1.0.0")
            ));

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
        fn version_partial_cmp_numeric_and_non_numeric_release_ids_should_be_compared_by_leading_numeric_values_first(
        ) {
            assert!(Version::from("0.78b") < Version::from("0.86"));
            assert!(Version::from("0.86") > Version::from("0.78b"));
        }

        #[test]
        fn version_partial_cmp_non_numeric_release_ids_should_be_greater_than_release_ids() {
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
        fn version_eq_should_pad_release_id_vecs_to_equal_length_with_zeroes() {
            assert_eq!(Version::from("1-beta"), Version::from("1.0.0-beta"));
            assert_eq!(Version::from("1.0.0-beta"), Version::from("1-beta"));

            assert_eq!(Version::from("0.0.0.1"), Version::from("0.0.0.1.0.0"));
            assert_eq!(Version::from("0.0.0.1.0.0"), Version::from("0.0.0.1"));

            assert_ne!(Version::from("1.0.0.0"), Version::from("1.0.0.0.0.1"));
            assert_ne!(Version::from("1.0.0.0.0.1"), Version::from("1.0.0.0"));
        }

        #[test]
        fn version_partial_cmp_should_pad_release_id_vecs_to_equal_length_with_zeroes() {
            assert!(Version::from("1.0.0.0.0.0") < Version::from("1.0.0.1"));
            assert!(Version::from("1.0.0.1") > Version::from("1.0.0.0.0.0"));

            assert!(Version::from("1.0.0.0") < Version::from("1.0.0.0.0.1"));
            assert!(Version::from("1.0.0.0.0.1") > Version::from("1.0.0.0"));

            assert!(is_cmp_eq(
                &Version::from("1.0.0.0.0.0"),
                &Version::from("1.0.0.0")
            ));
            assert!(is_cmp_eq(
                &Version::from("1.0.0.0"),
                &Version::from("1.0.0.0.0.0")
            ));
        }

        #[test]
        fn version_from_should_treat_space_as_separator_between_release_and_pre_release_ids() {
            let version = Version::from("1.0.0 alpha");
            assert_eq!(
                version.release_ids,
                vec![
                    ReleaseId::Numeric(1),
                    ReleaseId::Numeric(0),
                    ReleaseId::Numeric(0)
                ]
            );
            assert_eq!(
                version.pre_release_ids,
                vec![PreReleaseId::NonNumeric("alpha".into())]
            );
        }

        #[test]
        fn version_from_should_treat_colon_as_separator_between_release_and_pre_release_ids() {
            let version = Version::from("1.0.0:alpha");
            assert_eq!(
                version.release_ids,
                vec![
                    ReleaseId::Numeric(1),
                    ReleaseId::Numeric(0),
                    ReleaseId::Numeric(0)
                ]
            );
            assert_eq!(
                version.pre_release_ids,
                vec![PreReleaseId::NonNumeric("alpha".into())]
            );
        }

        #[test]
        fn version_from_should_treat_underscore_as_separator_between_release_and_pre_release_ids() {
            let version = Version::from("1.0.0_alpha");
            assert_eq!(
                version.release_ids,
                vec![
                    ReleaseId::Numeric(1),
                    ReleaseId::Numeric(0),
                    ReleaseId::Numeric(0)
                ]
            );
            assert_eq!(
                version.pre_release_ids,
                vec![PreReleaseId::NonNumeric("alpha".into())]
            );
        }

        #[test]
        fn version_from_should_treat_space_as_separator_between_pre_release_ids() {
            let version = Version::from("1.0.0-alpha 1");
            assert_eq!(
                version.release_ids,
                vec![
                    ReleaseId::Numeric(1),
                    ReleaseId::Numeric(0),
                    ReleaseId::Numeric(0)
                ]
            );
            assert_eq!(
                version.pre_release_ids,
                vec![
                    PreReleaseId::NonNumeric("alpha".into()),
                    PreReleaseId::Numeric(1)
                ]
            );
        }

        #[test]
        fn version_from_should_treat_colon_as_separator_between_pre_release_ids() {
            let version = Version::from("1.0.0-alpha:1");
            assert_eq!(
                version.release_ids,
                vec![
                    ReleaseId::Numeric(1),
                    ReleaseId::Numeric(0),
                    ReleaseId::Numeric(0)
                ]
            );
            assert_eq!(
                version.pre_release_ids,
                vec![
                    PreReleaseId::NonNumeric("alpha".into()),
                    PreReleaseId::Numeric(1)
                ]
            );
        }

        #[test]
        fn version_from_should_treat_underscore_as_separator_between_pre_release_ids() {
            let version = Version::from("1.0.0-alpha_1");
            assert_eq!(
                version.release_ids,
                vec![
                    ReleaseId::Numeric(1),
                    ReleaseId::Numeric(0),
                    ReleaseId::Numeric(0)
                ]
            );
            assert_eq!(
                version.pre_release_ids,
                vec![
                    PreReleaseId::NonNumeric("alpha".into()),
                    PreReleaseId::Numeric(1)
                ]
            );
        }

        #[test]
        fn version_from_should_treat_dash_as_separator_between_pre_release_ids() {
            let version = Version::from("1.0.0-alpha-1");
            assert_eq!(
                version.release_ids,
                vec![
                    ReleaseId::Numeric(1),
                    ReleaseId::Numeric(0),
                    ReleaseId::Numeric(0)
                ]
            );
            assert_eq!(
                version.pre_release_ids,
                vec![
                    PreReleaseId::NonNumeric("alpha".into()),
                    PreReleaseId::Numeric(1)
                ]
            );
        }
    }
}
