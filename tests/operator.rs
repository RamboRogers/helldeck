use std::process::Command;
fn call(a: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_helldeck"))
        .args(a)
        .output()
        .unwrap()
}
#[test]
fn preview_has_mission_shortcuts_and_sync() {
    let out = call(&["layout"]);
    assert!(out.status.success(), "layout failed");
    let view = String::from_utf8(out.stdout).unwrap();
    for key in ["HELLBOMB", "MISSIONS", "SYNC", "ARM"] {
        assert!(view.contains(key), "missing {key}");
    }
}
#[test]
fn safe_default_no_keys() {
    let out = call(&["dry-run", "hellbomb"]);
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8(out.stdout).unwrap().trim(),
        "HOME(tap) DOWN UP LEFT DOWN UP RIGHT DOWN UP"
    );
}
#[test]
fn alt_bindings_hold_ctrl() {
    let out = call(&["dry-run", "hellbomb", "ctrl-wasd"]);
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8(out.stdout).unwrap().trim(),
        "LEFTCTRL(hold) S W A S W D S W LEFTCTRL(release)"
    );
}
