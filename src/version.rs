use std::cmp::Ordering;
use std::path::Path;

use pelite::resources::version_info::VersionInfo;
use pelite::resources::{FindError, Resources};
use pelite::FileMap;

use crate::error::Error;

const VERSION_INFO_RESOURCE_PATH: &str = "/16/1";

#[derive(Clone, Debug, PartialEq, PartialOrd)]
enum Identifier {
    Numeric(u32),
    NonNumeric(String),
}

impl<'a> From<&'a str> for Identifier {
    fn from(string: &'a str) -> Self {
        u32::from_str_radix(string.trim(), 10)
            .map(Identifier::Numeric)
            .unwrap_or_else(|_| Identifier::NonNumeric(string.to_lowercase()))
    }
}

#[derive(Debug)]
pub struct Version {
    release_ids: Vec<Identifier>,
    pre_release_ids: Vec<Identifier>,
}

impl Version {
    pub fn read_file_version(file_path: &Path) -> Result<Self, Error> {
        Self::read_version(file_path, |v| {
            v.fixed()
                .map(|f| {
                    format!(
                        "{}.{}.{}.{}",
                        f.dwFileVersion.Major,
                        f.dwFileVersion.Minor,
                        f.dwFileVersion.Patch,
                        f.dwFileVersion.Build
                    )
                })
                .unwrap_or_else(String::new)
        })
    }

    pub fn read_product_version(file_path: &Path) -> Result<Self, Error> {
        Self::read_version(file_path, |v| {
            v.query_value(&"ProductVersion")
                .map(String::from_utf16_lossy)
                .unwrap_or_else(String::new)
        })
    }

    fn read_version<F: Fn(&VersionInfo) -> String>(
        file_path: &Path,
        formatter: F,
    ) -> Result<Self, Error> {
        let file_map =
            FileMap::open(file_path).map_err(|e| Error::IoError(file_path.to_path_buf(), e))?;
        let version_info = get_pe_version_info(file_map.as_ref())
            .map_err(|e| Error::PeParsingError(file_path.to_path_buf(), e.into()))?;

        Ok(Version::from(formatter(&version_info).as_str()))
    }
}

fn get_pe_version_info(bytes: &[u8]) -> Result<VersionInfo, FindError> {
    let resources = get_pe_resources(bytes)?;

    // Can't just call resources.version_info() because that only gets the
    // version info for US English, which may not exist. Instead, get the first
    // version info block, whatever the language.
    let bytes = resources
        .find_dir(Path::new(VERSION_INFO_RESOURCE_PATH))?
        .entries()
        .next()
        .ok_or(FindError::NotFound)?
        .entry()?
        .data()
        .ok_or(FindError::UnDirectory)?
        .bytes()?;

    VersionInfo::try_from(bytes).map_err(FindError::Pe)
}

fn get_pe_resources(bytes: &[u8]) -> Result<Resources, pelite::Error> {
    use pelite::pe64;
    match pe64::PeFile::from_bytes(bytes) {
        Ok(file) => {
            use pelite::pe64::Pe;

            file.resources()
        }
        Err(pelite::Error::PeMagic) => {
            use pelite::pe32::{Pe, PeFile};

            PeFile::from_bytes(bytes)?.resources()
        }
        Err(e) => Err(e),
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

    match string.find(is_separator) {
        Some(i) if i + 1 < string.len() => (&string[..i], &string[i + 1..]),
        Some(_) | None => (string, ""),
    }
}

impl<'a> From<&'a str> for Version {
    fn from(string: &'a str) -> Self {
        let (release, pre_release) = split_version_string(trim_metadata(string));

        Version {
            release_ids: release
                .split(|c| c == '.' || c == ',')
                .map(Identifier::from)
                .collect(),
            pre_release_ids: pre_release
                .split_terminator(is_pre_release_separator)
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
        let (self_release_ids, other_release_ids) =
            pad_release_ids(&self.release_ids, &other.release_ids);

        match self_release_ids.partial_cmp(&other_release_ids) {
            Some(Ordering::Equal) | None => {
                self.pre_release_ids.partial_cmp(&other.pre_release_ids)
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

fn pad_release_ids(ids1: &[Identifier], ids2: &[Identifier]) -> (Vec<Identifier>, Vec<Identifier>) {
    let mut ids1 = ids1.to_vec();
    let mut ids2 = ids2.to_vec();

    if ids1.len() < ids2.len() {
        ids1.resize(ids2.len(), Identifier::Numeric(0));
    } else if ids2.len() < ids1.len() {
        ids2.resize(ids1.len(), Identifier::Numeric(0));
    }

    (ids1, ids2)
}

#[cfg(test)]
mod tests {
    mod constructors {
        use super::super::*;

        #[test]
        fn version_read_file_version_should_read_the_file_version_field_of_a_32_bit_executable() {
            let version =
                Version::read_file_version(Path::new("tests/loot_api_win32/loot_api.dll")).unwrap();

            assert_eq!(
                version.release_ids,
                vec![
                    Identifier::Numeric(0),
                    Identifier::Numeric(13),
                    Identifier::Numeric(8),
                    Identifier::Numeric(0),
                ]
            );
            assert!(version.pre_release_ids.is_empty());
        }

        #[test]
        fn version_read_file_version_should_read_the_file_version_field_of_a_64_bit_executable() {
            let version =
                Version::read_file_version(Path::new("tests/loot_api_win64/loot_api.dll")).unwrap();

            assert_eq!(
                version.release_ids,
                vec![
                    Identifier::Numeric(0),
                    Identifier::Numeric(13),
                    Identifier::Numeric(8),
                    Identifier::Numeric(0),
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

            assert_eq!("An error was encountered while reading the version fields of \"Cargo.toml\": bad magic", error.to_string());
        }

        #[test]
        fn version_read_product_version_should_read_the_file_version_field_of_a_32_bit_executable()
        {
            let version = Version::read_product_version(Path::new("tests/7z/7za.exe")).unwrap();

            assert_eq!(
                version.release_ids,
                vec![Identifier::Numeric(18), Identifier::Numeric(5),]
            );
            assert!(version.pre_release_ids.is_empty());
        }

        #[test]
        fn version_read_product_version_should_read_the_file_version_field_of_a_64_bit_executable()
        {
            let version = Version::read_product_version(Path::new("tests/7z/x64/7za.exe")).unwrap();

            assert_eq!(
                version.release_ids,
                vec![Identifier::Numeric(18), Identifier::Numeric(5),]
            );
            assert!(version.pre_release_ids.is_empty());
        }

        #[test]
        fn version_read_file_version_should_find_non_us_english_version_strings() {
            let tmp_dir = tempfile::tempdir().unwrap();
            let dll_path = tmp_dir.path().join("7zxa.ru.dll");

            // Set the version info block's language code to 1049 (Russian).
            let mut dll_bytes = std::fs::read("tests/7z/7zxa.dll").unwrap();
            dll_bytes[0x23B10] = 0x19;
            std::fs::write(&dll_path, dll_bytes).unwrap();

            let version = Version::read_product_version(&dll_path).unwrap();

            assert_eq!(
                version.release_ids,
                vec![Identifier::Numeric(18), Identifier::Numeric(5)]
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

            assert_eq!("An error was encountered while reading the version fields of \"Cargo.toml\": bad magic", error.to_string());
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
            assert!(!(Version::from("0.0.1+alpha") < Version::from("0.0.1+1")));
            assert!(!(Version::from("0.0.1+1") < Version::from("0.0.1+alpha")));

            assert!(!(Version::from("0.0.1+2") < Version::from("0.0.1+1")));
            assert!(!(Version::from("0.0.1+1") < Version::from("0.0.1+2")));
        }
    }

    mod extensions {
        use super::super::*;

        #[test]
        fn version_from_should_parse_comma_separated_versions() {
            // OBSE and SKSE use version string fields of the form "0, 2, 0, 12".
            let version = Version::from("0, 2, 0, 12");

            assert_eq!(
                version.release_ids,
                vec![
                    Identifier::Numeric(0),
                    Identifier::Numeric(2),
                    Identifier::Numeric(0),
                    Identifier::Numeric(12),
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

            assert!(!(Version::from("1.0.0.0.0.0") < Version::from("1.0.0.0")));
            assert!(!(Version::from("1.0.0.0") < Version::from("1.0.0.0.0.0")));
        }

        #[test]
        fn version_from_should_treat_space_as_separator_between_release_and_pre_release_ids() {
            let version = Version::from("1.0.0 alpha");
            assert_eq!(
                version.release_ids,
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
                version.release_ids,
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
                version.release_ids,
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
                version.release_ids,
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
                version.release_ids,
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
                version.release_ids,
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
                version.release_ids,
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
