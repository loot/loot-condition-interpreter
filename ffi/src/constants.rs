use libc::c_int;
use loot_condition_interpreter::GameType;

#[no_mangle]
pub static LCI_OK: c_int = 0;

#[no_mangle]
pub static LCI_RESULT_FALSE: c_int = 0;

#[no_mangle]
pub static LCI_RESULT_TRUE: c_int = 1;

/// Invalid arguments were given for the function.
#[no_mangle]
pub static LCI_ERROR_INVALID_ARGS: c_int = -1;

/// Something went wrong while parsing the condition expression.
#[no_mangle]
pub static LCI_ERROR_PARSING_ERROR: c_int = -2;

/// Something went wrong while getting the version of an executable.
#[no_mangle]
pub static LCI_ERROR_PE_PARSING_ERROR: c_int = -3;

/// Some sort of I/O error occurred.
#[no_mangle]
pub static LCI_ERROR_IO_ERROR: c_int = -4;

/// Something panicked.
#[no_mangle]
pub static LCI_ERROR_PANICKED: c_int = -5;

/// A thread lock was poisoned.
#[no_mangle]
pub static LCI_ERROR_POISONED_THREAD_LOCK: c_int = -6;

/// Failed to encode string as a C string, e.g. because there was a nul present.
#[no_mangle]
pub static LCI_ERROR_TEXT_ENCODE_FAIL: c_int = -7;

/// Game code for The Elder Scrolls III: Morrowind.
#[no_mangle]
pub static LCI_GAME_MORROWIND: c_int = GameType::Morrowind as c_int;

/// Game code for The Elder Scrolls IV: Oblivion.
#[no_mangle]
pub static LCI_GAME_OBLIVION: c_int = GameType::Oblivion as c_int;

/// Game code for The Elder Scrolls V: Skyrim.
#[no_mangle]
pub static LCI_GAME_SKYRIM: c_int = GameType::Skyrim as c_int;

/// Game code for Fallout 3.
#[no_mangle]
pub static LCI_GAME_FALLOUT_3: c_int = GameType::Fallout3 as c_int;

/// Game code for Fallout: New Vegas.
#[no_mangle]
pub static LCI_GAME_FALLOUT_NV: c_int = GameType::FalloutNV as c_int;

/// Game code for Fallout 4.
#[no_mangle]
pub static LCI_GAME_FALLOUT_4: c_int = GameType::Fallout4 as c_int;

/// Game code for The Elder Scrolls V: Skyrim Special Edition.
#[no_mangle]
pub static LCI_GAME_SKYRIM_SE: c_int = GameType::SkyrimSE as c_int;

/// Game code for The Elder Scrolls V: Skyrim VR.
#[no_mangle]
pub static LCI_GAME_SKYRIM_VR: c_int = GameType::SkyrimVR as c_int;

/// Game code for Fallout 4 VR.
#[no_mangle]
pub static LCI_GAME_FALLOUT_4_VR: c_int = GameType::Fallout4VR as c_int;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_constants_should_have_expected_integer_values() {
        assert_eq!(8, LCI_GAME_MORROWIND);
        assert_eq!(0, LCI_GAME_OBLIVION);
        assert_eq!(1, LCI_GAME_SKYRIM);
        assert_eq!(2, LCI_GAME_SKYRIM_SE);
        assert_eq!(3, LCI_GAME_SKYRIM_VR);
        assert_eq!(4, LCI_GAME_FALLOUT_3);
        assert_eq!(5, LCI_GAME_FALLOUT_NV);
        assert_eq!(6, LCI_GAME_FALLOUT_4);
        assert_eq!(7, LCI_GAME_FALLOUT_4_VR);
    }
}
