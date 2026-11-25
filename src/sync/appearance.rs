//! Compare the light and colored ink without binarizing away antialiasing or small markings.
use image::{RgbImage, imageops};

const SIDE: usize = 48;
pub(super) struct Appearance(Vec<f32>);

/// A near-exact match must also explain substantially more of the signal than
/// any alternative ID. Repeated drawings for one ID aren't competing labels.
pub(super) fn decisive(bank: &[Vec<Appearance>], tile: &RgbImage) -> Option<Vec<(usize, f32)>> {
    let query = Appearance::extract(tile)?;
    let mut scores: Vec<_> = bank
        .iter()
        .enumerate()
        .filter(|(_, refs)| !refs.is_empty())
        .map(|(id, refs)| {
            (
                id,
                refs.iter().map(|r| query.similarity(r)).fold(0., f32::max),
            )
        })
        .collect();
    scores.sort_unstable_by(|a, b| b.1.total_cmp(&a.1));
    let [best, rival, ..] = scores.as_slice() else {
        return None;
    };
    if best.1 < 0.975 || best.1 - rival.1 < 0.004 || (1. - best.1) >= 0.45 * (1. - rival.1) {
        return None;
    }
    scores.truncate(3);
    Some(scores)
}

impl Appearance {
    pub(super) fn extract(tile: &RgbImage) -> Option<Self> {
        let (w, h) = tile.dimensions();
        if w.max(h) > 160 {
            let size = |n: u32| (n * 160 / w.max(h)).max(1);
            return Self::extract(&imageops::resize(
                tile,
                size(w),
                size(h),
                imageops::FilterType::Triangle,
            ));
        }
        let mut hist = [0u32; 256];
        for p in tile.pixels() {
            hist[usize::from(*p.0.iter().max()?)] += 1;
        }
        let mut n = 0;
        let peak = (0..256).rev().find(|&v| {
            n += hist[v];
            n >= (w * h).div_ceil(200)
        })? as f32;
        if peak < 74. {
            return None;
        }
        let mut field = Vec::with_capacity((w * h) as usize);
        let (mut mass, mut mx, mut my, mut second) = (0., 0., 0., 0.);
        let mut hue = [0.; 3];
        for (x, y, p) in tile.enumerate_pixels() {
            let rgb = p.0.map(f32::from);
            let hi = rgb[0].max(rgb[1]).max(rgb[2]);
            let lo = rgb[0].min(rgb[1]).min(rgb[2]);
            let chroma = (hi - lo) / peak;
            let white = (lo / peak - 0.7 * chroma - 0.30).max(0.);
            let color = (chroma - 0.12).max(0.) * ((hi / peak - 0.26) * 6.).clamp(0., 1.);
            field.push([white, color]);
            let m = white + color;
            let (x, y) = (x as f32, y as f32);
            mass += m;
            mx += x * m;
            my += y * m;
            second += (x * x + y * y) * m;
            for (h, v) in hue.iter_mut().zip(rgb) {
                *h += (v - lo) * color;
            }
        }
        if mass < 0.012 * (w * h) as f32 {
            return None;
        }
        mx /= mass;
        my /= mass;
        // Centroid and RMS radius register translation and isotropic scale without
        // making a thin weapon square or relying on one extreme foreground pixel.
        let radius = (second / mass - mx * mx - my * my).max(0.).sqrt() * 2.4;
        if radius < 4. {
            return None;
        }
        let mut v = vec![0.; 2 * SIDE * SIDE];
        let mut energy = [0.; 2];
        for y in 0..SIDE {
            for x in 0..SIDE {
                let coordinate = |n: usize, center: f32| {
                    center + ((n as f32 + 0.5) * 2. / SIDE as f32 - 1.) * radius
                };
                let (px, py) = (coordinate(x, mx), coordinate(y, my));
                if px < -0.5 || py < -0.5 || px >= w as f32 - 0.5 || py >= h as f32 - 0.5 {
                    continue;
                }
                let (px, py) = (px.max(0.), py.max(0.));
                let (x0, y0) = (px as u32, py as u32);
                let (x1, y1) = ((x0 + 1).min(w - 1), (y0 + 1).min(h - 1));
                let at = |x, y| field[(y * w + x) as usize];
                let (a, b, c, d) = (at(x0, y0), at(x1, y0), at(x0, y1), at(x1, y1));
                let (fx, fy) = (px.fract(), py.fract());
                for k in 0..2 {
                    let val = (a[k] * (1. - fx) + b[k] * fx) * (1. - fy)
                        + (c[k] * (1. - fx) + d[k] * fx) * fy;
                    v[k * SIDE * SIDE + y * SIDE + x] = val;
                    energy[k] += val * val;
                }
            }
        }
        for (layer, e) in v
            .as_chunks_mut::<{ SIDE * SIDE }>()
            .0
            .iter_mut()
            .zip(energy)
        {
            if e < 1. {
                return None;
            }
            let scale = (0.46 / e).sqrt();
            for val in layer {
                *val *= scale;
            }
        }
        let norm = hue.iter().map(|v| v * v).sum::<f32>().sqrt();
        if norm < 1. {
            return None;
        }
        v.extend(hue.map(|v| v * (0.08f32).sqrt() / norm));
        Some(Self(v))
    }
    pub(super) fn similarity(&self, other: &Self) -> f32 {
        self.0
            .iter()
            .zip(&other.0)
            .map(|(a, b)| a * b)
            .sum::<f32>()
            .min(1.)
    }
}
