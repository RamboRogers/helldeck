use helldeck::{
    Catalog,
    ui::{Action, Page, Panel, Tile},
};
#[test]
fn paused_press_explains_the_next_step_and_sends_nothing() {
    let c = Catalog::load().unwrap();
    let mut p = Panel::new([None; 4]);
    assert_eq!(p.press(6, &c), Action::None);
    assert_eq!(p.tiles(&c)[13], Tile::Label("TAP ARM".into()));
}
#[test]
fn reviewed_partial_can_be_used_without_inheriting_stale_slots() {
    let c = Catalog::load().unwrap();
    let id = c.id("supply-pack").unwrap();
    let mut p = Panel::new([Some(id); 4]);
    p.review([None, Some(id), None, None]);
    assert_eq!(p.tiles(&c)[11], Tile::Label("USE FOUND".into()));
    assert_eq!(p.press(11, &c), Action::None);
    assert_eq!(p.page, Page::Home);
    assert_eq!(p.slots, [None, Some(id), None, None]);
    assert!(!p.armed);
    assert_eq!(p.press(14, &c), Action::Arm);
    p.armed = true;
    assert_eq!(p.press(0, &c), Action::None);
    assert_eq!(p.press(1, &c), Action::Cast(id));
}
