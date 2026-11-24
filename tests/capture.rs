//! Exercise the real capture pipeline with isolated stand-ins for compositor I/O,
//! not a user's desktop or Steam session. The generated row uses embedded art.
use helldeck::{Catalog, platform::capture, sync::Matcher};
use std::{fs, os::unix::fs::PermissionsExt, process::Command};
#[test]
fn capture_contract() {
    if let Ok(mode) = std::env::var("HELLDECK_CAPTURE_CHILD") {
        let result = capture(None, false);
        if mode == "ok" {
            let (r, img) = result.unwrap();
            assert_eq!((r.x, r.y, r.w, r.h), (100, 100, 288, 72));
            let c = Catalog::load().unwrap();
            let matches = Matcher::new(&c).recognize(&img).unwrap();
            assert_eq!(
                matches,
                [
                    "machine-gun",
                    "eagle-airstrike",
                    "gatling-sentry",
                    "orbital-laser"
                ]
                .map(|id| c.id(id))
            );
        } else {
            assert!(result.is_err())
        }
        return;
    }
    let c = Catalog::load().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let mut im = image::RgbImage::new(288, 72);
    for (i, id) in [
        "machine-gun",
        "eagle-airstrike",
        "gatling-sentry",
        "orbital-laser",
    ]
    .iter()
    .enumerate()
    {
        image::imageops::replace(&mut im, &c.icons[c.id(id).unwrap()], (i * 72) as i64, 0)
    }
    im.save(root.join("fixture.png")).unwrap();
    let script = |name: &str, body: &str| {
        let path = root.join(name);
        fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
    };
    script(
        "hyprctl",
        r#"class=steam_app_553850
if [ "$HELLDECK_CAPTURE_CHILD" = outside ] || { [ "$HELLDECK_CAPTURE_CHILD" = changed ] && [ -f grabbed ]; }; then class=terminal; fi
printf '{"class":"%s","address":"0x1","pid":11,"mapped":true,"at":[-1920,0],"size":[1920,1080]}' "$class""#,
    );
    script(
        "slurp",
        "printf 'selected' > selector; printf '%s' '-1820 100 288 72'",
    );
    script(
        "grim",
        r#"[ "$1/$2/$3/$4/$5" = '-s/1/-g/-1820,100 288x72/-' ] || exit 2
: > grabbed
exec /usr/bin/cat fixture.png"#,
    );
    for mode in ["outside", "ok", "changed"] {
        for f in ["selector", "grabbed"] {
            let _ = fs::remove_file(root.join(f));
        }
        let out = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "capture_contract", "--nocapture"])
            .current_dir(root)
            .env("HELLDECK_CAPTURE_CHILD", mode)
            .env("PATH", format!("{}:/usr/bin", root.display()))
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(root.join("grabbed").exists(), mode != "outside");
        assert_eq!(root.join("selector").exists(), mode != "outside");
    }
}
