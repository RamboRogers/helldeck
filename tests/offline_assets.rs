use helldeck::Catalog;
use std::collections::HashSet;

#[test]
fn every_catalog_entry_has_a_unique_id_and_visible_local_art() {
    let catalog = Catalog::load().unwrap();
    assert_eq!(catalog.icons.len(), catalog.entries.len());
    let mut ids = HashSet::new();
    for (entry, tile) in catalog.entries.iter().zip(&catalog.icons) {
        assert!(ids.insert(&entry.id), "duplicate ID: {}", entry.id);
        assert_eq!(tile.dimensions(), (72, 72), "{}", entry.id);
        assert!(
            tile.pixels().any(|pixel| pixel != tile.get_pixel(0, 0)),
            "blank icon: {}",
            entry.id
        );
    }
}
