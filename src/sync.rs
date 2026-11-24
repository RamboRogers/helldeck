//! Recognize four equally spaced cells in a user-calibrated strip. No screen data leaves the process.
use crate::Catalog;
use anyhow::{Result, ensure};
use image::{
    GenericImageView, GrayImage, Luma, RgbImage,
    imageops::{self, FilterType},
};
type Glyph = Vec<bool>;
fn glyph(tile: &RgbImage, trim: u32) -> Option<Glyph> {
    let (w, h) = tile.dimensions();
    let (dx, dy) = (w * trim / 100, h * trim / 100);
    let mut mask = GrayImage::new(w, h);
    let mut light = GrayImage::new(w, h);
    let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0, 0);
    let mut n = 0;
    for y in dy..h - dy {
        for x in dx..w - dx {
            let c = tile.get_pixel(x, y).0;
            let lo = *c.iter().min()?;
            let hi = *c.iter().max()?;
            if (lo >= 145 && hi - lo <= 65) || (hi >= 130 && hi - lo >= 48) {
                mask.put_pixel(x, y, Luma([255]));
                if lo >= 145 && hi - lo <= 65 {
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
    if n < w * h / 55 || n > w * h * 3 / 5 || x1.saturating_sub(x0) < 4 || y1.saturating_sub(y0) < 4
    {
        return None;
    }
    let glyph = mask.view(x0, y0, x1 - x0 + 1, y1 - y0 + 1).to_image();
    let detail = light.view(x0, y0, x1 - x0 + 1, y1 - y0 + 1).to_image();
    Some(
        [glyph, detail]
            .iter()
            .flat_map(|m| imageops::resize(m, 32, 32, FilterType::Triangle).into_raw())
            .map(|x| x >= 128)
            .collect(),
    )
}
fn score(a: &Glyph, b: &Glyph) -> f32 {
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
    pub accepted: Option<usize>,
    pub top: Vec<(usize, f32)>,
    pub reason: &'static str,
}
pub struct Matcher(Vec<(usize, Vec<Glyph>)>);
impl SlotMatch {
    pub fn log(&self, slot: usize, cat: &Catalog) {
        let names = self
            .top
            .iter()
            .map(|(i, score)| format!("{}={score:.3}", cat.entries[*i].id))
            .collect::<Vec<_>>()
            .join(" ");
        eprintln!("Sync slot {}: {}; {}", slot + 1, self.reason, names);
    }
}
impl Matcher {
    pub fn new(cat: &Catalog) -> Self {
        Self(
            cat.icons
                .iter()
                .enumerate()
                .filter(|(i, _)| cat.entries[*i].group == "loadout")
                .filter_map(|(i, t)| {
                    let mut refs = vec![glyph(t, 1)?];
                    refs.extend(glyph(&imageops::blur(t, 1.1), 1));
                    Some((i, refs))
                })
                .collect(),
        )
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
            let exposed = exposure(&tile);
            let samples: Vec<_> = std::iter::once(&tile)
                .chain(exposed.as_ref())
                .flat_map(|im| [1, 5, 9, 13].into_iter().filter_map(move |t| glyph(im, t)))
                .collect();
            let mut scores: Vec<_> = self
                .0
                .iter()
                .map(|(id, refs)| {
                    (
                        *id,
                        refs.iter()
                            .flat_map(|g| samples.iter().map(move |s| score(s, g)))
                            .fold(0., f32::max),
                    )
                })
                .collect();
            scores.sort_by(|a, b| b.1.total_cmp(&a.1));
            scores.truncate(3);
            out.reason = match scores.as_slice() {
                _ if samples.is_empty() => "no glyph",
                [best, ..] if best.1 < 0.86 => "low score",
                [best, second, ..] if best.1 - second.1 < 0.065 => "ambiguous",
                [best, _, ..] => {
                    out.accepted = Some(best.0);
                    "accepted"
                }
                _ => "insufficient references",
            };
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
