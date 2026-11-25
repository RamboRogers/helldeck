//! User-confirmed icon fingerprints. No RGB pixels, screen coordinates or game inputs are stored.
use anyhow::{Result, ensure};
use image::{
    RgbImage,
    imageops::{self, FilterType},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    path::Path,
};

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Print {
    white: [u32; 32],
    color: [u32; 32],
    hue: u8,
}
impl Print {
    pub fn extract(tile: &RgbImage) -> Option<Self> {
        let (w, h) = tile.dimensions();
        let (dx, dy) = (w / 10, h / 10);
        let thumb = imageops::resize(
            &imageops::crop_imm(tile, dx, dy, w - 2 * dx, h - 2 * dy).to_image(),
            32,
            32,
            FilterType::Triangle,
        );
        let mut levels: Vec<_> = thumb.pixels().map(|p| *p.0.iter().max().unwrap()).collect();
        levels.sort_unstable();
        let peak = u16::from(levels[1013]);
        if peak < 80 || peak - u16::from(levels[100]) < 45 {
            return None;
        }
        let mut fp = Self {
            white: [0; 32],
            color: [0; 32],
            hue: 0,
        };
        let mut hues = [0u16; 3];
        for (x, y, p) in thumb.enumerate_pixels() {
            let [r, g, b] = p.0.map(u16::from);
            let hi = r.max(g).max(b);
            let lo = r.min(g).min(b);
            if lo * 100 >= peak * 57 && (hi - lo) * 100 <= peak * 25 {
                fp.white[y as usize] |= 1 << x;
            } else if hi * 100 >= peak * 51 && (hi - lo) * 100 >= peak * 19 {
                fp.color[y as usize] |= 1 << x;
                hues[if hi == r {
                    0
                } else if hi == g {
                    1
                } else {
                    2
                }] += 1;
            }
        }
        let bits = |a: &[u32; 32]| a.iter().map(|r| r.count_ones()).sum::<u32>();
        let (white, color) = (bits(&fp.white), bits(&fp.color));
        if white < 12 || color < 8 || white + color > 780 {
            return None;
        }
        fp.hue = hues.iter().enumerate().max_by_key(|(_, n)| *n)?.0 as u8;
        Some(fp)
    }
    pub fn similarity(&self, other: &Self) -> f32 {
        if self.hue != other.hue {
            return 0.;
        }
        fn dice(a: &[u32; 32], b: &[u32; 32]) -> f32 {
            let mut both = 0;
            let mut total = 0;
            for (a, b) in a.iter().zip(b) {
                both += (a & b).count_ones();
                total += a.count_ones() + b.count_ones();
            }
            2. * both as f32 / total.max(1) as f32
        }
        dice(&self.white, &other.white).min(dice(&self.color, &other.color))
    }
}
#[derive(Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct HudBook(BTreeMap<String, Vec<Print>>);
impl HudBook {
    pub fn path() -> Result<std::path::PathBuf> {
        Ok(crate::platform::config_path()?.with_file_name("hud-v1.json"))
    }
    pub fn remember(&mut self, id: &str, sample: Print) {
        let prints = self.0.entry(id.to_owned()).or_default();
        prints.retain(|p| p != &sample);
        prints.push(sample);
        if prints.len() > 3 {
            prints.remove(0);
        }
    }
    pub fn indexed(&self, cat: &crate::Catalog) -> Vec<(usize, Vec<Print>)> {
        self.0
            .iter()
            .filter_map(|(id, v)| {
                cat.id(id)
                    .filter(|&i| cat.entries[i].group == "loadout")
                    .map(|i| (i, v.clone()))
            })
            .collect()
    }
    pub fn load(path: &Path) -> Result<Self> {
        let f = match fs::File::open(path) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(e.into()),
        };
        let mut bytes = vec![];
        f.take(2_000_001).read_to_end(&mut bytes)?;
        ensure!(bytes.len() <= 2_000_000, "oversize HUD library");
        let book: Self = serde_json::from_slice(&bytes)?;
        ensure!(
            book.0.len() <= 512 && book.0.values().all(|v| v.len() <= 3),
            "invalid HUD library"
        );
        Ok(book)
    }
    pub fn save(&self, path: &Path) -> Result<()> {
        let parent = path.parent().expect("HUD config directory");
        fs::create_dir_all(parent)?;
        let mut tmp = tempfile::NamedTempFile::new_in(parent)?;
        serde_json::to_writer(&mut tmp, self)?;
        tmp.flush()?;
        tmp.as_file().sync_all()?;
        tmp.persist(path)?;
        Ok(())
    }
}
