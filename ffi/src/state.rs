use std::ffi::{c_char, c_int};
use std::panic::catch_unwind;
use std::path::PathBuf;
use std::sync::RwLock;

use loot_condition_interpreter::State;

use crate::constants::{
    LCI_ERROR_INVALID_ARGS, LCI_ERROR_PANICKED, LCI_ERROR_POISONED_THREAD_LOCK, LCI_OK,
};
use crate::helpers::{
    error, map_game_type, map_plugin_crcs, map_plugin_versions, to_path_buf_vec, to_str, to_str_vec,
};

#[expect(non_camel_case_types)]
#[derive(Debug)]
pub struct lci_state(pub RwLock<State>);

#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Debug)]
pub struct plugin_version {
    pub plugin_name: *const c_char,
    pub version: *const c_char,
}

#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Debug)]
pub struct plugin_crc {
    pub plugin_name: *const c_char,
    pub crc: u32,
}

#[no_mangle]
pub unsafe extern "C" fn lci_state_create(
    state: *mut *mut lci_state,
    game_type: c_int,
    data_path: *const c_char,
) -> c_int {
    catch_unwind(|| {
        if state.is_null() || data_path.is_null() {
            error(LCI_ERROR_INVALID_ARGS, "Null pointer passed")
        } else {
            let game_type = match map_game_type(game_type) {
                Ok(x) => x,
                Err(x) => return error(x, "Invalid game specified"),
            };

            let data_path = match to_str(data_path) {
                Ok(x) => PathBuf::from(x),
                Err(e) => return e,
            };

            *state = Box::into_raw(Box::new(lci_state(RwLock::new(State::new(
                game_type, data_path,
            )))));

            LCI_OK
        }
    })
    .unwrap_or(LCI_ERROR_PANICKED)
}

#[no_mangle]
pub unsafe extern "C" fn lci_state_destroy(state: *mut lci_state) {
    if !state.is_null() {
        drop(Box::from_raw(state));
    }
}

#[no_mangle]
pub unsafe extern "C" fn lci_state_set_active_plugins(
    state: *mut lci_state,
    plugin_names: *const *const c_char,
    num_plugins: usize,
) -> c_int {
    catch_unwind(|| {
        if state.is_null() {
            error(LCI_ERROR_INVALID_ARGS, "Null state pointer passed")
        } else if plugin_names.is_null() && num_plugins != 0 {
            error(
                LCI_ERROR_INVALID_ARGS,
                "Null plugin_names pointer passed but num_plugins is non-zero",
            )
        } else if !plugin_names.is_null() && num_plugins == 0 {
            error(
                LCI_ERROR_INVALID_ARGS,
                "Non-null plugin_names pointer passed but num_plugins is zero",
            )
        } else {
            let plugins: Vec<&str> = match to_str_vec(plugin_names, num_plugins) {
                Ok(x) => x,
                Err(e) => return e,
            };

            let mut state = match (*state).0.write() {
                Err(e) => return error(LCI_ERROR_POISONED_THREAD_LOCK, &e.to_string()),
                Ok(h) => h,
            };

            state.set_active_plugins(&plugins);

            LCI_OK
        }
    })
    .unwrap_or(LCI_ERROR_PANICKED)
}

#[no_mangle]
pub unsafe extern "C" fn lci_state_set_plugin_versions(
    state: *mut lci_state,
    plugin_versions: *const plugin_version,
    num_plugins: usize,
) -> c_int {
    catch_unwind(|| {
        if state.is_null() {
            error(LCI_ERROR_INVALID_ARGS, "Null state pointer passed")
        } else if plugin_versions.is_null() && num_plugins != 0 {
            error(
                LCI_ERROR_INVALID_ARGS,
                "Null plugin_versions pointer passed but num_plugins is non-zero",
            )
        } else if !plugin_versions.is_null() && num_plugins == 0 {
            error(
                LCI_ERROR_INVALID_ARGS,
                "Non-null plugin_versions pointer passed but num_plugins is zero",
            )
        } else {
            let plugin_versions = match map_plugin_versions(plugin_versions, num_plugins) {
                Ok(x) => x,
                Err(e) => return e,
            };

            let mut state = match (*state).0.write() {
                Err(e) => return error(LCI_ERROR_POISONED_THREAD_LOCK, &e.to_string()),
                Ok(h) => h,
            };

            state.set_plugin_versions(&plugin_versions);

            LCI_OK
        }
    })
    .unwrap_or(LCI_ERROR_PANICKED)
}

#[no_mangle]
pub unsafe extern "C" fn lci_state_set_crc_cache(
    state: *mut lci_state,
    entries: *const plugin_crc,
    num_entries: usize,
) -> c_int {
    catch_unwind(|| {
        if state.is_null() {
            error(LCI_ERROR_INVALID_ARGS, "Null state pointer passed")
        } else if entries.is_null() && num_entries != 0 {
            error(
                LCI_ERROR_INVALID_ARGS,
                "Null entries pointer passed but num_entries is non-zero",
            )
        } else if !entries.is_null() && num_entries == 0 {
            error(
                LCI_ERROR_INVALID_ARGS,
                "Non-null entries pointer passed but num_entries is zero",
            )
        } else {
            let plugin_crcs = match map_plugin_crcs(entries, num_entries) {
                Ok(x) => x,
                Err(e) => return e,
            };

            match (*state).0.write() {
                Err(e) => error(LCI_ERROR_POISONED_THREAD_LOCK, &e.to_string()),
                Ok(mut s) => match s.set_cached_crcs(&plugin_crcs) {
                    Err(e) => error(LCI_ERROR_POISONED_THREAD_LOCK, &e.to_string()),
                    Ok(()) => LCI_OK,
                },
            }
        }
    })
    .unwrap_or(LCI_ERROR_PANICKED)
}

#[no_mangle]
pub unsafe extern "C" fn lci_state_clear_condition_cache(state: *mut lci_state) -> c_int {
    catch_unwind(|| {
        if state.is_null() {
            error(LCI_ERROR_INVALID_ARGS, "Null state pointer passed")
        } else {
            match (*state).0.write() {
                Err(e) => error(LCI_ERROR_POISONED_THREAD_LOCK, &e.to_string()),
                Ok(mut s) => match s.clear_condition_cache() {
                    Err(e) => error(LCI_ERROR_POISONED_THREAD_LOCK, &e.to_string()),
                    Ok(()) => LCI_OK,
                },
            }
        }
    })
    .unwrap_or(LCI_ERROR_PANICKED)
}

/// Sets the external data paths for the given state.
///
/// If the operating environment contains multiple directories containing relevant plugins and other
/// data files, this function can be used to provide those directory paths that are not the game's
/// main data directory so that files in those directories are taken into account when evaluating
/// conditions.
///
/// Returns `LCI_OK` if successful, otherwise a `LCI_ERROR_*` code is returned.
#[no_mangle]
pub unsafe extern "C" fn lci_state_set_additional_data_paths(
    state: *mut lci_state,
    paths: *const *const c_char,
    num_paths: usize,
) -> c_int {
    catch_unwind(|| {
        if state.is_null() || (paths.is_null() && num_paths != 0) {
            return error(LCI_ERROR_INVALID_ARGS, "Null pointer passed");
        }

        let mut state = match (*state).0.write() {
            Err(e) => return error(LCI_ERROR_POISONED_THREAD_LOCK, &e.to_string()),
            Ok(h) => h,
        };

        let additional_data_paths = match to_path_buf_vec(paths, num_paths) {
            Ok(x) => x,
            Err(x) => return error(x, "An external data path contained a null byte"),
        };

        state.set_additional_data_paths(additional_data_paths);

        LCI_OK
    })
    .unwrap_or(LCI_ERROR_PANICKED)
}
