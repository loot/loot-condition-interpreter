use std::ffi::{c_char, c_int, CStr, CString};
use std::path::PathBuf;
use std::slice;

use libc::size_t;
use loot_condition_interpreter::{Error, GameType};

use super::ERROR_MESSAGE;
use crate::constants::{
    LCI_ERROR_INTERNAL_LOGIC_ERROR, LCI_ERROR_INVALID_ARGS, LCI_ERROR_IO_ERROR,
    LCI_ERROR_PARSING_ERROR, LCI_ERROR_PE_PARSING_ERROR, LCI_GAME_FALLOUT_3, LCI_GAME_FALLOUT_4,
    LCI_GAME_FALLOUT_4_VR, LCI_GAME_FALLOUT_NV, LCI_GAME_MORROWIND, LCI_GAME_OBLIVION,
    LCI_GAME_OPENMW, LCI_GAME_SKYRIM, LCI_GAME_SKYRIM_SE, LCI_GAME_SKYRIM_VR, LCI_GAME_STARFIELD,
};
use crate::state::{plugin_crc, plugin_version};

pub(crate) fn error(code: c_int, message: &str) -> c_int {
    ERROR_MESSAGE.with(|f| {
        *f.borrow_mut() = CString::new(message.as_bytes())
            .or_else(|_e| CString::new(message.replace('\0', "\\0").as_bytes()))
            .unwrap_or_else(|_e| c"Failed to retrieve error message".into());
    });
    code
}

pub(crate) fn handle_error(err: &Error) -> c_int {
    let code = map_error(err);
    error(code, &format!("{err}"))
}

fn map_error(err: &Error) -> c_int {
    match err {
        Error::ParsingIncomplete(_) | Error::UnconsumedInput(_) | Error::ParsingError(_, _) => {
            LCI_ERROR_PARSING_ERROR
        }
        Error::PeParsingError(_, _) => LCI_ERROR_PE_PARSING_ERROR,
        Error::IoError(_, _) => LCI_ERROR_IO_ERROR,
        _ => LCI_ERROR_INTERNAL_LOGIC_ERROR,
    }
}

pub(crate) fn map_game_type(game_type: c_int) -> Result<GameType, c_int> {
    match game_type {
        x if x == LCI_GAME_OPENMW => Ok(GameType::OpenMW),
        x if x == LCI_GAME_MORROWIND => Ok(GameType::Morrowind),
        x if x == LCI_GAME_OBLIVION => Ok(GameType::Oblivion),
        x if x == LCI_GAME_SKYRIM => Ok(GameType::Skyrim),
        x if x == LCI_GAME_SKYRIM_SE => Ok(GameType::SkyrimSE),
        x if x == LCI_GAME_SKYRIM_VR => Ok(GameType::SkyrimVR),
        x if x == LCI_GAME_FALLOUT_3 => Ok(GameType::Fallout3),
        x if x == LCI_GAME_FALLOUT_NV => Ok(GameType::FalloutNV),
        x if x == LCI_GAME_FALLOUT_4 => Ok(GameType::Fallout4),
        x if x == LCI_GAME_FALLOUT_4_VR => Ok(GameType::Fallout4VR),
        x if x == LCI_GAME_STARFIELD => Ok(GameType::Starfield),
        _ => Err(LCI_ERROR_INVALID_ARGS),
    }
}

pub(crate) unsafe fn to_str<'a>(c_string: *const c_char) -> Result<&'a str, c_int> {
    if c_string.is_null() {
        Err(error(LCI_ERROR_INVALID_ARGS, "Null pointer passed"))
    } else {
        CStr::from_ptr(c_string)
            .to_str()
            .map_err(|_e| error(LCI_ERROR_INVALID_ARGS, "Non-UTF-8 string passed"))
    }
}

pub(crate) unsafe fn to_vec<U, V, F>(
    array: *const U,
    array_size: size_t,
    mapper: F,
) -> Result<Vec<V>, c_int>
where
    F: Fn(&U) -> Result<V, c_int>,
{
    if array.is_null() || array_size == 0 {
        Ok(Vec::new())
    } else {
        slice::from_raw_parts(array, array_size)
            .iter()
            .map(mapper)
            .collect()
    }
}

pub(crate) unsafe fn to_str_vec<'a>(
    array: *const *const c_char,
    array_size: size_t,
) -> Result<Vec<&'a str>, c_int> {
    to_vec(array, array_size, |c| to_str(*c))
}

pub(crate) unsafe fn to_path_buf_vec(
    array: *const *const c_char,
    array_size: size_t,
) -> Result<Vec<PathBuf>, c_int> {
    to_vec(array, array_size, |c| to_str(*c).map(PathBuf::from))
}

unsafe fn map_plugin_version(c_object: &plugin_version) -> Result<(String, String), c_int> {
    to_str(c_object.plugin_name)
        .and_then(|n| to_str(c_object.version).map(|v| (n.into(), v.into())))
}

pub(crate) unsafe fn map_plugin_versions(
    plugin_versions: *const plugin_version,
    num_plugins: size_t,
) -> Result<Vec<(String, String)>, c_int> {
    to_vec(plugin_versions, num_plugins, |v| map_plugin_version(v))
}

pub(crate) unsafe fn map_plugin_crcs(
    plugin_crcs: *const plugin_crc,
    num_entries: size_t,
) -> Result<Vec<(String, u32)>, c_int> {
    to_vec(plugin_crcs, num_entries, |v| {
        to_str(v.plugin_name).map(|s| (s.into(), v.crc))
    })
}
