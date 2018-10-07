use std::error::Error;
use std::panic::catch_unwind;
use std::path::PathBuf;
use std::sync::RwLock;

use libc::{c_char, c_int, size_t, uint32_t};
use loot_condition_interpreter::State;

use constants::*;
use helpers::{error, map_game_type, map_plugin_crcs, map_plugin_versions, to_str, to_str_vec};

#[allow(non_camel_case_types)]
#[no_mangle]
pub struct lci_state(pub RwLock<State>);

#[allow(non_camel_case_types)]
#[no_mangle]
#[repr(C)]
pub struct plugin_version {
    pub plugin_name: *const c_char,
    pub version: *const c_char,
}

#[allow(non_camel_case_types)]
#[no_mangle]
#[repr(C)]
pub struct plugin_crc {
    pub plugin_name: *const c_char,
    pub crc: uint32_t,
}

#[no_mangle]
pub unsafe extern "C" fn lci_state_create(
    state: *mut *mut lci_state,
    game_type: c_int,
    data_path: *const c_char,
    loot_path: *const c_char,
) -> c_int {
    catch_unwind(|| {
        if state.is_null() || data_path.is_null() || loot_path.is_null() {
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

            let loot_path = match to_str(loot_path) {
                Ok(x) => PathBuf::from(x),
                Err(e) => return e,
            };

            *state = Box::into_raw(Box::new(lci_state(RwLock::new(State::new(
                game_type, data_path, loot_path,
            )))));

            LCI_OK
        }
    }).unwrap_or(LCI_ERROR_PANICKED)
}

#[no_mangle]
pub unsafe extern "C" fn lci_state_destroy(state: *mut lci_state) {
    if !state.is_null() {
        Box::from_raw(state);
    }
}

#[no_mangle]
pub unsafe extern "C" fn lci_state_set_active_plugins(
    state: *mut lci_state,
    plugin_names: *const *const c_char,
    num_plugins: size_t,
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
                Err(e) => return error(LCI_ERROR_POISONED_THREAD_LOCK, e.description()),
                Ok(h) => h,
            };

            state.set_active_plugins(&plugins);

            LCI_OK
        }
    }).unwrap_or(LCI_ERROR_PANICKED)
}

#[no_mangle]
pub unsafe extern "C" fn lci_state_set_plugin_versions(
    state: *mut lci_state,
    plugin_versions: *const plugin_version,
    num_plugins: size_t,
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
                Err(e) => return error(LCI_ERROR_POISONED_THREAD_LOCK, e.description()),
                Ok(h) => h,
            };

            state.set_plugin_versions(&plugin_versions);

            LCI_OK
        }
    }).unwrap_or(LCI_ERROR_PANICKED)
}

#[no_mangle]
pub unsafe extern "C" fn lci_state_set_crc_cache(
    state: *mut lci_state,
    entries: *const plugin_crc,
    num_entries: size_t,
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
                Err(e) => error(LCI_ERROR_POISONED_THREAD_LOCK, e.description()),
                Ok(mut s) => match s.set_cached_crcs(&plugin_crcs) {
                    Err(e) => error(LCI_ERROR_POISONED_THREAD_LOCK, e.description()),
                    Ok(_) => LCI_OK,
                },
            }
        }
    }).unwrap_or(LCI_ERROR_PANICKED)
}

#[no_mangle]
pub unsafe extern "C" fn lci_state_clear_condition_cache(state: *mut lci_state) -> c_int {
    catch_unwind(|| {
        if state.is_null() {
            error(LCI_ERROR_INVALID_ARGS, "Null state pointer passed")
        } else {
            match (*state).0.write() {
                Err(e) => error(LCI_ERROR_POISONED_THREAD_LOCK, e.description()),
                Ok(mut s) => match s.clear_condition_cache() {
                    Err(e) => error(LCI_ERROR_POISONED_THREAD_LOCK, e.description()),
                    Ok(_) => LCI_OK,
                },
            }
        }
    }).unwrap_or(LCI_ERROR_PANICKED)
}
