use libc::c_int;

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
pub static LCI_GAME_MORROWIND: c_int = 8;

/// Game code for The Elder Scrolls IV: Oblivion.
#[no_mangle]
pub static LCI_GAME_OBLIVION: c_int = 0;

/// Game code for The Elder Scrolls V: Skyrim.
#[no_mangle]
pub static LCI_GAME_SKYRIM: c_int = 1;

/// Game code for Fallout 3.
#[no_mangle]
pub static LCI_GAME_FALLOUT_3: c_int = 4;

/// Game code for Fallout: New Vegas.
#[no_mangle]
pub static LCI_GAME_FALLOUT_NV: c_int = 5;

/// Game code for Fallout 4.
#[no_mangle]
pub static LCI_GAME_FALLOUT_4: c_int = 6;

/// Game code for The Elder Scrolls V: Skyrim Special Edition.
#[no_mangle]
pub static LCI_GAME_SKYRIM_SE: c_int = 2;

/// Game code for The Elder Scrolls V: Skyrim VR.
#[no_mangle]
pub static LCI_GAME_SKYRIM_VR: c_int = 3;

/// Game code for Fallout 4 VR.
#[no_mangle]
pub static LCI_GAME_FALLOUT_4_VR: c_int = 7;
