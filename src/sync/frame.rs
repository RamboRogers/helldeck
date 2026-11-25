//! Find the four straight sides of a slot decoration without assuming a fixed inset.
use image::{Rgb, RgbImage};

// The outline is a single solid UI color. Allow small changes from resampling,
// not arbitrary bright pixels from a weapon or the scene behind the HUD.
fn near(p: Rgb<u8>, q: Rgb<u8>) -> bool {
    p.0.into_iter().zip(q.0).all(|(a, b)| a.abs_diff(b) <= 32)
}

/// Seek a long uniform band perpendicular to the scan direction. Return its
/// outer and inner (exclusive) depth and color; measure from the edge inwards.
fn band(depth: u32, span: u32, pixel: impl Fn(u32, u32) -> Rgb<u8>) -> Option<(u32, u32, Rgb<u8>)> {
    let along = span / 4..span - span / 4;
    let solid = |d, color, shade| {
        along
            .clone()
            .filter(|&s| {
                let p = pixel(d, s);
                near(p, color) || near(p, shade)
            })
            .count()
            * 10
            >= along.len() * 9
    };
    for d in 0..depth / 4 {
        let c = pixel(d, span / 2);
        if c.0.into_iter().max()? < 85 || !solid(d, c, c) {
            continue;
        }
        let end = (d + 1..depth / 3).find(|&y| !solid(y, c, c))?;
        if end - d <= depth.div_ceil(10) {
            // An antialiased inner lip can still be bright enough to enter
            // the glyph mask. Remove it only if the whole line has the same tint.
            let fringe = Rgb(c.0.map(|v| (u16::from(v) * 7 / 10) as u8));
            return Some((d, end + u32::from(solid(end, c, fringe)), c));
        }
        return None;
    }
    None
}

pub(super) fn interior(tile: &RgbImage) -> Option<RgbImage> {
    let (w, h) = tile.dimensions();
    let (l, x0, c) = band(w, h, |d, s| *tile.get_pixel(d, s))?;
    let (r, right, cr) = band(w, h, |d, s| *tile.get_pixel(w - 1 - d, s))?;
    let (t, y0, ct) = band(h, w, |d, s| *tile.get_pixel(s, d))?;
    let (b, bottom, cb) = band(h, w, |d, s| *tile.get_pixel(s, h - 1 - d))?;
    let (fw, fh) = (w - l - r, h - t - b);
    if ![cr, ct, cb].into_iter().all(|p| near(p, c)) || fw.abs_diff(fh) > fw.min(fh) / 5 {
        return None;
    }
    // Check near the corners too: four independent bars aren't necessarily a box.
    for x in [l + fw / 10, l + fw - 1 - fw / 10] {
        for y in [t + (y0 - t) / 2, h - b - 1 - (bottom - b) / 2] {
            if !near(*tile.get_pixel(x, y), c) {
                return None;
            }
        }
    }
    let mut result = tile.clone();
    for (x, y, p) in result.enumerate_pixels_mut() {
        if x < x0 || x >= w - right || y < y0 || y >= h - bottom {
            *p = Rgb([0; 3]);
        }
    }
    Some(result)
}
