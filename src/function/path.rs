use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

use crate::{GameType, State};

const GHOST_EXTENSION: &str = "ghost";
const GHOST_EXTENSION_WITH_PERIOD: &str = ".ghost";

fn is_unghosted_plugin_file_extension(game_type: GameType, extension: &OsStr) -> bool {
    extension.eq_ignore_ascii_case("esp")
        || extension.eq_ignore_ascii_case("esm")
        || (game_type.supports_light_plugins() && extension.eq_ignore_ascii_case("esl"))
}

fn has_unghosted_plugin_file_extension(game_type: GameType, path: &Path) -> bool {
    match path.extension() {
        Some(ext) => is_unghosted_plugin_file_extension(game_type, ext),
        _ => false,
    }
}

pub fn has_plugin_file_extension(game_type: GameType, path: &Path) -> bool {
    match path.extension() {
        Some(ext) if ext.eq_ignore_ascii_case(GHOST_EXTENSION) => path
            .file_stem()
            .map(|s| has_unghosted_plugin_file_extension(game_type, Path::new(s)))
            .unwrap_or(false),
        Some(ext) => is_unghosted_plugin_file_extension(game_type, ext),
        _ => false,
    }
}

fn add_ghost_extension(path: PathBuf) -> PathBuf {
    match path.extension() {
        Some(e) => {
            let mut new_extension = e.to_os_string();
            new_extension.push(GHOST_EXTENSION_WITH_PERIOD);
            path.with_extension(&new_extension)
        }
        None => path.with_extension(GHOST_EXTENSION),
    }
}

pub fn normalise_file_name(game_type: GameType, name: &OsStr) -> &OsStr {
    let path = Path::new(name);
    if path
        .extension()
        .map(|s| s.eq_ignore_ascii_case(GHOST_EXTENSION))
        .unwrap_or(false)
    {
        // name ends in .ghost, trim it and then check the file extension.
        if let Some(stem) = path.file_stem() {
            if has_unghosted_plugin_file_extension(game_type, Path::new(stem)) {
                return stem;
            }
        }
    }

    name
}

pub fn resolve_path(state: &State, path: &Path) -> PathBuf {
    // First check external data paths, as files there may override files in the main data path.
    for data_path in &state.additional_data_paths {
        let mut path = data_path.join(path);

        if path.exists() {
            return path;
        }

        if has_unghosted_plugin_file_extension(state.game_type, &path) {
            path = add_ghost_extension(path);
        }

        if path.exists() {
            return path;
        }
    }

    // Now check the main data path.
    let path = state.data_path.join(path);

    if !path.exists() && has_unghosted_plugin_file_extension(state.game_type, &path) {
        add_ghost_extension(path)
    } else {
        path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_unghosted_plugin_file_extension_should_be_true_for_esp_for_all_game_types() {
        let extension = OsStr::new("Esp");

        assert!(is_unghosted_plugin_file_extension(
            GameType::Morrowind,
            extension
        ));
        assert!(is_unghosted_plugin_file_extension(
            GameType::Oblivion,
            extension
        ));
        assert!(is_unghosted_plugin_file_extension(
            GameType::Skyrim,
            extension
        ));
        assert!(is_unghosted_plugin_file_extension(
            GameType::SkyrimSE,
            extension
        ));
        assert!(is_unghosted_plugin_file_extension(
            GameType::SkyrimVR,
            extension
        ));
        assert!(is_unghosted_plugin_file_extension(
            GameType::Fallout3,
            extension
        ));
        assert!(is_unghosted_plugin_file_extension(
            GameType::FalloutNV,
            extension
        ));
        assert!(is_unghosted_plugin_file_extension(
            GameType::Fallout4,
            extension
        ));
        assert!(is_unghosted_plugin_file_extension(
            GameType::Fallout4VR,
            extension
        ));
    }

    #[test]
    fn is_unghosted_plugin_file_extension_should_be_true_for_esm_for_all_game_types() {
        let extension = OsStr::new("Esm");

        assert!(is_unghosted_plugin_file_extension(
            GameType::Morrowind,
            extension
        ));
        assert!(is_unghosted_plugin_file_extension(
            GameType::Oblivion,
            extension
        ));
        assert!(is_unghosted_plugin_file_extension(
            GameType::Skyrim,
            extension
        ));
        assert!(is_unghosted_plugin_file_extension(
            GameType::SkyrimSE,
            extension
        ));
        assert!(is_unghosted_plugin_file_extension(
            GameType::SkyrimVR,
            extension
        ));
        assert!(is_unghosted_plugin_file_extension(
            GameType::Fallout3,
            extension
        ));
        assert!(is_unghosted_plugin_file_extension(
            GameType::FalloutNV,
            extension
        ));
        assert!(is_unghosted_plugin_file_extension(
            GameType::Fallout4,
            extension
        ));
        assert!(is_unghosted_plugin_file_extension(
            GameType::Fallout4VR,
            extension
        ));
    }

    #[test]
    fn is_unghosted_plugin_file_extension_should_be_true_for_esl_for_tes5se_tes5vr_fo4_and_fo4vr() {
        let extension = OsStr::new("Esl");

        assert!(is_unghosted_plugin_file_extension(
            GameType::SkyrimSE,
            extension
        ));
        assert!(is_unghosted_plugin_file_extension(
            GameType::SkyrimVR,
            extension
        ));
        assert!(is_unghosted_plugin_file_extension(
            GameType::Fallout4,
            extension
        ));
        assert!(is_unghosted_plugin_file_extension(
            GameType::Fallout4VR,
            extension
        ));
    }

    #[test]
    fn is_unghosted_plugin_file_extension_should_be_false_for_esl_for_tes3_to_5_fo3_and_fonv() {
        let extension = OsStr::new("Esl");

        assert!(!is_unghosted_plugin_file_extension(
            GameType::Morrowind,
            extension
        ));
        assert!(!is_unghosted_plugin_file_extension(
            GameType::Oblivion,
            extension
        ));
        assert!(!is_unghosted_plugin_file_extension(
            GameType::Skyrim,
            extension
        ));
        assert!(!is_unghosted_plugin_file_extension(
            GameType::Fallout3,
            extension
        ));
        assert!(!is_unghosted_plugin_file_extension(
            GameType::FalloutNV,
            extension
        ));
    }

    #[test]
    fn is_unghosted_plugin_file_extension_should_be_false_for_ghost_for_all_game_types() {
        let extension = OsStr::new("Ghost");

        assert!(!is_unghosted_plugin_file_extension(
            GameType::Morrowind,
            extension
        ));
        assert!(!is_unghosted_plugin_file_extension(
            GameType::Oblivion,
            extension
        ));
        assert!(!is_unghosted_plugin_file_extension(
            GameType::Skyrim,
            extension
        ));
        assert!(!is_unghosted_plugin_file_extension(
            GameType::SkyrimSE,
            extension
        ));
        assert!(!is_unghosted_plugin_file_extension(
            GameType::SkyrimVR,
            extension
        ));
        assert!(!is_unghosted_plugin_file_extension(
            GameType::Fallout3,
            extension
        ));
        assert!(!is_unghosted_plugin_file_extension(
            GameType::FalloutNV,
            extension
        ));
        assert!(!is_unghosted_plugin_file_extension(
            GameType::Fallout4,
            extension
        ));
        assert!(!is_unghosted_plugin_file_extension(
            GameType::Fallout4VR,
            extension
        ));
    }

    #[test]
    fn is_unghosted_plugin_file_extension_should_be_false_for_non_esp_esm_esl_for_all_game_types() {
        let extension = OsStr::new("txt");

        assert!(!is_unghosted_plugin_file_extension(
            GameType::Morrowind,
            extension
        ));
        assert!(!is_unghosted_plugin_file_extension(
            GameType::Oblivion,
            extension
        ));
        assert!(!is_unghosted_plugin_file_extension(
            GameType::Skyrim,
            extension
        ));
        assert!(!is_unghosted_plugin_file_extension(
            GameType::SkyrimSE,
            extension
        ));
        assert!(!is_unghosted_plugin_file_extension(
            GameType::SkyrimVR,
            extension
        ));
        assert!(!is_unghosted_plugin_file_extension(
            GameType::Fallout3,
            extension
        ));
        assert!(!is_unghosted_plugin_file_extension(
            GameType::FalloutNV,
            extension
        ));
        assert!(!is_unghosted_plugin_file_extension(
            GameType::Fallout4,
            extension
        ));
        assert!(!is_unghosted_plugin_file_extension(
            GameType::Fallout4VR,
            extension
        ));
    }

    #[test]
    fn has_unghosted_plugin_file_extension_should_return_false_if_the_path_has_no_extension() {
        assert!(!has_unghosted_plugin_file_extension(
            GameType::Skyrim,
            Path::new("file")
        ));
    }

    #[test]
    fn has_unghosted_plugin_file_extension_should_return_false_if_the_path_has_a_non_plugin_extension(
    ) {
        assert!(!has_unghosted_plugin_file_extension(
            GameType::Skyrim,
            Path::new("plugin.bsa")
        ));
    }

    #[test]
    fn has_unghosted_plugin_file_extension_should_return_false_if_the_path_has_a_ghosted_plugin_extension(
    ) {
        assert!(!has_unghosted_plugin_file_extension(
            GameType::Skyrim,
            Path::new("plugin.esp.ghost")
        ));
    }

    #[test]
    fn has_unghosted_plugin_file_extension_should_return_true_if_the_path_has_an_unghosted_plugin_extension(
    ) {
        assert!(has_unghosted_plugin_file_extension(
            GameType::Skyrim,
            Path::new("plugin.esp")
        ));
    }

    #[test]
    fn has_plugin_file_extension_should_return_true_if_the_path_has_an_unghosted_plugin_extension()
    {
        assert!(has_plugin_file_extension(
            GameType::Skyrim,
            Path::new("plugin.esp")
        ));
    }

    #[test]
    fn has_plugin_file_extension_should_return_true_if_the_path_has_a_ghosted_plugin_extension() {
        assert!(has_plugin_file_extension(
            GameType::Skyrim,
            Path::new("plugin.esp.Ghost")
        ));
    }

    #[test]
    fn has_plugin_file_extension_should_return_false_if_the_path_has_a_non_plugin_extension() {
        assert!(!has_plugin_file_extension(
            GameType::Skyrim,
            Path::new("plugin.bsa")
        ));
    }

    #[test]
    fn has_plugin_file_extension_should_return_false_if_the_path_has_a_ghosted_non_plugin_extension(
    ) {
        assert!(!has_plugin_file_extension(
            GameType::Skyrim,
            Path::new("plugin.bsa.Ghost")
        ));
    }

    #[test]
    fn has_plugin_file_extension_should_return_false_if_the_path_has_only_ghost_extension() {
        assert!(!has_plugin_file_extension(
            GameType::Skyrim,
            Path::new("plugin.Ghost")
        ));
    }

    #[test]
    fn has_plugin_file_extension_should_return_false_if_the_path_has_no_extension() {
        assert!(!has_plugin_file_extension(
            GameType::Skyrim,
            Path::new("plugin")
        ));
    }

    #[test]
    fn add_ghost_extension_should_add_dot_ghost_to_an_existing_extension() {
        let path = add_ghost_extension("plugin.esp".into());
        assert_eq!(PathBuf::from("plugin.esp.ghost"), path);
    }

    #[test]
    fn add_ghost_extension_should_add_dot_ghost_to_an_a_path_with_no_extension() {
        let path = add_ghost_extension("plugin".into());
        assert_eq!(PathBuf::from("plugin.ghost"), path);
    }

    #[test]
    fn resolve_path_should_return_the_data_path_prefixed_path_if_it_exists() {
        let data_path = PathBuf::from(".");
        let state = State::new(GameType::Skyrim, data_path.clone());
        let input_path = Path::new("README.md");
        let resolved_path = resolve_path(&state, input_path);

        assert_eq!(data_path.join(input_path), resolved_path);
    }

    #[test]
    fn resolve_path_should_return_the_data_path_prefixed_path_if_it_does_not_exist_and_is_not_an_unghosted_plugin_filename(
    ) {
        let data_path = PathBuf::from(".");
        let state = State::new(GameType::Skyrim, data_path.clone());
        let input_path = Path::new("plugin.esp.ghost");
        let resolved_path = resolve_path(&state, input_path);

        assert_eq!(data_path.join(input_path), resolved_path);

        let input_path = Path::new("file.txt");
        let resolved_path = resolve_path(&state, input_path);

        assert_eq!(data_path.join(input_path), resolved_path);
    }

    #[test]
    fn resolve_path_should_return_the_given_data_relative_path_plus_a_ghost_extension_if_the_plugin_path_does_not_exist(
    ) {
        let data_path = PathBuf::from(".");
        let state = State::new(GameType::Skyrim, data_path.clone());
        let input_path = Path::new("plugin.esp");
        let resolved_path = resolve_path(&state, input_path);

        assert_eq!(
            data_path.join(input_path.with_extension("esp.ghost")),
            resolved_path
        );
    }

    #[test]
    fn resolve_path_should_check_external_data_paths_in_order_before_data_path() {
        use std::fs::copy;
        use std::fs::create_dir;

        let tmp_dir = tempfile::tempdir().unwrap();
        let external_data_path_1 = tmp_dir.path().join("Data1");
        let external_data_path_2 = tmp_dir.path().join("Data2");
        let data_path = tmp_dir.path().join("Data3");

        create_dir(&external_data_path_1).unwrap();
        create_dir(&external_data_path_2).unwrap();
        create_dir(&data_path).unwrap();
        copy(
            Path::new("Cargo.toml"),
            external_data_path_2.join("Cargo.toml"),
        )
        .unwrap();
        copy(Path::new("Cargo.toml"), data_path.join("Cargo.toml")).unwrap();

        let mut state = State::new(GameType::Skyrim, data_path);
        state.set_additional_data_paths(vec![external_data_path_1, external_data_path_2.clone()]);

        let input_path = Path::new("Cargo.toml");
        let resolved_path = resolve_path(&state, input_path);

        assert_eq!(external_data_path_2.join(input_path), resolved_path);
    }
}
