use std::ffi::{CStr, CString};
use std::slice;

use libc::{c_char, c_int, size_t};
use loot_condition_interpreter::{Error, GameType};

use super::ERROR_MESSAGE;
use crate::constants::*;
use crate::state::{plugin_crc, plugin_version};

pub fn error(code: c_int, message: &str) -> c_int {
    ERROR_MESSAGE.with(|f| {
        *f.borrow_mut() = unsafe { CString::from_vec_unchecked(message.as_bytes().to_vec()) }
    });
    code
}

pub fn handle_error(err: Error) -> c_int {
    let code = map_error(&err);
    error(code, &format!("{}", err))
}

fn map_error(err: &Error) -> c_int {
    match err {
        Error::ParsingIncomplete => LCI_ERROR_PARSING_ERROR,
        Error::UnconsumedInput(_) => LCI_ERROR_PARSING_ERROR,
        Error::ParsingError(_, _) => LCI_ERROR_PARSING_ERROR,
        Error::PeParsingError(_, _) => LCI_ERROR_PE_PARSING_ERROR,
        Error::IoError(_, _) => LCI_ERROR_IO_ERROR,
    }
}

pub fn map_game_type(game_type: c_int) -> Result<GameType, c_int> {
    match game_type {
        x if x == LCI_GAME_TES3 => Ok(GameType::Morrowind),
        x if x == LCI_GAME_TES4 => Ok(GameType::Oblivion),
        x if x == LCI_GAME_TES5 => Ok(GameType::Skyrim),
        x if x == LCI_GAME_TES5SE => Ok(GameType::SkyrimSE),
        x if x == LCI_GAME_TES5VR => Ok(GameType::SkyrimVR),
        x if x == LCI_GAME_FO3 => Ok(GameType::Fallout3),
        x if x == LCI_GAME_FNV => Ok(GameType::FalloutNV),
        x if x == LCI_GAME_FO4 => Ok(GameType::Fallout4),
        x if x == LCI_GAME_FO4VR => Ok(GameType::Fallout4VR),
        _ => Err(LCI_ERROR_INVALID_ARGS),
    }
}

pub unsafe fn to_str<'a>(c_string: *const c_char) -> Result<&'a str, c_int> {
    if c_string.is_null() {
        Err(error(LCI_ERROR_INVALID_ARGS, "Null pointer passed"))
    } else {
        CStr::from_ptr(c_string)
            .to_str()
            .map_err(|_| error(LCI_ERROR_INVALID_ARGS, "Non-UTF-8 string passed"))
    }
}

pub unsafe fn to_vec<U, V, F>(array: *const U, array_size: size_t, mapper: F) -> Result<Vec<V>, i32>
where
    F: Fn(&U) -> Result<V, i32>,
{
    if array.is_null() || array_size == 0 {
        Ok(Vec::new())
    } else {
        slice::from_raw_parts(array, array_size)
            .iter()
            .map(|c| mapper(c))
            .collect()
    }
}

pub unsafe fn to_str_vec<'a>(
    array: *const *const c_char,
    array_size: size_t,
) -> Result<Vec<&'a str>, i32> {
    to_vec(array, array_size, |c| to_str(*c))
}

unsafe fn map_plugin_version(c_object: &plugin_version) -> Result<(String, String), i32> {
    to_str(c_object.plugin_name)
        .and_then(|n| to_str(c_object.version).map(|v| (n.into(), v.into())))
}

pub unsafe fn map_plugin_versions(
    plugin_versions: *const plugin_version,
    num_plugins: size_t,
) -> Result<Vec<(String, String)>, i32> {
    to_vec(plugin_versions, num_plugins, |v| map_plugin_version(v))
}

pub unsafe fn map_plugin_crcs(
    plugin_crcs: *const plugin_crc,
    num_entries: size_t,
) -> Result<Vec<(String, u32)>, i32> {
    to_vec(plugin_crcs, num_entries, |v| {
        to_str(v.plugin_name).map(|s| (s.into(), v.crc))
    })
}
