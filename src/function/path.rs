use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
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

pub fn has_ghosted_plugin_file_extension(game_type: GameType, path: &Path) -> bool {
    match path.extension() {
        Some(ext) if ext.eq_ignore_ascii_case(GHOST_EXTENSION) => path
            .file_stem()
            .map(|s| has_unghosted_plugin_file_extension(game_type, Path::new(s)))
            .unwrap_or(false),
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

fn get_ghosted_filename(path: &Path) -> Option<OsString> {
    let mut filename = path.file_name()?.to_os_string();
    filename.push(GHOST_EXTENSION_WITH_PERIOD);
    Some(filename)
}

fn add_ghost_extension(
    path: &Path,
    ghosted_plugins: &HashMap<PathBuf, Vec<OsString>>,
) -> Option<PathBuf> {
    // Can't just append a .ghost extension as the filesystem may be case-sensitive and the ghosted
    // file may have a .GHOST extension (for example). Instead loop through the other files in the
    // same parent directory and look for one that's unicode-case-insensitively-equal.
    let expected_filename = get_ghosted_filename(&path)?;
    let expected_filename = expected_filename.to_str()?;
    let parent_path = path.parent()?;

    let ghosted_plugins = ghosted_plugins.get(&parent_path.to_path_buf())?;

    for ghosted_plugin in ghosted_plugins {
        let ghosted_plugin_str = ghosted_plugin.to_str()?;

        if unicase::eq(expected_filename, ghosted_plugin_str) {
            return Some(parent_path.join(ghosted_plugin));
        }
    }

    None
}

pub fn resolve_path(state: &State, path: &Path) -> PathBuf {
    // First check external data paths, as files there may override files in the main data path.
    for data_path in &state.additional_data_paths {
        let mut path = data_path.join(path);

        if path.exists() {
            return path;
        }

        if has_unghosted_plugin_file_extension(state.game_type, &path) {
            if let Some(ghosted_path) = add_ghost_extension(&path, &state.ghosted_plugins) {
                path = ghosted_path
            }
        }

        if path.exists() {
            return path;
        }
    }

    // Now check the main data path.
    let path = state.data_path.join(path);

    if !path.exists() && has_unghosted_plugin_file_extension(state.game_type, &path) {
        add_ghost_extension(&path, &state.ghosted_plugins).unwrap_or(path)
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
    fn add_ghost_extension_should_return_none_if_the_given_parent_path_is_not_in_hashmap() {
        let path = Path::new("subdir/plugin.esp");
        let result = add_ghost_extension(path, &HashMap::new());

        assert!(result.is_none());
    }

    #[test]
    fn add_ghost_extension_should_return_none_if_the_given_parent_path_has_no_ghosted_plugins() {
        let path = Path::new("subdir/plugin.esp");
        let mut map = HashMap::new();
        map.insert(PathBuf::from("subdir"), Vec::new());

        let result = add_ghost_extension(path, &map);

        assert!(result.is_none());
    }

    #[test]
    fn add_ghost_extension_should_return_none_if_the_given_parent_path_has_no_matching_ghosted_plugins(
    ) {
        let path = Path::new("subdir/plugin.esp");
        let mut map = HashMap::new();
        map.insert(
            PathBuf::from("subdir"),
            vec![OsString::from("plugin.esm.ghost")],
        );
        let result = add_ghost_extension(path, &map);

        assert!(result.is_none());
    }

    #[test]
    fn add_ghost_extension_should_return_some_if_the_given_parent_path_has_a_case_insensitively_equal_ghosted_plugin(
    ) {
        let path = Path::new("subdir/plugin.esp");
        let ghosted_plugin = "Plugin.ESp.GHoST";
        let mut map = HashMap::new();
        map.insert(
            PathBuf::from("subdir"),
            vec![OsString::from(ghosted_plugin)],
        );
        let result = add_ghost_extension(path, &map);

        assert!(result.is_some());
        assert_eq!(Path::new("subdir").join(ghosted_plugin), result.unwrap());
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
        let mut state = State::new(GameType::Skyrim, data_path.clone());
        state
            .ghosted_plugins
            .insert(data_path.clone(), vec![OsString::from("plugin.esp.ghost")]);

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
