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
pub static LCI_GAME_TES3: c_int = GameType::Tes3 as c_int;

/// Game code for The Elder Scrolls IV: Oblivion.
#[no_mangle]
pub static LCI_GAME_TES4: c_int = GameType::Tes4 as c_int;

/// Game code for The Elder Scrolls V: Skyrim.
#[no_mangle]
pub static LCI_GAME_TES5: c_int = GameType::Tes5 as c_int;

/// Game code for Fallout 3.
#[no_mangle]
pub static LCI_GAME_FO3: c_int = GameType::Fo3 as c_int;

/// Game code for Fallout: New Vegas.
#[no_mangle]
pub static LCI_GAME_FNV: c_int = GameType::Fonv as c_int;

/// Game code for Fallout 4.
#[no_mangle]
pub static LCI_GAME_FO4: c_int = GameType::Fo4 as c_int;

/// Game code for The Elder Scrolls V: Skyrim Special Edition.
#[no_mangle]
pub static LCI_GAME_TES5SE: c_int = GameType::Tes5se as c_int;

/// Game code for The Elder Scrolls V: Skyrim VR.
#[no_mangle]
pub static LCI_GAME_TES5VR: c_int = GameType::Tes5vr as c_int;

/// Game code for Fallout 4 VR.
#[no_mangle]
pub static LCI_GAME_FO4VR: c_int = GameType::Fo4vr as c_int;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_constants_should_have_expected_integer_values() {
        assert_eq!(8, LCI_GAME_TES3);
        assert_eq!(0, LCI_GAME_TES4);
        assert_eq!(1, LCI_GAME_TES5);
        assert_eq!(2, LCI_GAME_TES5SE);
        assert_eq!(3, LCI_GAME_TES5VR);
        assert_eq!(4, LCI_GAME_FO3);
        assert_eq!(5, LCI_GAME_FNV);
        assert_eq!(6, LCI_GAME_FO4);
        assert_eq!(7, LCI_GAME_FO4VR);
    }
}
