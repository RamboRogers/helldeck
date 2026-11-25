//! Optional private game references; never replace the artwork on the deck.
use anyhow::{Result, ensure};
use image::{ImageFormat, ImageReader, Limits, Rgb, RgbImage, imageops};
use std::{fs, io::ErrorKind, path::Path};

pub(super) fn load(path: &Path) -> Result<Option<RgbImage>> {
    let meta = match fs::symlink_metadata(path) {
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
        other => other?,
    };
    ensure!(
        meta.is_file() && meta.len() <= 2_000_000,
        "expected a small regular PNG"
    );
    let mut reader = ImageReader::open(path)?;
    reader.set_format(ImageFormat::Png);
    let mut limits = Limits::default();
    limits.max_image_width = Some(512);
    limits.max_image_height = Some(512);
    limits.max_alloc = Some(8 << 20);
    reader.limits(limits);
    let rgba = reader.decode()?.into_rgba8();
    ensure!(
        rgba.width() == rgba.height() && rgba.width() >= 24,
        "expected a square icon"
    );
    let rgb = RgbImage::from_fn(rgba.width(), rgba.height(), |x, y| {
        let [r, g, b, a] = rgba.get_pixel(x, y).0;
        Rgb([r, g, b].map(|c| (u16::from(c) * u16::from(a) / 255) as u8))
    });
    Ok(Some(imageops::resize(
        &rgb,
        72,
        72,
        imageops::FilterType::Triangle,
    )))
}
