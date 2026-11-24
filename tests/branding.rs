use std::process::Command;
#[test]
fn about_and_help_credit_the_publisher_without_opening_a_deck() {
    for arg in ["about", "--help"] {
        let res = Command::new(env!("CARGO_BIN_EXE_helldeck"))
            .arg(arg)
            .output()
            .unwrap();
        assert!(
            res.status.success(),
            "{arg}: {}",
            String::from_utf8_lossy(&res.stderr)
        );
        let out = String::from_utf8(res.stdout).unwrap();
        for expected in ["HellDeck", "Matthew Rogers", "@matthewrogers"] {
            assert!(out.contains(expected), "{arg} missing {expected}");
        }
    }
}
