use std::ffi::OsStr;
use std::fs::{read_dir, File};
use std::hash::Hasher;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

use crc::{crc32, Hasher32};
use regex::Regex;

use super::{ComparisonOperator, Function};
use version::Version;
use Error;
use State;

fn add_extension(path: &Path, extension: &str) -> PathBuf {
    match path.extension() {
        Some(e) => {
            let mut new_extension = e.to_os_string();
            new_extension.push(format!(".{}", extension));
            path.with_extension(&new_extension)
        }
        None => path.with_extension(extension),
    }
}

fn equals(path: &Path, test: &str) -> bool {
    path.to_str().map(|s| s == test).unwrap_or(false)
}

fn resolve_path(state: &State, path: &Path) -> PathBuf {
    if equals(path, "LOOT") {
        state.loot_path.clone()
    } else {
        let path = state.data_path.join(path);

        if !path.exists() && state.game_type.is_plugin_filename(&path) {
            add_extension(&path, "ghost")
        } else {
            path
        }
    }
}

fn evaluate_file_path(state: &State, file_path: &Path) -> Result<bool, Error> {
    Ok(resolve_path(state, file_path).exists())
}

fn is_match(regex: &Regex, file_name: &OsStr) -> bool {
    file_name
        .to_str()
        .map(|s| regex.is_match(s))
        .unwrap_or(false)
}

fn evaluate_file_regex(state: &State, parent_path: &Path, regex: &Regex) -> Result<bool, Error> {
    let dir_iterator = match read_dir(state.data_path.join(parent_path)) {
        Ok(i) => i,
        Err(_) => return Ok(false),
    };

    for entry in dir_iterator {
        if is_match(regex, &entry?.file_name()) {
            return Ok(true);
        }
    }

    Ok(false)
}

fn evaluate_many(state: &State, parent_path: &Path, regex: &Regex) -> Result<bool, Error> {
    let dir_iterator = match read_dir(state.data_path.join(parent_path)) {
        Ok(i) => i,
        Err(_) => return Ok(false),
    };

    let mut found_one = false;
    for entry in dir_iterator {
        if is_match(regex, &entry?.file_name()) {
            if found_one {
                return Ok(true);
            } else {
                found_one = true;
            }
        }
    }

    Ok(false)
}

fn evaluate_active_path(state: &State, path: &Path) -> Result<bool, Error> {
    Ok(path
        .to_str()
        .map(|s| state.active_plugins.contains(&s.to_lowercase()))
        .unwrap_or(false))
}

fn evaluate_active_regex(state: &State, regex: &Regex) -> Result<bool, Error> {
    Ok(state.active_plugins.iter().any(|p| regex.is_match(p)))
}

fn evaluate_many_active(state: &State, regex: &Regex) -> Result<bool, Error> {
    let mut found_one = false;
    for active_plugin in &state.active_plugins {
        if regex.is_match(&active_plugin) {
            if found_one {
                return Ok(true);
            } else {
                found_one = true;
            }
        }
    }

    Ok(false)
}

fn lowercase(path: &Path) -> Option<String> {
    path.to_str().map(str::to_lowercase)
}

fn evaluate_checksum(state: &State, file_path: &Path, crc: u32) -> Result<bool, Error> {
    if let Ok(reader) = state.crc_cache.read() {
        if let Some(key) = lowercase(file_path) {
            if let Some(cached_crc) = reader.get(&key) {
                return Ok(*cached_crc == crc);
            }
        }
    }

    let path = resolve_path(state, file_path);

    if !path.exists() {
        return Ok(false);
    }

    let reader = BufReader::new(File::open(path)?);
    let mut digest = crc32::Digest::new(crc32::IEEE);

    for byte in reader.bytes() {
        digest.write_u8(byte?);
    }

    let calculated_crc = digest.sum32();
    if let Ok(mut writer) = state.crc_cache.write() {
        if let Some(key) = lowercase(file_path) {
            writer.insert(key, calculated_crc);
        }
    }

    Ok(calculated_crc == crc)
}

fn lowercase_filename(path: &Path) -> Option<String> {
    path.file_name()
        .and_then(OsStr::to_str)
        .map(str::to_lowercase)
}

fn get_version(state: &State, file_path: &Path) -> Result<Version, Error> {
    if let Some(key) = lowercase_filename(file_path) {
        if let Some(version) = state.plugin_versions.get(&key) {
            return Ok(Version::from(version.as_str()));
        }
    }

    Version::read_file_version(file_path)
}

fn evaluate_version(
    state: &State,
    file_path: &Path,
    given_version: &str,
    comparator: ComparisonOperator,
) -> Result<bool, Error> {
    let file_path = resolve_path(state, file_path);
    if !file_path.exists() {
        return Ok(comparator == ComparisonOperator::NotEqual
            || comparator == ComparisonOperator::LessThan
            || comparator == ComparisonOperator::LessThanOrEqual);
    }

    let given_version = Version::from(given_version);
    let actual_version = get_version(state, &file_path)?;

    match comparator {
        ComparisonOperator::Equal => Ok(actual_version == given_version),
        ComparisonOperator::NotEqual => Ok(actual_version != given_version),
        ComparisonOperator::LessThan => Ok(actual_version < given_version),
        ComparisonOperator::GreaterThan => Ok(actual_version > given_version),
        ComparisonOperator::LessThanOrEqual => Ok(actual_version <= given_version),
        ComparisonOperator::GreaterThanOrEqual => Ok(actual_version >= given_version),
    }
}

impl Function {
    pub fn eval(&self, state: &State) -> Result<bool, Error> {
        match *self {
            Function::FilePath(ref f) => evaluate_file_path(state, f),
            Function::FileRegex(ref p, ref r) => evaluate_file_regex(state, p, r),
            Function::ActivePath(ref p) => evaluate_active_path(state, p),
            Function::ActiveRegex(ref r) => evaluate_active_regex(state, r),
            Function::Many(ref p, ref r) => evaluate_many(state, p, r),
            Function::ManyActive(ref r) => evaluate_many_active(state, r),
            Function::Checksum(ref path, ref crc) => evaluate_checksum(state, path, *crc),
            Function::Version(ref p, ref v, ref c) => evaluate_version(state, p, v, *c),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs::{copy, create_dir};
    use std::sync::RwLock;

    use regex::RegexBuilder;
    use tempfile::tempdir;

    use GameType;

    fn state<T: Into<PathBuf>>(data_path: T) -> State {
        state_with_active_plugins(data_path, &[])
    }

    fn state_with_active_plugins<T: Into<PathBuf>>(data_path: T, active_plugins: &[&str]) -> State {
        state_with_data(data_path, "", active_plugins, &[])
    }

    fn state_with_loot_path<T: Into<PathBuf>>(data_path: T, loot_path: &str) -> State {
        state_with_data(data_path, loot_path, &[], &[])
    }

    fn state_with_versions<T: Into<PathBuf>>(
        data_path: T,
        plugin_versions: &[(&str, &str)],
    ) -> State {
        state_with_data(data_path, "", &[], plugin_versions)
    }

    fn state_with_data<T: Into<PathBuf>>(
        data_path: T,
        loot_path: &str,
        active_plugins: &[&str],
        plugin_versions: &[(&str, &str)],
    ) -> State {
        let data_path = data_path.into();
        if !data_path.exists() {
            create_dir(&data_path).unwrap();
        }

        State {
            game_type: GameType::Tes4,
            data_path: data_path,
            loot_path: loot_path.into(),
            active_plugins: active_plugins
                .into_iter()
                .map(|s| s.to_lowercase())
                .collect(),
            crc_cache: RwLock::default(),
            plugin_versions: plugin_versions
                .iter()
                .map(|(p, v)| (p.to_lowercase(), v.to_string()))
                .collect(),
        }
    }

    fn regex(string: &str) -> Regex {
        RegexBuilder::new(string)
            .case_insensitive(true)
            .build()
            .unwrap()
    }

    #[test]
    fn function_file_path_eval_should_return_true_if_the_file_exists_relative_to_the_data_path() {
        let function = Function::FilePath(PathBuf::from("Cargo.toml"));
        let state = state(".");

        assert!(function.eval(&state).unwrap());
    }

    #[test]
    fn function_file_path_eval_should_return_true_if_given_a_plugin_that_is_ghosted() {
        let tmp_dir = tempdir().unwrap();
        let data_path = tmp_dir.path().join("Data");
        let state = state(data_path);

        copy(
            Path::new("testing-plugins/Oblivion/Data/Blank.esp"),
            &state.data_path.join("Blank.esp.ghost"),
        ).unwrap();

        let function = Function::FilePath(PathBuf::from("Blank.esp"));

        assert!(function.eval(&state).unwrap());
    }

    #[test]
    #[allow(non_snake_case)]
    fn function_file_path_eval_should_be_true_if_given_LOOT_and_loot_path_exists() {
        let function = Function::FilePath(PathBuf::from("LOOT"));
        let state = state_with_loot_path(".", "Cargo.toml");

        assert!(function.eval(&state).unwrap());
    }

    #[test]
    #[allow(non_snake_case)]
    fn function_file_path_eval_should_be_false_if_given_LOOT_and_loot_path_does_not_exist() {
        let function = Function::FilePath(PathBuf::from("LOOT"));
        let state = state_with_loot_path(".", "missing");

        assert!(!function.eval(&state).unwrap());
    }

    #[test]
    fn function_file_path_eval_should_not_check_for_ghosted_non_plugin_file() {
        let tmp_dir = tempdir().unwrap();
        let data_path = tmp_dir.path().join("Data");
        let state = state(data_path);

        copy(
            Path::new("Cargo.toml"),
            &state.data_path.join("Cargo.toml.ghost"),
        ).unwrap();

        let function = Function::FilePath(PathBuf::from("Cargo.toml"));

        assert!(!function.eval(&state).unwrap());
    }

    #[test]
    fn function_file_path_eval_should_return_false_if_the_file_does_not_exist() {
        let function = Function::FilePath(PathBuf::from("missing"));
        let state = state(".");

        assert!(!function.eval(&state).unwrap());
    }

    #[test]
    fn function_file_regex_eval_should_be_false_if_no_directory_entries_match() {
        let function = Function::FileRegex(PathBuf::from("."), regex("missing"));
        let state = state(".");

        assert!(!function.eval(&state).unwrap());
    }

    #[test]
    fn function_file_regex_eval_should_be_false_if_the_parent_path_part_is_not_a_directory() {
        let function = Function::FileRegex(PathBuf::from("missing"), regex("Cargo.*"));
        let state = state(".");

        assert!(!function.eval(&state).unwrap());
    }

    #[test]
    fn function_file_regex_eval_should_be_true_if_a_directory_entry_matches() {
        let function = Function::FileRegex(
            PathBuf::from("testing-plugins/Oblivion/Data"),
            regex("Blank\\.esp"),
        );
        let state = state(".");

        assert!(function.eval(&state).unwrap());
    }

    #[test]
    fn function_active_path_eval_should_be_true_if_the_path_is_an_active_plugin() {
        let function = Function::ActivePath(PathBuf::from("Blank.esp"));
        let state = state_with_active_plugins(".", &["Blank.esp"]);

        assert!(function.eval(&state).unwrap());
    }

    #[test]
    fn function_active_path_eval_should_be_case_insensitive() {
        let function = Function::ActivePath(PathBuf::from("Blank.esp"));
        let state = state_with_active_plugins(".", &["blank.esp"]);

        assert!(function.eval(&state).unwrap());
    }

    #[test]
    fn function_active_path_eval_should_be_false_if_the_path_is_not_an_active_plugin() {
        let function = Function::ActivePath(PathBuf::from("inactive.esp"));
        let state = state_with_active_plugins(".", &["Blank.esp"]);

        assert!(!function.eval(&state).unwrap());
    }

    #[test]
    fn function_active_regex_eval_should_be_true_if_the_regex_matches_an_active_plugin() {
        let function = Function::ActiveRegex(regex("Blank\\.esp"));
        let state = state_with_active_plugins(".", &["Blank.esp"]);

        assert!(function.eval(&state).unwrap());
    }

    #[test]
    fn function_active_regex_eval_should_be_false_if_the_regex_does_not_match_an_active_plugin() {
        let function = Function::ActiveRegex(regex("inactive\\.esp"));
        let state = state_with_active_plugins(".", &["Blank.esp"]);

        assert!(!function.eval(&state).unwrap());
    }

    #[test]
    fn function_many_eval_should_be_false_if_no_directory_entries_match() {
        let function = Function::Many(PathBuf::from("."), regex("missing"));
        let state = state(".");

        assert!(!function.eval(&state).unwrap());
    }

    #[test]
    fn function_many_eval_should_be_false_if_the_parent_path_part_is_not_a_directory() {
        let function = Function::Many(PathBuf::from("missing"), regex("Cargo.*"));
        let state = state(".");

        assert!(!function.eval(&state).unwrap());
    }

    #[test]
    fn function_many_eval_should_be_false_if_one_directory_entry_matches() {
        let function = Function::Many(
            PathBuf::from("testing-plugins/Oblivion/Data"),
            regex("Blank\\.esp"),
        );
        let state = state(".");

        assert!(!function.eval(&state).unwrap());
    }

    #[test]
    fn function_many_eval_should_be_true_if_more_than_one_directory_entry_matches() {
        let function = Function::Many(
            PathBuf::from("testing-plugins/Oblivion/Data"),
            regex("Blank.*"),
        );
        let state = state(".");

        assert!(function.eval(&state).unwrap());
    }

    #[test]
    fn function_many_active_eval_should_be_true_if_the_regex_matches_more_than_one_active_plugin() {
        let function = Function::ManyActive(regex("Blank.*"));
        let state = state_with_active_plugins(".", &["Blank.esp", "Blank.esm"]);

        assert!(function.eval(&state).unwrap());
    }

    #[test]
    fn function_many_active_eval_should_be_false_if_one_active_plugin_matches() {
        let function = Function::ManyActive(regex("Blank\\.esp"));
        let state = state_with_active_plugins(".", &["Blank.esp", "Blank.esm"]);

        assert!(!function.eval(&state).unwrap());
    }

    #[test]
    fn function_many_active_eval_should_be_false_if_the_regex_does_not_match_an_active_plugin() {
        let function = Function::ManyActive(regex("inactive\\.esp"));
        let state = state_with_active_plugins(".", &["Blank.esp", "Blank.esm"]);

        assert!(!function.eval(&state).unwrap());
    }

    #[test]
    fn function_checksum_eval_should_be_false_if_the_file_does_not_exist() {
        let function = Function::Checksum(PathBuf::from("missing"), 0x374E2A6F);
        let state = state(".");

        assert!(!function.eval(&state).unwrap());
    }

    #[test]
    fn function_checksum_eval_should_be_false_if_the_file_checksum_does_not_equal_the_given_checksum(
) {
        let function = Function::Checksum(
            PathBuf::from("testing-plugins/Oblivion/Data/Blank.esm"),
            0xDEADBEEF,
        );
        let state = state(".");

        assert!(!function.eval(&state).unwrap());
    }

    #[test]
    fn function_checksum_eval_should_be_true_if_the_file_checksum_equals_the_given_checksum() {
        let function = Function::Checksum(
            PathBuf::from("testing-plugins/Oblivion/Data/Blank.esm"),
            0x374E2A6F,
        );
        let state = state(".");

        assert!(function.eval(&state).unwrap());
    }

    #[test]
    fn function_checksum_eval_should_support_checking_the_crc_of_a_ghosted_plugin() {
        let tmp_dir = tempdir().unwrap();
        let data_path = tmp_dir.path().join("Data");
        let state = state(data_path);

        copy(
            Path::new("testing-plugins/Oblivion/Data/Blank.esm"),
            &state.data_path.join("Blank.esm.ghost"),
        ).unwrap();

        let function = Function::Checksum(PathBuf::from("Blank.esm"), 0x374E2A6F);

        assert!(function.eval(&state).unwrap());
    }

    #[test]
    fn function_checksum_eval_should_not_check_for_ghosted_non_plugin_file() {
        let tmp_dir = tempdir().unwrap();
        let data_path = tmp_dir.path().join("Data");
        let state = state(data_path);

        copy(
            Path::new("testing-plugins/Oblivion/Data/Blank.bsa"),
            &state.data_path.join("Blank.bsa.ghost"),
        ).unwrap();

        let function = Function::Checksum(PathBuf::from("Blank.bsa"), 0x22AB79D9);

        assert!(!function.eval(&state).unwrap());
    }

    #[test]
    #[allow(non_snake_case)]
    fn function_checksum_eval_should_be_true_if_given_LOOT_crc_matches() {
        let function = Function::Checksum(PathBuf::from("LOOT"), 0x374E2A6F);
        let state = state_with_loot_path(".", "testing-plugins/Oblivion/Data/Blank.esm");

        assert!(function.eval(&state).unwrap());
    }

    #[test]
    #[allow(non_snake_case)]
    fn function_checksum_eval_should_be_false_if_given_LOOT_crc_does_not_match() {
        let function = Function::Checksum(PathBuf::from("LOOT"), 0xDEADBEEF);
        let state = state_with_loot_path(".", "testing-plugins/Oblivion/Data/Blank.esm");

        assert!(!function.eval(&state).unwrap());
    }

    #[test]
    fn function_checksum_eval_should_cache_and_use_cached_crcs() {
        let tmp_dir = tempdir().unwrap();
        let data_path = tmp_dir.path().join("Data");
        let state = state(data_path);

        copy(
            Path::new("testing-plugins/Oblivion/Data/Blank.esm"),
            &state.data_path.join("Blank.esm"),
        ).unwrap();

        let function = Function::Checksum(PathBuf::from("Blank.esm"), 0x374E2A6F);

        assert!(function.eval(&state).unwrap());

        // Change the CRC of the file to test that the cached value is used.
        copy(
            Path::new("testing-plugins/Oblivion/Data/Blank.bsa"),
            &state.data_path.join("Blank.esm"),
        ).unwrap();

        let function = Function::Checksum(PathBuf::from("Blank.esm"), 0x374E2A6F);

        assert!(function.eval(&state).unwrap());
    }

    #[test]
    fn function_version_eval_should_be_true_if_the_path_does_not_exist_and_comparator_is_ne() {
        let function =
            Function::Version("missing".into(), "1.0".into(), ComparisonOperator::NotEqual);
        let state = state(".");

        assert!(function.eval(&state).unwrap());
    }

    #[test]
    fn function_version_eval_should_be_true_if_the_path_does_not_exist_and_comparator_is_lt() {
        let function =
            Function::Version("missing".into(), "1.0".into(), ComparisonOperator::LessThan);
        let state = state(".");

        assert!(function.eval(&state).unwrap());
    }

    #[test]
    fn function_version_eval_should_be_true_if_the_path_does_not_exist_and_comparator_is_lteq() {
        let function = Function::Version(
            "missing".into(),
            "1.0".into(),
            ComparisonOperator::LessThanOrEqual,
        );
        let state = state(".");

        assert!(function.eval(&state).unwrap());
    }

    #[test]
    fn function_version_eval_should_be_false_if_the_path_does_not_exist_and_comparator_is_eq() {
        let function = Function::Version("missing".into(), "1.0".into(), ComparisonOperator::Equal);
        let state = state(".");

        assert!(!function.eval(&state).unwrap());
    }

    #[test]
    fn function_version_eval_should_be_false_if_the_path_does_not_exist_and_comparator_is_gt() {
        let function = Function::Version(
            "missing".into(),
            "1.0".into(),
            ComparisonOperator::GreaterThan,
        );
        let state = state(".");

        assert!(!function.eval(&state).unwrap());
    }

    #[test]
    fn function_version_eval_should_be_false_if_the_path_does_not_exist_and_comparator_is_gteq() {
        let function = Function::Version(
            "missing".into(),
            "1.0".into(),
            ComparisonOperator::GreaterThanOrEqual,
        );
        let state = state(".");

        assert!(!function.eval(&state).unwrap());
    }

    #[test]
    fn function_version_eval_should_be_false_if_versions_are_not_equal_and_comparator_is_eq() {
        let function = Function::Version("Blank.esm".into(), "5".into(), ComparisonOperator::Equal);
        let state = state_with_versions("./testing-plugins/Oblivion/Data", &[("Blank.esm", "1")]);

        assert!(!function.eval(&state).unwrap());
    }

    #[test]
    fn function_version_eval_should_be_true_if_versions_are_equal_and_comparator_is_eq() {
        let function = Function::Version("Blank.esm".into(), "5".into(), ComparisonOperator::Equal);
        let state = state_with_versions("./testing-plugins/Oblivion/Data", &[("Blank.esm", "5")]);

        assert!(function.eval(&state).unwrap());
    }

    #[test]
    fn function_version_eval_should_be_false_if_versions_are_equal_and_comparator_is_ne() {
        let function =
            Function::Version("Blank.esm".into(), "5".into(), ComparisonOperator::NotEqual);
        let state = state_with_versions("./testing-plugins/Oblivion/Data", &[("Blank.esm", "5")]);

        assert!(!function.eval(&state).unwrap());
    }

    #[test]
    fn function_version_eval_should_be_true_if_versions_are_not_equal_and_comparator_is_ne() {
        let function =
            Function::Version("Blank.esm".into(), "5".into(), ComparisonOperator::NotEqual);
        let state = state_with_versions("./testing-plugins/Oblivion/Data", &[("Blank.esm", "1")]);

        assert!(function.eval(&state).unwrap());
    }

    #[test]
    fn function_version_eval_should_be_false_if_actual_version_is_eq_and_comparator_is_lt() {
        let function =
            Function::Version("Blank.esm".into(), "5".into(), ComparisonOperator::LessThan);
        let state = state_with_versions("./testing-plugins/Oblivion/Data", &[("Blank.esm", "5")]);

        assert!(!function.eval(&state).unwrap());
    }

    #[test]
    fn function_version_eval_should_be_false_if_actual_version_is_gt_and_comparator_is_lt() {
        let function =
            Function::Version("Blank.esm".into(), "5".into(), ComparisonOperator::LessThan);
        let state = state_with_versions("./testing-plugins/Oblivion/Data", &[("Blank.esm", "6")]);

        assert!(!function.eval(&state).unwrap());
    }

    #[test]
    fn function_version_eval_should_be_true_if_actual_version_is_lt_and_comparator_is_lt() {
        let function =
            Function::Version("Blank.esm".into(), "5".into(), ComparisonOperator::NotEqual);
        let state = state_with_versions("./testing-plugins/Oblivion/Data", &[("Blank.esm", "1")]);

        assert!(function.eval(&state).unwrap());
    }

    #[test]
    fn function_version_eval_should_be_false_if_actual_version_is_eq_and_comparator_is_gt() {
        let function = Function::Version(
            "Blank.esm".into(),
            "5".into(),
            ComparisonOperator::GreaterThan,
        );
        let state = state_with_versions("./testing-plugins/Oblivion/Data", &[("Blank.esm", "5")]);

        assert!(!function.eval(&state).unwrap());
    }

    #[test]
    fn function_version_eval_should_be_false_if_actual_version_is_lt_and_comparator_is_gt() {
        let function = Function::Version(
            "Blank.esm".into(),
            "5".into(),
            ComparisonOperator::GreaterThan,
        );
        let state = state_with_versions("./testing-plugins/Oblivion/Data", &[("Blank.esm", "4")]);

        assert!(!function.eval(&state).unwrap());
    }

    #[test]
    fn function_version_eval_should_be_true_if_actual_version_is_gt_and_comparator_is_gt() {
        let function = Function::Version(
            "Blank.esm".into(),
            "5".into(),
            ComparisonOperator::GreaterThan,
        );
        let state = state_with_versions("./testing-plugins/Oblivion/Data", &[("Blank.esm", "6")]);

        assert!(function.eval(&state).unwrap());
    }

    #[test]
    fn function_version_eval_should_be_false_if_actual_version_is_gt_and_comparator_is_lteq() {
        let function = Function::Version(
            "Blank.esm".into(),
            "5".into(),
            ComparisonOperator::LessThanOrEqual,
        );
        let state = state_with_versions("./testing-plugins/Oblivion/Data", &[("Blank.esm", "6")]);

        assert!(!function.eval(&state).unwrap());
    }

    #[test]
    fn function_version_eval_should_be_true_if_actual_version_is_eq_and_comparator_is_lteq() {
        let function = Function::Version(
            "Blank.esm".into(),
            "5".into(),
            ComparisonOperator::LessThanOrEqual,
        );
        let state = state_with_versions("./testing-plugins/Oblivion/Data", &[("Blank.esm", "5")]);

        assert!(function.eval(&state).unwrap());
    }

    #[test]
    fn function_version_eval_should_be_true_if_actual_version_is_lt_and_comparator_is_lteq() {
        let function = Function::Version(
            "Blank.esm".into(),
            "5".into(),
            ComparisonOperator::LessThanOrEqual,
        );
        let state = state_with_versions("./testing-plugins/Oblivion/Data", &[("Blank.esm", "4")]);

        assert!(function.eval(&state).unwrap());
    }

    #[test]
    fn function_version_eval_should_be_false_if_actual_version_is_lt_and_comparator_is_gteq() {
        let function = Function::Version(
            "Blank.esm".into(),
            "5".into(),
            ComparisonOperator::GreaterThanOrEqual,
        );
        let state = state_with_versions("./testing-plugins/Oblivion/Data", &[("Blank.esm", "4")]);

        assert!(!function.eval(&state).unwrap());
    }

    #[test]
    fn function_version_eval_should_be_true_if_actual_version_is_eq_and_comparator_is_gteq() {
        let function = Function::Version(
            "Blank.esm".into(),
            "5".into(),
            ComparisonOperator::GreaterThanOrEqual,
        );
        let state = state_with_versions("./testing-plugins/Oblivion/Data", &[("Blank.esm", "5")]);

        assert!(function.eval(&state).unwrap());
    }

    #[test]
    fn function_version_eval_should_be_true_if_actual_version_is_gt_and_comparator_is_gteq() {
        let function = Function::Version(
            "Blank.esm".into(),
            "5".into(),
            ComparisonOperator::GreaterThanOrEqual,
        );
        let state = state_with_versions("./testing-plugins/Oblivion/Data", &[("Blank.esm", "6")]);

        assert!(function.eval(&state).unwrap());
    }
}
