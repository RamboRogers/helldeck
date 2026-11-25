use helldeck::{
    Catalog,
    ui::{Action, Page, Panel, Tile},
};
#[test]
fn mission_paging_reaches_every_objective_while_paused_and_armed() {
    let c = Catalog::load().unwrap();
    let mut p = Panel::new([None; 4]);
    p.press(9, &c);
    let mut reached = std::collections::BTreeSet::new();
    loop {
        let ts = p.tiles(&c);
        for (k, t) in ts[..10].iter().enumerate() {
            if let Tile::Icon(i) = t {
                assert_eq!(p.press(k, &c), Action::None);
                p.armed = true;
                assert_eq!(p.press(k, &c), Action::Cast(*i));
                p.armed = false;
                reached.insert(c.entries[*i].id.clone());
            }
        }
        if ts[12] == Tile::Blank {
            break;
        }
        p.press(12, &c);
    }
    assert_eq!(
        reached,
        c.entries
            .iter()
            .filter(|e| e.group == "mission")
            .map(|e| e.id.clone())
            .collect()
    );
    p.press(14, &c);
    assert_eq!(p.page, Page::Home);
}
#[test]
fn teaching_requires_an_explicit_pick_from_a_fresh_scan() {
    let c = Catalog::load().unwrap();
    let ids = [
        "shield-generator-pack",
        "las-44-nova",
        "rocket-sentry",
        "leveller",
    ]
    .map(|s| c.id(s).unwrap());
    let mut row = image::RgbImage::new(288, 72);
    for (slot, &id) in ids.iter().enumerate() {
        image::imageops::replace(&mut row, &c.icons[id], (slot * 72) as i64, 0);
    }
    let mut scan = helldeck::sync::Matcher::new(&c).scan(&row).unwrap();
    scan[1].accepted = None;
    let mut p = Panel::new(ids.map(Some));
    p.review_scan(scan);
    assert_eq!(p.tiles(&c)[13], Tile::Blank);
    p.press(1, &c);
    p.press(0, &c); // explicitly pick previous Nova
    assert_eq!(p.tiles(&c)[13], Tile::Label("TEACH".into()));
    assert_eq!(
        p.confirmed_samples().map(|(id, _)| id).collect::<Vec<_>>(),
        vec![ids[1]]
    );
    assert_eq!(p.press(12, &c), Action::Save(ids));
    assert_eq!(p.press(13, &c), Action::Teach(ids));
    p.press(14, &c);
    p.press(10, &c);
    p.press(1, &c);
    p.press(0, &c); // manual Edit is not a camera sample
    assert_eq!(p.press(13, &c), Action::None);
}
#[test]
fn previous_slot_is_a_first_page_choice_not_an_automatic_match() {
    let c = Catalog::load().unwrap();
    let nova = c.id("las-44-nova").unwrap();
    let mut p = Panel::new([None, Some(nova), None, None]);
    p.review([None; 4]);
    p.press(1, &c);
    assert_eq!(p.tiles(&c)[0], Tile::Icon(nova));
    assert_eq!(p.draft, [None; 4]);
    assert_eq!(p.press(0, &c), Action::None);
    assert_eq!(p.draft[1], Some(nova));
}
#[test]
fn corrected_draft_cannot_apply_duplicates_or_replace_utility() {
    let c = Catalog::load().unwrap();
    let ids = [
        "eagle-airstrike",
        "machine-gun",
        "gatling-sentry",
        "orbital-laser",
    ]
    .map(|s| c.id(s).unwrap());
    let mut p = Panel::new([None; 4]);
    let fixed = p.tiles(&c)[5..10].to_vec();
    p.review([Some(ids[0]), Some(ids[0]), Some(ids[2]), Some(ids[3])]);
    assert_eq!(p.press(12, &c), Action::None);
    p.press(1, &c);
    loop {
        let t = p.tiles(&c);
        if let Some(pos) = t[..10].iter().position(|x| *x == Tile::Icon(ids[1])) {
            p.press(pos, &c);
            break;
        }
        assert_ne!(t[12], Tile::Blank);
        p.press(12, &c);
    }
    assert_eq!(p.press(12, &c), Action::Save(ids));
    assert_eq!(p.slots, [None; 4]);
    p.saved(ids);
    assert_eq!(p.slots, ids.map(Some));
    assert_eq!(p.tiles(&c)[5..10], fixed);
    assert!(!p.armed);
}
