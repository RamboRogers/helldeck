use helldeck::Catalog;
use image::{Rgb, RgbImage};
use std::process::Command;
fn invoke(img: RgbImage) -> std::process::Output {
    let path = std::env::temp_dir().join(format!(
        "helldeck-sync-test-{}-{:?}.png",
        std::process::id(),
        std::thread::current().id()
    ));
    img.save(&path).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_helldeck"))
        .arg("recognize")
        .arg(&path)
        .output()
        .unwrap();
    std::fs::remove_file(path).unwrap();
    out
}
fn row(ids: [&str; 4]) -> RgbImage {
    let cat = Catalog::load().unwrap();
    let mut row = RgbImage::from_pixel(352, 88, Rgb([25, 31, 39]));
    for (i, id) in ids.iter().enumerate() {
        if let Some(ix) = cat.id(id) {
            let icon = image::imageops::resize(
                &cat.icons[ix],
                65,
                65,
                image::imageops::FilterType::Triangle,
            );
            image::imageops::replace(&mut row, &icon, (i * 88 + 11) as i64, 11);
        }
    }
    row
}
#[test]
fn sync_recognizes_scaled_padded_row_in_order() {
    let ids = [
        "machine-gun",
        "eagle-airstrike",
        "gatling-sentry",
        "orbital-laser",
    ];
    let out = invoke(row(ids));
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(String::from_utf8(out.stdout).unwrap().trim(), ids.join(" "));
}
#[test]
fn nova_in_slot_two_survives_loadout_dimming_and_blur() {
    let ids = [
        "orbital-napalm-barrage",
        "las-44-nova",
        "jump-pack",
        "flame-sentry",
    ];
    for (light, blur) in [(0.65, 0.), (1., 1.), (0.72, 1.)] {
        let mut img = row(ids);
        let mut nova = image::imageops::crop_imm(&img, 88, 0, 88, 88).to_image();
        if blur > 0. {
            nova = image::imageops::blur(&nova, blur);
        }
        nova.iter_mut().for_each(|v| *v = (*v as f32 * light) as u8);
        image::imageops::replace(&mut img, &nova, 88, 0);
        let out = invoke(img);
        assert_eq!(
            String::from_utf8_lossy(&out.stdout).trim(),
            ids.join(" "),
            "light={light}, blur={blur}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}
#[test]
fn sync_refuses_partial_or_duplicate_rows() {
    for ids in [
        ["machine-gun", "", "gatling-sentry", "orbital-laser"],
        [
            "machine-gun",
            "machine-gun",
            "gatling-sentry",
            "orbital-laser",
        ],
    ] {
        let out = invoke(row(ids));
        assert!(!out.status.success());
        assert!(String::from_utf8_lossy(&out.stderr).contains("review"));
        let log = String::from_utf8_lossy(&out.stderr);
        if ids[1].is_empty() {
            assert!(log.contains("slot 2: no glyph"), "{log}");
        } else {
            assert!(
                log.contains("slot 2: duplicate") && log.contains("machine-gun="),
                "{log}"
            );
        }
    }
}
