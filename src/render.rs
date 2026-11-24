use font8x8::{BASIC_FONTS, UnicodeFonts};
use helldeck::{Catalog, ui::Tile};
use image::{Rgb, RgbImage, imageops};
fn text(img: &mut RgbImage, s: &str, y: u32, col: [u8; 3]) {
    let chars: Vec<_> = s.chars().take(9).collect();
    let start = (72 - chars.len() as u32 * 8) / 2;
    for (i, c) in chars.into_iter().enumerate() {
        if let Some(g) = BASIC_FONTS.get(c) {
            for (row, b) in g.iter().enumerate() {
                for x in 0..8 {
                    if b & (1 << x) != 0 {
                        img.put_pixel(start + i as u32 * 8 + x, y + row as u32, Rgb(col))
                    }
                }
            }
        }
    }
}
fn lines(s: &str) -> Vec<String> {
    let mut out = vec![];
    let mut line = String::new();
    for word in s.split_whitespace() {
        if !line.is_empty() && line.len() + 1 + word.len() > 9 {
            out.push(std::mem::take(&mut line))
        }
        if !line.is_empty() {
            line.push(' ')
        }
        line.extend(word.chars().take(9))
    }
    if !line.is_empty() {
        out.push(line)
    }
    out
}
pub fn image(t: &Tile, c: &Catalog) -> RgbImage {
    // The render layer alone changes: hardware input, labels in CLI layout, and pause semantics stay intact.
    if let Tile::Label(s) = t {
        let art: Option<&[u8]> = match s.as_str() {
            "ARM" => Some(include_bytes!("../assets/brand/arm-standby.png")),
            "PAUSE" => Some(include_bytes!("../assets/brand/arm-live.png")),
            _ => None,
        };
        if let Some(bytes) = art {
            return image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
                .expect("embedded arming key artwork")
                .to_rgb8();
        }
    }
    let mut img = RgbImage::from_pixel(72, 72, Rgb([13, 18, 21]));
    let mut label = String::new();
    let mut offset = 28;
    let mut color = [255, 214, 71];
    match t {
        Tile::Blank => (),
        Tile::Label(s) => label = s.clone(),
        Tile::Icon(id) => {
            let icon = imageops::resize(&c.icons[*id], 52, 52, imageops::FilterType::Triangle);
            imageops::replace(&mut img, &icon, 10, 0);
            label = c.entries[*id].name.to_uppercase();
            offset = 53;
            color = [226, 233, 236]
        }
    }
    for (i, l) in lines(&label).into_iter().take(2).enumerate() {
        text(&mut img, &l, offset + i as u32 * 9, color)
    }
    img
}
pub fn sheet(tiles: &[Tile; 15], c: &Catalog) -> RgbImage {
    let mut out = RgbImage::from_pixel(388, 236, Rgb([4, 7, 10]));
    for (i, t) in tiles.iter().enumerate() {
        imageops::replace(
            &mut out,
            &image(t, c),
            4 + (i % 5) as i64 * 77,
            4 + (i / 5) as i64 * 77,
        )
    }
    imageops::resize(&out, 776, 472, imageops::FilterType::Nearest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use helldeck::ui::{Page, Panel};
    #[test]
    fn arming_key_has_pictorial_states_on_both_pages() {
        let c = Catalog::load().unwrap();
        let mut panel = Panel::new([None; 4]);
        for armed in [false, true] {
            panel.armed = armed;
            panel.page = Page::Home;
            let home = image(&panel.tiles(&c)[14], &c);
            panel.page = Page::Missions(0);
            assert_eq!(home, image(&panel.tiles(&c)[13], &c));
            assert_eq!(home.dimensions(), (72, 72));
            let lit = home
                .enumerate_pixels()
                .filter(|(_, y, p)| {
                    *y < 48
                        && if armed {
                            p[1] > 150 && p[0] < p[1]
                        } else {
                            p[0] > 160 && p[1] > 100 && p[2] < p[0]
                        }
                })
                .count();
            assert!(
                lit > 200,
                "missing large state-specific pictogram (armed={armed}, lit pixels={lit})"
            );
        }
    }
}
