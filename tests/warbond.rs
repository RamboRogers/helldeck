use helldeck::{Catalog, sync::Matcher};
#[test]
fn warbond_gaps_have_icons_and_verified_sequences() {
    let cat = Catalog::load().unwrap();
    for (id, code) in [
        ("amw-20-apex", "DLLUDL"),
        ("las-44-nova", "DLRDD"),
        ("breaching-hammer", "DLRLU"),
        ("leveller", "DDLUD"),
        ("belt-fed-grenade-launcher", "DLULUU"),
        ("gas-mortar-sentry", "DURDL"),
        ("flame-sentry", "DURDUU"),
    ] {
        let ix = cat.id(id).unwrap_or_else(|| panic!("missing {id}"));
        assert_eq!(cat.entries[ix].group, "loadout");
        assert_eq!(cat.entries[ix].code, code);
    }
}
#[test]
fn synced_row_with_scope_of_authority_is_actionable() {
    let c = Catalog::load().unwrap();
    let ids = [
        "amw-20-apex",
        "las-44-nova",
        "supply-pack",
        "machine-gun-sentry",
    ]
    .map(|id| c.id(id).expect(id));
    let mut row = image::RgbImage::new(452, 112);
    for (i, ix) in ids.iter().enumerate() {
        let t =
            image::imageops::resize(&c.icons[*ix], 89, 89, image::imageops::FilterType::Triangle);
        image::imageops::replace(&mut row, &t, (113 * i + 12) as i64, 11);
    }
    assert_eq!(Matcher::new(&c).recognize(&row).unwrap(), ids.map(Some));
}
