use std::process::Command;
fn run(args: &[&str]) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_helldeck"))
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}
#[test]
fn mission_catalog_includes_verified_codes() {
    let s = run(&["catalog"]);
    for (id, code) in [
        ("hellbomb", "DULDURDU"),
        ("seismic-probe", "UULRDD"),
        ("super-earth-flag", "DUDU"),
        ("upload-data", "LRUUU"),
    ] {
        assert!(
            s.lines()
                .any(|l| l.starts_with(&format!("{id}\tmission\t{code}\t"))),
            "missing {id}"
        );
    }
}
