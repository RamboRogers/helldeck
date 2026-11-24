pub mod platform;
pub mod sync;
pub mod ui;
use anyhow::{Result, ensure};
use image::{GenericImageView, RgbImage};
use serde::Deserialize;
#[derive(Deserialize, Debug)]
pub struct Entry {
    pub id: String,
    pub name: String,
    pub group: String,
    pub code: String,
}
pub struct Catalog {
    pub entries: Vec<Entry>,
    pub icons: Vec<RgbImage>,
}
impl Catalog {
    pub fn load() -> Result<Self> {
        let entries: Vec<Entry> = serde_json::from_str(include_str!("../assets/catalog.json"))?;
        let atlas = image::load_from_memory(include_bytes!("../assets/atlas.png"))?.to_rgb8();
        let mut icons = vec![];
        for (i, e) in entries.iter().enumerate() {
            ensure!(
                e.code.len() >= 3
                    && e.code.len() <= 12
                    && e.code.bytes().all(|x| b"UDLR".contains(&x)),
                "invalid catalog sequence: {}",
                e.id
            );
            let i = i as u32;
            icons.push(atlas.view(i % 8 * 72, i / 8 * 72, 72, 72).to_image());
        }
        Ok(Self { entries, icons })
    }
    pub fn id(&self, id: &str) -> Option<usize> {
        self.entries.iter().position(|e| e.id == id)
    }
}
