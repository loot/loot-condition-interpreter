use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};

use super::Function;
use Error;
use State;

fn has_plugin_file_extension(path: &Path, state: &State) -> bool {
    match path.extension().and_then(OsStr::to_str) {
        Some("esp") | Some("esm") => true,
        Some("esl") if state.game_type.supports_light_plugins() => true,
        Some("ghost") => path
            .file_stem()
            .map(|s| has_plugin_file_extension(Path::new(s), state))
            .unwrap_or(false),
        _ => false,
    }
}

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

fn evaluate_file_path(state: &State, file_path: &Path) -> Result<bool, Error> {
    if equals(file_path, "LOOT") {
        return Ok(true);
    }

    let path = state.data_path.join(file_path);
    let exists = path.exists();

    if !exists && has_plugin_file_extension(&path, state) {
        Ok(add_extension(&path, "ghost").exists())
    } else {
        Ok(exists)
    }
}

impl Function {
    pub fn eval(&self, state: &State) -> Result<bool, Error> {
        // TODO: Handle all variants.
        // TODO: Paths may not lead outside game directory.
        match *self {
            Function::FilePath(ref f) => evaluate_file_path(state, f),
            _ => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs::{copy, create_dir};

    use tempfile::tempdir;

    use GameType;

    fn state<T: Into<PathBuf>>(data_path: T) -> State {
        let data_path = data_path.into();
        if !data_path.exists() {
            create_dir(&data_path).unwrap();
        }

        State {
            game_type: GameType::tes4,
            data_path: data_path,
        }
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
    fn function_file_path_eval_should_be_true_if_given_LOOT() {
        let function = Function::FilePath(PathBuf::from("LOOT"));
        let state = state(".");

        assert!(function.eval(&state).unwrap());
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
        unimplemented!();
    }

    #[test]
    fn function_file_regex_eval_should_be_false_if_the_parent_path_part_is_not_a_directory() {
        unimplemented!();
    }

    #[test]
    fn function_file_regex_eval_should_be_true_if_a_directory_entry_matches() {
        unimplemented!();
    }
}
