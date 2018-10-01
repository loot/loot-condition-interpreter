use ::Error;
use super::Function;

impl Function {
    pub fn eval(&self) -> Result<bool, Error> {
        // TODO: Handle all variants.
        // TODO: Paths may not lead outside game directory.
        match *self {
            Function::FilePath(ref f) => Ok(f.exists()),
            _ => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use function::Function;

    use std::path::PathBuf;

    #[test]
    fn function_file_path_eval_should_return_true_if_the_file_exists_relative_to_the_data_path() {
        let function = Function::FilePath(PathBuf::from("Cargo.toml"));

        assert!(function.eval().unwrap());

        unimplemented!("not yet any way to actually specify the data path");
    }

    #[test]
    fn function_file_path_eval_should_return_true_if_given_a_plugin_that_is_ghosted() {
        let function = Function::FilePath(PathBuf::from("test.esp"));

        assert!(function.eval().unwrap());

        unimplemented!("need to add tempdir and create a test.esp.ghost");
    }

    #[test]
    #[allow(non_snake_case)]
    fn function_file_path_eval_should_be_true_if_given_LOOT() {
        unimplemented!();
    }

    #[test]
    fn function_file_path_eval_should_not_check_for_ghosted_non_plugin_file() {
        unimplemented!();
    }

    #[test]
    fn function_file_path_eval_should_error_if_the_path_is_outside_game_directory() {
        unimplemented!("to do");
    }

    #[test]
    fn function_file_path_eval_should_return_false_if_the_file_does_not_exist() {
        let function = Function::FilePath(PathBuf::from("missing"));

        assert!(!function.eval().unwrap());
    }

    #[test]
    fn function_file_regex_eval_should_error_if_the_path_is_outside_game_directory() {
        unimplemented!();
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
