use crate::Catalog;
use anyhow::{Context, Result, bail, ensure};
use evdev::{AttributeSet, KeyCode, KeyEvent, uinput::VirtualDevice};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Region {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub game: [i32; 2],
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub slots: [Option<String>; 4],
    pub row: Option<Region>,
    pub preset: String,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            slots: Default::default(),
            row: None,
            preset: "home-arrows".into(),
        }
    }
}
pub fn config_path() -> Result<PathBuf> {
    let root = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|s| PathBuf::from(s).join(".config")))
        .context("no config directory")?;
    Ok(root.join("helldeck/config.json"))
}
impl Config {
    pub fn load(c: &Catalog) -> Result<Self> {
        let p = config_path()?;
        let s: Self = match fs::read(&p) {
            Ok(d) => serde_json::from_slice(&d).context("invalid config; original left intact")?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Self::default(),
            Err(e) => return Err(e.into()),
        };
        bindings(&s.preset)?;
        for (i, id) in s.slots.iter().enumerate() {
            if let Some(id) = id {
                ensure!(
                    c.id(id).is_some_and(|n| c.entries[n].group == "loadout")
                        && !s.slots[..i].contains(&Some(id.clone())),
                    "invalid saved slot: {id}"
                );
            }
        }
        Ok(s)
    }
    pub fn save(&self) -> Result<()> {
        let path = config_path()?;
        let dir = path.parent().unwrap();
        fs::create_dir_all(dir)?;
        let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
        serde_json::to_writer_pretty(&mut tmp, self)?;
        tmp.flush()?;
        tmp.as_file().sync_all()?;
        tmp.persist(path)?;
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Window {
    pub class: String,
    pub address: String,
    pub pid: i64,
    pub mapped: bool,
    pub at: [i32; 2],
    pub size: [i32; 2],
}
pub fn is_game(w: &Window) -> bool {
    w.pid > 0
        && w.mapped
        && !w.address.is_empty()
        && w.address != "0x0"
        && matches!(
            w.class.to_ascii_lowercase().as_str(),
            "steam_app_553850" | "helldivers2.exe"
        )
}
fn output(timeout: &str, cmd: &str, args: &[&str]) -> Result<Vec<u8>> {
    let r = Command::new("timeout")
        .arg(timeout)
        .arg(cmd)
        .args(args)
        .output()
        .with_context(|| format!("cannot run {cmd}"))?;
    ensure!(
        r.status.success(),
        "{cmd} failed/cancelled: {}",
        String::from_utf8_lossy(&r.stderr).trim()
    );
    Ok(r.stdout)
}
pub fn game() -> Result<Window> {
    let w: Window = serde_json::from_slice(&output("1s", "hyprctl", &["-j", "activewindow"])?)
        .context("no active game window")?;
    ensure!(is_game(&w), "focus Helldivers 2 first");
    Ok(w)
}
fn geom(w: &Window, r: Region) -> Result<String> {
    ensure!(r.game == w.size, "game size changed; use SET ROW");
    ensure!(
        r.x >= 0
            && r.y >= 0
            && (64..=4096).contains(&r.w)
            && (16..=1024).contains(&r.h)
            && (3 * r.h..=6 * r.h).contains(&r.w)
            && i64::from(r.x) + i64::from(r.w) <= i64::from(w.size[0])
            && i64::from(r.y) + i64::from(r.h) <= i64::from(w.size[1]),
        "select only the four equipped icons in a horizontal row"
    );
    Ok(format!(
        "{},{} {}x{}",
        i64::from(w.at[0]) + i64::from(r.x),
        i64::from(w.at[1]) + i64::from(r.y),
        r.w,
        r.h
    ))
}
pub fn capture(old: Option<Region>, force: bool) -> Result<(Region, image::RgbImage)> {
    let w = game()?;
    let r = if !force && let Some(r) = old {
        r
    } else {
        let raw = output("45s", "slurp", &["-f", "%x %y %w %h"])?;
        let v = std::str::from_utf8(&raw)?
            .split_whitespace()
            .map(str::parse::<i32>)
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let [x, y, rw, rh] = v[..] else {
            bail!("selection cancelled")
        };
        Region {
            x: x.checked_sub(w.at[0]).context("bad x")?,
            y: y.checked_sub(w.at[1]).context("bad y")?,
            w: rw,
            h: rh,
            game: w.size,
        }
    };
    let bounds = geom(&w, r)?;
    ensure!(game()? == w, "game focus changed");
    let png = output("5s", "grim", &["-s", "1", "-g", &bounds, "-"])?;
    ensure!(game()? == w, "game focus changed; capture discarded");
    let im = image::load_from_memory(&png)?.to_rgb8();
    ensure!(
        im.dimensions() == (r.w as u32, r.h as u32),
        "capture scale mismatch; recalibrate"
    );
    Ok((r, im))
}
#[derive(Clone, Copy)]
pub struct Bind {
    pub menu: KeyCode,
    pub dirs: [KeyCode; 4],
    pub hold: bool,
}
pub fn bindings(s: &str) -> Result<Bind> {
    use KeyCode as K;
    match s {
        "home-arrows" => Ok(Bind {
            menu: K::KEY_HOME,
            dirs: [K::KEY_UP, K::KEY_DOWN, K::KEY_LEFT, K::KEY_RIGHT],
            hold: false,
        }),
        "ctrl-wasd" => Ok(Bind {
            menu: K::KEY_LEFTCTRL,
            dirs: [K::KEY_W, K::KEY_S, K::KEY_A, K::KEY_D],
            hold: true,
        }),
        _ => bail!("choose home-arrows or ctrl-wasd"),
    }
}
impl Bind {
    pub fn keys(self, code: &str) -> Result<Vec<KeyCode>> {
        code.bytes()
            .map(|x| {
                b"UDLR"
                    .iter()
                    .position(|&d| d == x)
                    .map(|i| self.dirs[i])
                    .context("invalid direction")
            })
            .collect()
    }
    pub fn describe(self, code: &str) -> Result<String> {
        let name = |k: KeyCode| format!("{k:?}").trim_start_matches("KEY_").to_owned();
        let mut v = vec![format!(
            "{}({})",
            name(self.menu),
            if self.hold { "hold" } else { "tap" }
        )];
        v.extend(self.keys(code)?.into_iter().map(name));
        if self.hold {
            v.push(format!("{}(release)", name(self.menu)))
        }
        Ok(v.join(" "))
    }
}
pub struct Keyboard {
    dev: VirtualDevice,
    held: Vec<KeyCode>,
}
#[derive(Debug)]
pub struct InputInterrupted;
impl std::fmt::Display for InputInterrupted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("sequence cancelled: paused or no longer the focused game window")
    }
}
impl std::error::Error for InputInterrupted {}
impl Keyboard {
    pub fn new(b: Bind) -> Result<Self> {
        let mut caps = AttributeSet::new();
        for k in b.dirs.into_iter().chain([b.menu]) {
            caps.insert(k)
        }
        let dev = VirtualDevice::builder()?
            .name("HellDeck Stratagem Keys")
            .with_keys(&caps)?
            .build()?;
        Ok(Self { dev, held: vec![] })
    }
    fn down(&mut self, k: KeyCode) -> Result<()> {
        self.held.push(k);
        self.dev.emit(&[*KeyEvent::new(k, 1)])?;
        Ok(())
    }
    fn up(&mut self, k: KeyCode) -> Result<()> {
        self.dev.emit(&[*KeyEvent::new(k, 0)])?;
        self.held.retain(|x| *x != k);
        Ok(())
    }
    pub fn send(
        &mut self,
        b: Bind,
        code: &str,
        target: &Window,
        cancel: &AtomicBool,
    ) -> Result<()> {
        let guard = || -> Result<()> {
            ensure!(
                !cancel.load(Ordering::SeqCst)
                    && game().is_ok_and(|w| w.pid == target.pid && w.address == target.address)
                    && !cancel.load(Ordering::SeqCst),
                InputInterrupted
            );
            Ok(())
        };
        let result = (|| {
            guard()?;
            self.down(b.menu)?;
            std::thread::sleep(Duration::from_millis(35));
            if !b.hold {
                self.up(b.menu)?;
            }
            std::thread::sleep(Duration::from_millis(75));
            for k in b.keys(code)? {
                guard()?;
                self.down(k)?;
                std::thread::sleep(Duration::from_millis(35));
                self.up(k)?;
                std::thread::sleep(Duration::from_millis(28));
            }
            Ok(())
        })();
        self.release();
        result
    }
    fn release(&mut self) {
        for k in self.held.drain(..).rev() {
            let _ = self.dev.emit(&[*KeyEvent::new(k, 0)]);
        }
    }
}
impl Drop for Keyboard {
    fn drop(&mut self) {
        self.release()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn window() -> Window {
        Window {
            class: "steam_app_553850".into(),
            address: "0x1".into(),
            pid: 10,
            mapped: true,
            at: [-1920, 0],
            size: [1920, 1080],
        }
    }
    #[test]
    fn geometry_checks_game_and_monitor_origin() {
        let w = window();
        let r = Region {
            x: 10,
            y: 20,
            w: 200,
            h: 50,
            game: w.size,
        };
        assert_eq!(geom(&w, r).unwrap(), "-1910,20 200x50");
        assert!(geom(&w, Region { x: i32::MAX, ..r }).is_err());
        assert!(
            geom(
                &w,
                Region {
                    game: [1600, 900],
                    ..r
                }
            )
            .is_err()
        )
    }
    #[test]
    fn title_or_substring_is_not_identity() {
        let mut w = window();
        assert!(is_game(&w));
        w.class = "my-steam_app_553850-chat".into();
        assert!(!is_game(&w));
    }
}
