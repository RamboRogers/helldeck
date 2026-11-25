//! Recognize four equally spaced cells in a user-calibrated strip. No screen data leaves the process.
mod appearance;
mod frame;
mod native;
use crate::Catalog;
use anyhow::{Result, ensure};
use image::{
    GenericImageView, GrayImage, Luma, RgbImage,
    imageops::{self, FilterType},
};
struct Glyph {
    body: Vec<bool>,
    holes: Vec<bool>,
}

// The bolt and bars cut out of the shields are tiny compared with the shared
// backpack silhouette. Compare enclosed negative space independently of area.
fn holes(light: &GrayImage) -> Vec<bool> {
    let (w, h) = light.dimensions();
    let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0, 0);
    for (x, y, p) in light.enumerate_pixels() {
        if p[0] != 0 {
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x);
            y1 = y1.max(y);
        }
    }
    if x1 <= x0 || y1 <= y0 {
        return vec![];
    }
    let mask = imageops::resize(
        &light.view(x0, y0, x1 - x0 + 1, y1 - y0 + 1).to_image(),
        32,
        32,
        FilterType::Triangle,
    );
    let mut dark: Vec<bool> = mask.into_raw().iter().map(|&v| v < 128).collect();
    let mut todo: Vec<usize> = (0..1024)
        .filter(|&i| !(32..992).contains(&i) || i % 32 == 0 || i % 32 == 31)
        .collect();
    while let Some(i) = todo.pop() {
        if !dark[i] {
            continue;
        }
        dark[i] = false;
        if i >= 32 {
            todo.push(i - 32);
        }
        if i < 992 {
            todo.push(i + 32);
        }
        if i % 32 > 0 {
            todo.push(i - 1);
        }
        if i % 32 < 31 {
            todo.push(i + 1);
        }
    }
    if dark.iter().filter(|&&v| v).count() < 8 {
        return vec![];
    }
    dark
}
fn ink(rgb: [u8; 3]) -> (bool, bool) {
    let lo = rgb[0].min(rgb[1]).min(rgb[2]);
    let hi = rgb[0].max(rgb[1]).max(rgb[2]);
    (lo >= 145 && hi - lo <= 65, hi >= 130 && hi - lo >= 48)
}

// Register a small two-ink HUD glyph *before* thresholding its antialiased
// edges. Padding belongs to the cell, not to the observed icon's pixel size.
fn small_view(tile: &RgbImage) -> Option<RgbImage> {
    let (w, h) = tile.dimensions();
    let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0, 0);
    let (mut light, mut color) = (0, 0);
    for (x, y, p) in tile.enumerate_pixels() {
        let (l, c) = ink(p.0);
        if l || c {
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x);
            y1 = y1.max(y);
            light += u32::from(l);
            color += u32::from(c);
        }
    }
    if light.min(color) < 12 || light + color < 80 || x0 < 2 || y0 < 2 || x1 + 2 >= w || y1 + 2 >= h
    {
        return None;
    }
    let (gw, gh) = (x1 - x0 + 5, y1 - y0 + 5);
    if gw.min(gh) < 12 || gw.max(gh) >= 68 {
        return None;
    }
    let scaled = |v| (v * 68 + gw.max(gh) / 2) / gw.max(gh);
    let glyph = imageops::resize(
        &tile.view(x0 - 2, y0 - 2, gw, gh).to_image(),
        scaled(gw),
        scaled(gh),
        FilterType::Triangle,
    );
    let mut canvas = RgbImage::new(100, 100);
    imageops::replace(
        &mut canvas,
        &glyph,
        i64::from((100 - glyph.width()) / 2),
        i64::from((100 - glyph.height()) / 2),
    );
    Some(canvas)
}

fn glyph(tile: &RgbImage, trim: u32) -> Option<Glyph> {
    let (w, h) = tile.dimensions();
    let (dx, dy) = (w * trim / 100, h * trim / 100);
    let mut mask = GrayImage::new(w, h);
    let mut light = GrayImage::new(w, h);
    let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0, 0);
    let mut n = 0;
    for y in dy..h - dy {
        for x in dx..w - dx {
            let (white, color) = ink(tile.get_pixel(x, y).0);
            if white || color {
                mask.put_pixel(x, y, Luma([255]));
                if white {
                    light.put_pixel(x, y, Luma([255]));
                }
                x0 = x0.min(x);
                x1 = x1.max(x);
                y0 = y0.min(y);
                y1 = y1.max(y);
                n += 1;
            }
        }
    }
    // Pixels excluded by the trim are not evidence of a background. A
    // uniformly lit cell otherwise becomes a solid rectangular "glyph".
    let inspected = (w - 2 * dx) * (h - 2 * dy);
    if n < w * h / 55
        || n > inspected * 3 / 5
        || x1.saturating_sub(x0) < 4
        || y1.saturating_sub(y0) < 4
    {
        return None;
    }
    let glyph = mask.view(x0, y0, x1 - x0 + 1, y1 - y0 + 1).to_image();
    let detail = light.view(x0, y0, x1 - x0 + 1, y1 - y0 + 1).to_image();
    let cutouts = holes(&detail);
    Some(Glyph {
        holes: cutouts,
        body: [glyph, detail]
            .iter()
            .flat_map(|m| imageops::resize(m, 32, 32, FilterType::Triangle).into_raw())
            .map(|x| x >= 128)
            .collect(),
    })
}
fn score(a: &[bool], b: &[bool]) -> f32 {
    let (mut both, mut sum) = (0, 0);
    for (&x, &y) in a.iter().zip(b) {
        both += u32::from(x && y);
        sum += u32::from(x) + u32::from(y);
    }
    2. * both as f32 / sum.max(1) as f32
}
// Normalize only usable highlights. A few bright HUD pixels should not set
// exposure for the whole cell; an empty dark cell is not a faint icon.
fn exposure(tile: &RgbImage) -> Option<RgbImage> {
    let mut hist = [0u32; 256];
    for p in tile.pixels() {
        hist[usize::from(*p.0.iter().max().unwrap())] += 1;
    }
    let count = tile.width() * tile.height();
    let mut n = 0;
    let peak = (0..256).rev().find(|&v| {
        n += hist[v];
        n >= count.div_ceil(100)
    })?;
    if !(85..245).contains(&peak) {
        return None;
    }
    let mut out = tile.clone();
    for v in out.iter_mut() {
        *v = (usize::from(*v) * 255 / peak).min(255) as u8;
    }
    Some(out)
}
#[derive(Default)]
pub struct SlotMatch {
    pub sample: Option<crate::hud::Print>,
    hud_best: Option<(usize, f32)>,

    pub accepted: Option<usize>,
    pub top: Vec<(usize, f32)>,
    pub reason: &'static str,
}
pub struct Matcher {
    templates: Vec<(usize, Vec<Glyph>)>,
    appearance: Vec<Vec<appearance::Appearance>>,
    hud: Vec<(usize, Vec<crate::hud::Print>)>,
}
impl SlotMatch {
    pub fn log(&self, slot: usize, cat: &Catalog) {
        let names = self
            .top
            .iter()
            .map(|(i, score)| format!("{}={score:.3}", cat.entries[*i].id))
            .collect::<Vec<_>>()
            .join(" ");
        eprintln!("Sync slot {}: {}; {}", slot + 1, self.reason, names);
        if let Some((id, score)) = self.hud_best {
            eprintln!(
                "Sync slot {} HUD reference: {}={score:.3}",
                slot + 1,
                cat.entries[id].id
            );
        }
    }
}
impl Matcher {
    pub fn new(cat: &Catalog) -> Self {
        Self {
            templates: cat
                .icons
                .iter()
                .enumerate()
                .filter(|(i, _)| cat.entries[*i].group == "loadout")
                .filter_map(|(i, t)| {
                    let mut refs = vec![glyph(t, 1)?];
                    refs.extend(glyph(&imageops::blur(t, 1.1), 1));
                    Some((i, refs))
                })
                .collect(),
            appearance: cat
                .icons
                .iter()
                .enumerate()
                .map(|(i, t)| {
                    if cat.entries[i].group == "loadout" {
                        appearance::Appearance::extract(t).into_iter().collect()
                    } else {
                        Vec::new()
                    }
                })
                .collect(),
            hud: vec![],
        }
    }
    pub fn with_hud(mut self, c: &Catalog, book: &crate::hud::HudBook) -> Self {
        self.hud = book.indexed(c);
        self
    }
    pub fn local(c: &Catalog) -> Self {
        let mut m = Self::new(c);
        if let Ok(config) = crate::platform::config_path() {
            let dir = config.with_file_name("native-icons");
            let mut count = 0;
            let mut covered = 0;
            for (i, refs) in &mut m.templates {
                let start = count;
                let id = &c.entries[*i].id;
                let file = format!("{id}.png");
                for path in [dir.join(&file), dir.join("hud").join(&file)] {
                    match native::load(&path) {
                        Ok(Some(tile)) => {
                            if let Some(a) = appearance::Appearance::extract(&tile) {
                                m.appearance[*i].push(a);
                            }
                            if let Some(g) = glyph(&tile, 1) {
                                refs.push(g);
                                refs.extend(glyph(&imageops::blur(&tile, 1.1), 1));
                                count += 1;
                            }
                        }
                        Ok(None) => (),
                        Err(e) => eprintln!("Ignoring native icon {}: {e:#}", path.display()),
                    }
                }
                covered += usize::from(count != start);
            }
            if count > 0 {
                eprintln!(
                    "Sync loaded {count} local native icon templates for {covered} loadout IDs"
                );
            }
        }
        match crate::hud::HudBook::path().and_then(|p| crate::hud::HudBook::load(&p)) {
            Ok(book) => m.with_hud(c, &book),
            Err(e) => {
                eprintln!("Ignoring unreadable local HUD references: {e:#}");
                m
            }
        }
    }
    pub fn recognize(&self, row: &RgbImage) -> Result<[Option<usize>; 4]> {
        Ok(self.scan(row)?.map(|s| s.accepted))
    }
    pub fn scan(&self, row: &RgbImage) -> Result<[SlotMatch; 4]> {
        let (w, h) = row.dimensions();
        ensure!(
            (64..=4096).contains(&w)
                && (16..=1024).contains(&h)
                && (3. * h as f32..=6. * h as f32).contains(&(w as f32)),
            "review: expected a tight strip of four square icons"
        );
        let mut found = std::array::from_fn(|_| SlotMatch::default());
        for (i, out) in found.iter_mut().enumerate() {
            let a = i as u32 * w / 4;
            let b = (i as u32 + 1) * w / 4;
            let tile = row.view(a, 0, b - a, h).to_image();
            let interior = frame::interior(&tile);
            let art = interior.as_ref().unwrap_or(&tile);
            let exposed = exposure(art);
            let small = small_view(art);
            let samples: Vec<_> = std::iter::once(art)
                .chain(exposed.as_ref())
                .flat_map(|im| [1, 5, 9, 13].into_iter().filter_map(move |t| glyph(im, t)))
                .chain(small.as_ref().and_then(|im| glyph(im, 1)))
                .collect();
            let mut scores: Vec<_> = self
                .templates
                .iter()
                .map(|(id, refs)| {
                    (
                        *id,
                        refs.iter()
                            .flat_map(|g| samples.iter().map(move |s| score(&s.body, &g.body)))
                            .fold(0., f32::max),
                    )
                })
                .collect();
            scores.sort_by(|a, b| b.1.total_cmp(&a.1));
            // A distinctive cutout may confirm the winner, never promote a
            // runner-up. Compare against *all* close rivals before truncating.
            let confirmed = if let [best, second, ..] = scores.as_slice()
                && best.1 >= 0.92
                && best.1 - second.1 < 0.065
            {
                let detail = |id| {
                    self.templates
                        .iter()
                        .find(|(i, _)| *i == id)
                        .unwrap()
                        .1
                        .iter()
                        .flat_map(|g| samples.iter().map(move |s| score(&s.holes, &g.holes)))
                        .fold(0., f32::max)
                };
                let own = detail(best.0);
                own >= 0.80
                    && scores[1..]
                        .iter()
                        .take_while(|s| best.1 - s.1 < 0.065)
                        .all(|s| own - detail(s.0) >= 0.15)
            } else {
                false
            };
            scores.truncate(3);
            out.reason = match scores.as_slice() {
                _ if samples.is_empty() => "no glyph",
                [best, ..] if best.1 < 0.86 => "low score",
                [best, second, ..] if best.1 - second.1 < 0.065 && !confirmed => "ambiguous",
                [best, _, ..] => {
                    out.accepted = Some(best.0);
                    if confirmed {
                        "accepted (cutout detail)"
                    } else {
                        "accepted"
                    }
                }
                _ => "insufficient references",
            };
            // Verify with a different representation, not a lower binary-score cutoff.
            let mut visual_conflict = false;
            if !samples.is_empty()
                && let Some(visual) = appearance::decisive(&self.appearance, art)
            {
                let (winner, _) = visual[0];
                if out.accepted.is_none()
                    && scores
                        .iter()
                        .any(|(id, s)| *id == winner && scores[0].1 - s <= 0.065)
                {
                    out.accepted = Some(winner);
                    out.reason = "accepted (registered color and detail)";
                    scores = visual;
                } else if out.accepted != Some(winner) {
                    // The silhouette cannot corroborate this decisive visual
                    // match. Do not let the highest-score fallback overrule it.
                    visual_conflict = true;
                    out.accepted = None;
                    out.reason = "conflicting visual evidence";
                }
            }
            // A learned label cannot break an unresolved exact tie between two
            // strong references. Compare the observed drawings, not their IDs.
            let tied = out.accepted.is_none()
                && matches!(scores.as_slice(),
                [first, second, ..] if first.1 >= 0.86 && first.1 == second.1);
            if !tied {
                out.sample = crate::hud::Print::extract(&tile);
            }
            if let Some(sample) = &out.sample
                && !visual_conflict
            {
                let mut hud: Vec<_> = self
                    .hud
                    .iter()
                    .map(|(id, prints)| {
                        (
                            *id,
                            prints
                                .iter()
                                .map(|p| p.similarity(sample))
                                .fold(0., f32::max),
                        )
                    })
                    .collect();
                hud.sort_by(|a, b| b.1.total_cmp(&a.1));
                if let Some(&(id, sim)) = hud.first()
                    && sim >= 0.95
                {
                    out.hud_best = Some((id, sim));
                    let conflict = hud.get(1).is_some_and(|s| sim - s.1 < 0.065)
                        || out.accepted.is_some_and(|a| a != id)
                        || scores.first().is_some_and(|&(a, s)| a != id && s >= 0.92);
                    if conflict {
                        out.accepted = None;
                        out.reason = "conflicting HUD reference";
                    } else {
                        out.accepted = Some(id);
                        out.reason = "confirmed HUD";
                    }
                }
            }

            // Extra evidence gets the first chance to resolve weak or close
            // matches. Without a conflict, use the unique leader rather than
            // requiring Review just because its score or margin was small.
            if out.accepted.is_none()
                && !samples.is_empty()
                && !visual_conflict
                && out.hud_best.is_none()
                && let [best, second, ..] = scores.as_slice()
                && best.1 > second.1
            {
                out.accepted = Some(best.0);
                out.reason = "accepted (highest score)";
            }
            out.top = scores;
        }
        let copy = found.each_ref().map(|s| s.accepted);
        for item in &mut found {
            if item.accepted.is_some() && copy.iter().filter(|x| **x == item.accepted).count() > 1 {
                item.accepted = None;
                item.reason = "duplicate";
            }
        }
        Ok(found)
    }
}
