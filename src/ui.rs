use crate::Catalog;
#[derive(Clone, Debug, PartialEq)]
pub enum Tile {
    Blank,
    Label(String),
    Icon(usize),
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Page {
    Home,
    Missions(usize),
    Review,
    Pick(usize, usize),
}
#[derive(Debug, PartialEq)]
pub enum Action {
    None,
    Arm,
    Cast(usize),
    Sync(bool),
    Save([usize; 4]),
    Teach([usize; 4]),
}
pub struct Panel {
    pub slots: [Option<usize>; 4],
    pub draft: [Option<usize>; 4],
    suggestions: [Vec<usize>; 4],
    samples: [Option<crate::hud::Print>; 4],
    picked: [bool; 4],
    pub page: Page,
    pub armed: bool,
    pub waiting_focus: bool,
    pub notice: String,
}
impl Panel {
    pub fn new(slots: [Option<usize>; 4]) -> Self {
        Self {
            slots,
            draft: [None; 4],
            suggestions: Default::default(),
            samples: Default::default(),
            picked: [false; 4],
            page: Page::Home,
            armed: false,
            waiting_focus: false,
            notice: "TAP ARM".into(),
        }
    }
    fn members(&self, c: &Catalog, kind: &str) -> Vec<usize> {
        let mut ids: Vec<_> = c
            .entries
            .iter()
            .enumerate()
            .filter_map(|(i, e)| (e.group == kind).then_some(i))
            .collect();
        ids.sort_by_key(|i| &c.entries[*i].name);
        if let Page::Pick(slot, _) = self.page {
            let priority: Vec<_> = self.slots[slot]
                .into_iter()
                .chain(self.suggestions[slot].iter().copied())
                .collect();
            ids.sort_by_key(|i| priority.iter().position(|id| id == i).unwrap_or(usize::MAX));
        }
        ids
    }
    pub fn review(&mut self, slots: [Option<usize>; 4]) {
        self.suggestions = Default::default();
        self.samples = Default::default();
        self.picked = [false; 4];
        self.draft = slots;
        self.page = Page::Review;
        self.notice.clear();
    }
    pub fn review_scan(&mut self, report: [crate::sync::SlotMatch; 4]) {
        self.review(report.each_ref().map(|s| s.accepted));
        self.samples = report.each_ref().map(|s| s.sample.clone());
        self.suggestions = report.map(|s| {
            s.top
                .into_iter()
                .filter(|(_, score)| *score > 0.)
                .map(|(id, _)| id)
                .collect()
        });
    }
    pub fn confirmed_samples(&self) -> impl Iterator<Item = (usize, &crate::hud::Print)> {
        self.samples.iter().enumerate().filter_map(|(i, p)| {
            if self.page == Page::Review && self.picked[i] {
                Some((self.draft[i]?, p.as_ref()?))
            } else {
                None
            }
        })
    }
    pub fn ready_notice(&mut self) {
        self.notice = if !self.armed {
            "TAP ARM"
        } else if self.waiting_focus {
            "WAIT GAME"
        } else {
            "READY"
        }
        .into();
    }
    pub fn tiles(&self, c: &Catalog) -> [Tile; 15] {
        use Tile::*;
        let mut t = std::array::from_fn(|_| Blank);
        let lab = |s: &str| Label(s.to_owned());
        let nav = |t: &mut [Tile; 15], p: usize, n: usize| {
            if p > 0 {
                t[10] = lab("PREV");
            }
            t[11] = lab(&format!("{}/{}", p + 1, n.div_ceil(10)));
            if (p + 1) * 10 < n {
                t[12] = lab("NEXT");
            }
            t[14] = lab("BACK");
        };
        match self.page {
            Page::Home => {
                for (i, id) in self.slots.iter().enumerate() {
                    t[i] = id
                        .map(Icon)
                        .unwrap_or_else(|| lab(&format!("SLOT {}", i + 1)))
                }
                t[4] = lab("SYNC");
                for (i, id) in ["reinforce", "resupply", "hellbomb", "sos-beacon"]
                    .iter()
                    .enumerate()
                {
                    t[i + 5] = c.id(id).map(Icon).unwrap_or(Blank);
                }
                t[9] = lab("MISSIONS");
                t[10] = lab("EDIT");
                t[11] = lab("SET ROW");
                t[13] = lab(&self.notice);
                t[14] = lab(if self.armed { "PAUSE" } else { "ARM" });
            }
            Page::Missions(p) | Page::Pick(_, p) => {
                let kind = if matches!(self.page, Page::Missions(_)) {
                    "mission"
                } else {
                    "loadout"
                };
                let ids = self.members(c, kind);
                for (i, id) in ids.iter().skip(10 * p).take(10).enumerate() {
                    t[i] = Icon(*id)
                }
                nav(&mut t, p, ids.len());
                if matches!(self.page, Page::Missions(_)) {
                    t[13] = lab(if self.armed { "PAUSE" } else { "ARM" });
                }
                if let Page::Pick(s, _) = self.page {
                    t[13] = lab(&format!("SLOT {}", s + 1));
                }
            }
            Page::Review => {
                for (i, id) in self.draft.iter().enumerate() {
                    t[i] = id
                        .map(Icon)
                        .unwrap_or_else(|| lab(&format!("PICK {}", i + 1)))
                }
                t[4] = lab("EDIT SLOTS");
                if self.partial_ok() {
                    t[11] = lab("USE FOUND");
                }
                if self.complete().is_some() {
                    t[12] = lab("APPLY");
                    if self.confirmed_samples().next().is_some() {
                        t[13] = lab("TEACH");
                    }
                }
                t[14] = lab("CANCEL");
            }
        }
        t
    }
    fn partial_ok(&self) -> bool {
        self.draft.iter().any(Option::is_some)
            && self.draft.iter().any(Option::is_none)
            && self
                .draft
                .iter()
                .enumerate()
                .all(|(i, x)| x.is_none() || !self.draft[..i].contains(x))
    }
    fn complete(&self) -> Option<[usize; 4]> {
        let [Some(a), Some(b), Some(c), Some(d)] = self.draft else {
            return None;
        };
        let v = [a, b, c, d];
        v.iter()
            .enumerate()
            .all(|(i, x)| !v[..i].contains(x))
            .then_some(v)
    }
    pub fn press(&mut self, k: usize, c: &Catalog) -> Action {
        use Action::*;
        if k >= 15 {
            return None;
        }
        let tile = self.tiles(c)[k].clone();
        if !self.armed
            && matches!(self.page, Page::Home | Page::Missions(_))
            && matches!(tile, Tile::Icon(_))
        {
            self.notice = "TAP ARM".into();
            return None;
        }
        match self.page {
            Page::Home => match k {
                4 | 11 => {
                    return Sync(k == 11);
                }
                9 => self.page = Page::Missions(0),
                10 => self.review(self.slots),
                14 => {
                    if self.armed {
                        self.armed = false;
                        self.notice = "PAUSED".into();
                    } else {
                        return Arm;
                    }
                }
                _ => {
                    if self.armed
                        && let Tile::Icon(i) = tile
                    {
                        return Cast(i);
                    }
                }
            },
            Page::Missions(p) => match k {
                13 => {
                    if self.armed {
                        self.armed = false;
                        self.notice = "PAUSED".into();
                    } else {
                        return Arm;
                    }
                }
                14 => self.page = Page::Home,
                10 if p > 0 => self.page = Page::Missions(p - 1),
                12 if tile != Tile::Blank => self.page = Page::Missions(p + 1),
                _ => {
                    if self.armed
                        && let Tile::Icon(i) = tile
                    {
                        return Cast(i);
                    }
                }
            },
            Page::Review => match k {
                0..=3 => self.page = Page::Pick(k, 0),
                13 if tile != Tile::Blank => {
                    if let Some(v) = self.complete() {
                        return Teach(v);
                    }
                }
                11 if self.partial_ok() => {
                    self.slots = self.draft;
                    self.page = Page::Home;
                    self.ready_notice();
                }
                14 => {
                    self.page = Page::Home;
                    self.ready_notice();
                }
                12 => {
                    if let Some(v) = self.complete() {
                        return Save(v);
                    }
                }
                _ => (),
            },
            Page::Pick(s, p) => match k {
                14 => self.page = Page::Review,
                10 if p > 0 => self.page = Page::Pick(s, p - 1),
                12 if tile != Tile::Blank => self.page = Page::Pick(s, p + 1),
                _ => {
                    if let Tile::Icon(i) = tile {
                        self.draft[s] = Some(i);
                        self.picked[s] = true;
                        self.page = Page::Review
                    }
                }
            },
        }
        None
    }
    pub fn saved(&mut self, slots: [usize; 4]) {
        self.slots = slots.map(Some);
        self.page = Page::Home;
        self.ready_notice();
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mission_and_review_never_mutate_live_loadout() {
        let c = Catalog::load().unwrap();
        let ids = [
            "machine-gun",
            "eagle-airstrike",
            "gatling-sentry",
            "orbital-laser",
        ]
        .map(|s| c.id(s));
        let mut p = Panel::new(ids);
        assert_eq!(p.press(7, &c), Action::None);
        p.armed = true;
        assert_eq!(p.press(7, &c), Action::Cast(c.id("hellbomb").unwrap()));
        p.press(9, &c);
        for id in p.members(&c, "mission") {
            assert!(matches!(c.entries[id].group.as_str(), "mission"))
        }
        p.press(14, &c);
        assert_eq!(p.press(4, &c), Action::Sync(false));
        p.review([None, ids[1], ids[2], ids[3]]);
        assert_eq!(p.press(12, &c), Action::None);
        p.press(14, &c);
        assert_eq!(p.slots, ids);
        assert!(
            p.armed,
            "editing and cancelling an armed session should not erase Arm"
        );
        assert_eq!(p.notice, "READY");
    }
}
