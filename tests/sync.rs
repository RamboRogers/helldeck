use helldeck::Catalog;
use image::{Rgb, RgbImage};
use std::process::Command;
fn invoke(img: RgbImage) -> std::process::Output {
    let config = tempfile::tempdir().unwrap();
    invoke_in(img, config.path())
}
fn invoke_in(img: RgbImage, config: &std::path::Path) -> std::process::Output {
    let path = std::env::temp_dir().join(format!(
        "helldeck-sync-test-{}-{:?}.png",
        std::process::id(),
        std::thread::current().id()
    ));
    img.save(&path).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_helldeck"))
        .env("XDG_CONFIG_HOME", config)
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
fn square_slot_frames_are_not_part_of_the_glyph() {
    let ids = [
        "machine-gun",
        "eagle-airstrike",
        "gatling-sentry",
        "orbital-laser",
    ];
    let c = Catalog::load().unwrap();
    let m = helldeck::sync::Matcher::new(&c);
    let mut img = row(ids);
    // The border is inset from the calibration box, with a different width per slot.
    for i in 0..4 {
        for y in 8..80u32 {
            for x in 7..79u32 {
                let inset = (x - 7).min(78 - x).min(y - 8).min(79 - y);
                if inset < 3 + i {
                    // Some of the inner lip meets same-colored icon pixels.
                    let color = if inset == 2 + i && !(30..41).contains(&y) {
                        [61, 134, 150]
                    } else {
                        [85, 185, 210]
                    };
                    img.put_pixel(i * 88 + x, y, Rgb(color));
                }
            }
        }
    }
    assert_eq!(m.recognize(&img).unwrap(), ids.map(|id| c.id(id)));
    for y in 11..77 {
        for x in 10..76 {
            img.put_pixel(x, y, Rgb([12, 15, 18]));
        }
    }
    assert_eq!(
        m.recognize(&img).unwrap()[0],
        None,
        "a frame alone is not an icon"
    );
}
#[test]
fn similar_shields_are_separated_by_their_markings() {
    let ids = [
        "shield-generator-pack",
        "ballistic-shield-backpack",
        "directional-shield",
        "leveller",
    ];
    let out = invoke(row(ids));
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        ids.join(" "),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
#[test]
fn small_hud_icons_survive_padding_but_isolated_specks_do_not() {
    let c = Catalog::load().unwrap();
    let m = helldeck::sync::Matcher::new(&c);
    let ids = [
        "gunner-frv",
        "supply-frv",
        "incinerator-frv",
        "40-k-meltagun",
    ];
    let mut img = RgbImage::from_pixel(448, 112, Rgb([24, 30, 39]));
    for (j, id) in ids.iter().enumerate() {
        let tiny = image::imageops::resize(
            &c.icons[c.id(id).unwrap()],
            30,
            30,
            image::imageops::FilterType::Triangle,
        );
        image::imageops::replace(&mut img, &tiny, (112 * j + 34) as i64, 31);
    }
    assert_eq!(m.recognize(&img).unwrap(), ids.map(|id| c.id(id)));
    let mut noise = RgbImage::new(112, 112);
    for i in 0..16 {
        noise.put_pixel(44 + i, 43 + i, Rgb([237, 237, 237]));
        noise.put_pixel(60 - i, 43 + i, Rgb([69, 172, 200]));
    }
    image::imageops::replace(&mut img, &noise, 0, 0);
    assert_eq!(m.recognize(&img).unwrap()[0], None);
}
#[test]
fn retains_color_evidence_when_binary_silhouettes_are_identical() {
    let mut c = Catalog::load().unwrap();
    let a = c.id("rocket-sentry").unwrap();
    let b = c.id("flame-sentry").unwrap();
    let mut alternate = c.icons[a].clone();
    for p in alternate.pixels_mut() {
        p.0.swap(0, 1);
    }
    c.icons[b] = alternate;
    let ids = [
        b,
        a,
        c.id("machine-gun").unwrap(),
        c.id("orbital-laser").unwrap(),
    ];
    let mut r = RgbImage::from_pixel(352, 88, Rgb([25, 31, 39]));
    for (slot, id) in ids.iter().enumerate() {
        let t =
            image::imageops::resize(&c.icons[*id], 65, 65, image::imageops::FilterType::Triangle);
        image::imageops::replace(&mut r, &t, slot as i64 * 88 + 9, 12);
    }
    assert_eq!(
        helldeck::sync::Matcher::new(&c).recognize(&r).unwrap(),
        ids.map(Some)
    );
}
#[test]
fn sync_takes_the_highest_scoring_shield_in_a_close_race() {
    let c = Catalog::load().unwrap();
    let ids = [
        "eagle-gas-airstrike",
        "las-44-nova",
        "shield-generator-pack",
        "expendable-anti-tank",
    ];
    let mut img = row(ids);
    let shields = row([
        "shield-generator-pack",
        "ballistic-shield-backpack",
        "directional-shield",
        "leveller",
    ]);
    // Make the markings hard to separate without duplicating either shield in the row.
    for y in 0..88 {
        for x in 0..88 {
            let a = shields.get_pixel(x, y).0;
            let b = shields.get_pixel(x + 88, y).0;
            img.put_pixel(
                176 + x,
                y,
                Rgb(std::array::from_fn(|k| {
                    ((a[k] as u16 + b[k] as u16) / 2) as u8
                })),
            );
        }
    }
    let matches = helldeck::sync::Matcher::new(&c).scan(&img).unwrap();
    let slot = &matches[2];
    let [(winner, first), (_, second), ..] = slot.top[..] else {
        panic!("missing scores");
    };
    assert!(first > second && first - second < 0.065, "{:?}", slot.top);
    assert_eq!(Some(winner), c.id("shield-generator-pack"));
    assert_eq!(matches.map(|s| s.accepted), ids.map(|id| c.id(id)));
}
#[test]
fn local_native_template_matches_a_rescaled_icon_without_changing_deck_art() {
    let c = Catalog::load().unwrap();
    let ids = ["quasar-cannon", "las-44-nova", "rocket-sentry", "leveller"];
    let mut img = row(ids);
    // Stand-in for a differently drawn game asset; no extracted game art in git.
    let nova = image::imageops::flip_horizontal(&c.icons[c.id(ids[1]).unwrap()]);
    let scaled = image::imageops::resize(&nova, 65, 65, image::imageops::FilterType::Triangle);
    image::imageops::replace(&mut img, &scaled, 99, 11);
    let profile = tempfile::tempdir().unwrap();
    let without = invoke_in(img.clone(), profile.path());
    assert!(
        String::from_utf8_lossy(&without.stderr).contains("Sync slot 2: accepted (highest score);")
    );
    let dir = profile.path().join("helldeck/native-icons");
    std::fs::create_dir_all(&dir).unwrap();
    nova.save(dir.join("las-44-nova.png")).unwrap();
    std::fs::write(dir.join("eagle-airstrike.png"), b"incomplete PNG").unwrap();
    let out = invoke_in(img.clone(), profile.path());
    assert!(String::from_utf8_lossy(&out.stderr).contains("Sync slot 2: accepted; las-44-nova="));
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        ids.join(" "),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    nova.save(dir.join("quasar-cannon.png")).unwrap();
    let conflict = invoke_in(img, profile.path());
    assert!(!conflict.status.success());
    assert!(String::from_utf8_lossy(&conflict.stderr).contains("Sync slot 2: ambiguous"));
}
#[test]
fn native_dictionary_and_hud_texture_banks_coexist() {
    let c = Catalog::load().unwrap();
    let ids = [
        "shield-generator-pack",
        "las-44-nova",
        "rocket-sentry",
        "leveller",
    ];
    let mut img = row(ids);
    let profile = tempfile::tempdir().unwrap();
    let dir = profile.path().join("helldeck/native-icons");
    std::fs::create_dir_all(dir.join("hud")).unwrap();
    for (slot, bank) in [(1usize, dir.clone()), (2, dir.join("hud"))] {
        let icon = image::imageops::flip_horizontal(&c.icons[c.id(ids[slot]).unwrap()]);
        icon.save(bank.join(format!("{}.png", ids[slot]))).unwrap();
        let scaled = image::imageops::resize(&icon, 65, 65, image::imageops::FilterType::Triangle);
        image::imageops::replace(&mut img, &scaled, (slot * 88 + 11) as i64, 11);
    }
    let out = invoke_in(img, profile.path());
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        ids.join(" "),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
#[test]
fn distinct_vehicle_markings_select_different_actions() {
    let ids = [
        "gunner-frv",
        "supply-frv",
        "incinerator-frv",
        "40-k-meltagun",
    ];
    let out = invoke(row(ids));
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        ids.join(" "),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
#[test]
fn truly_identical_vehicle_references_require_explicit_choice_even_after_teach() {
    use helldeck::{
        hud::HudBook,
        sync::Matcher,
        ui::{Panel, Tile},
    };
    let mut c = Catalog::load().unwrap();
    let ids = ["supply-frv", "rocket-sentry", "leveller", "las-44-nova"];
    let img = row(ids);
    let gunner = c.id("gunner-frv").unwrap();
    c.icons[gunner] = c.icons[c.id("supply-frv").unwrap()].clone();
    let m = Matcher::new(&c);
    let print =
        helldeck::hud::Print::extract(&image::imageops::crop_imm(&img, 0, 0, 88, 88).to_image())
            .unwrap();
    let mut book = HudBook::default();
    book.remember(ids[0], print);
    let scan = m.with_hud(&c, &book).scan(&img).unwrap();
    assert_eq!(scan[0].accepted, None);
    assert!(scan[0].sample.is_none());
    let mut panel = Panel::new([None; 4]);
    panel.review_scan(scan);
    panel.press(0, &c);
    let choices = panel.tiles(&c);
    for id in ["supply-frv", "gunner-frv"] {
        assert!(choices[..2].contains(&Tile::Icon(c.id(id).unwrap())));
    }
}
#[test]
fn corrected_hud_art_is_recognized_on_a_separate_exposure() {
    // Deliberately different HUD artwork, not a reconstruction of the user's screenshot.
    let c = Catalog::load().unwrap();
    let mut observed = row([
        "shield-generator-pack",
        "las-44-nova",
        "rocket-sentry",
        "leveller",
    ]);
    for y in 12..46 {
        for x in 12..44 {
            for start in [88, 264] {
                let a = *observed.get_pixel(start + x, y);
                let b = *observed.get_pixel(start + 87 - x, y);
                observed.put_pixel(start + x, y, b);
                observed.put_pixel(start + 87 - x, y, a);
            }
        }
    }
    let mut held_out = observed.clone();
    held_out
        .iter_mut()
        .for_each(|v| *v = (*v as f32 * 0.88) as u8);
    let matcher = helldeck::sync::Matcher::new(&c);
    let scan = matcher.scan(&observed).unwrap();
    for slot in [&scan[1], &matcher.scan(&held_out).unwrap()[1]] {
        assert_eq!(slot.reason, "accepted (highest score)");
        assert_eq!(slot.accepted, Some(slot.top[0].0));
    }
    let mut book = helldeck::hud::HudBook::default();
    let sample = scan[1].sample.clone().unwrap();
    book.remember("las-44-nova", sample.clone());
    book.remember("leveller", scan[3].sample.clone().unwrap());
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("hud-v1.json");
    book.save(&path).unwrap();
    let reloaded = helldeck::hud::HudBook::load(&path).unwrap();
    let report = matcher.with_hud(&c, &reloaded).scan(&held_out).unwrap();
    assert_eq!(report[1].accepted, c.id("las-44-nova"));
    assert_eq!(report[1].reason, "confirmed HUD");
    assert_eq!(report[3].accepted, c.id("leveller"));
    assert_eq!(report[3].reason, "confirmed HUD");
    book.remember("quasar-cannon", sample); // conflicting labels never break ties by insertion order
    assert_eq!(
        helldeck::sync::Matcher::new(&c)
            .with_hud(&c, &book)
            .scan(&held_out)
            .unwrap()[1]
            .accepted,
        None
    );
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
fn featureless_slot_cannot_become_a_match_after_trimming() {
    let cat = Catalog::load().unwrap();
    let mut img = RgbImage::from_pixel(400, 100, Rgb([128; 3]));
    for (i, id) in ["las-44-nova", "rocket-sentry", "leveller"]
        .iter()
        .enumerate()
    {
        image::imageops::replace(
            &mut img,
            &cat.icons[cat.id(id).unwrap()],
            (114 + 100 * i) as i64,
            14,
        );
    }
    let scan = helldeck::sync::Matcher::new(&cat).scan(&img).unwrap();
    assert_eq!(scan[0].accepted, None, "{}", scan[0].reason);
    assert_eq!(scan[0].reason, "no glyph");
}
#[test]
fn decisive_appearance_outside_the_binary_shortlist_blocks_a_wrong_pick() {
    let cat = Catalog::load().unwrap();
    let mut img = RgbImage::new(528, 132);
    let ids = ["las-44-nova", "rocket-sentry", "leveller", "hover-pack"];
    for (i, id) in ids.iter().enumerate() {
        let start = i as u32 * 132;
        image::imageops::replace(
            &mut img,
            &cat.icons[cat.id(id).unwrap()],
            i64::from(start) + 30,
            30,
        );
        for n in [18, 113] {
            img.put_pixel(start + n, n, Rgb([150; 3]));
        }
    }
    let scan = helldeck::sync::Matcher::new(&cat).scan(&img).unwrap();
    assert_eq!(scan[0].accepted, None, "{}", scan[0].reason);
    assert_eq!(scan[0].reason, "conflicting visual evidence");
    for (slot, id) in scan.iter().zip(ids) {
        assert!(
            slot.accepted.is_none() || slot.accepted == cat.id(id),
            "wrong pick for {id}"
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
