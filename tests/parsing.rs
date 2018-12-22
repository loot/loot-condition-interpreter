extern crate loot_condition_interpreter;

use std::str::FromStr;

use loot_condition_interpreter::{Error, Expression};

fn expression(string: &str) -> Result<Expression, Error> {
    Expression::from_str(string)
}

#[test]
fn expression_parsing_should_ignore_whitespace_between_function_arguments() {
    let e = expression("version(\"Cargo.toml\", \"1.2\", ==)");
    assert!(e.is_ok());

    let e = expression("version(\"Unofficial Oblivion Patch.esp\",\"3.4.0\",>=)");
    assert!(e.is_ok());

    let e = expression("version(\"Unofficial Skyrim Patch.esp\", \"2.0\", >=)");
    assert!(e.is_ok());

    let e = expression("version(\"..\\TESV.exe\", \"1.8\", >) and not checksum(\"EternalShineArmorAndWeapons.esp\",3E85A943)");
    assert!(e.is_ok());

    let e = expression("version(\"..\\TESV.exe\",\"1.8\",>) and not checksum(\"EternalShineArmorAndWeapons.esp\",3E85A943)");
    assert!(e.is_ok());

    let e = expression("checksum(\"HM_HotkeyMod.esp\",374C564C)");
    assert!(e.is_ok());

    let e = expression("checksum(\"HM_HotkeyMod.esp\",CF00AFFD)");
    assert!(e.is_ok());

    let e = expression(
        "checksum(\"HM_HotkeyMod.esp\",374C564C) or checksum(\"HM_HotkeyMod.esp\",CF00AFFD)",
    );
    assert!(e.is_ok());

    let e = expression(
        "( checksum(\"HM_HotkeyMod.esp\",374C564C) or checksum(\"HM_HotkeyMod.esp\",CF00AFFD) )",
    );
    assert!(e.is_ok());

    let e = expression("file(\"UFO - Ultimate Follower Overhaul.esp\")");
    assert!(e.is_ok());

    let e = expression("( checksum(\"HM_HotkeyMod.esp\",374C564C) or checksum(\"HM_HotkeyMod.esp\",CF00AFFD) ) and file(\"UFO - Ultimate Follower Overhaul.esp\")");
    assert!(e.is_ok());

    let e = expression("many(\"Deeper Thoughts (\\(Curie\\)|- (Expressive )?Curie)\\.esp\")");
    assert!(e.is_ok());
}
