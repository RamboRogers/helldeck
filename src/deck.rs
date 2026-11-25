use crate::render;
use anyhow::{Context, Result, bail, ensure};
use elgato_streamdeck::{self as sd, DeviceStateUpdate, StreamDeck, info::Kind};
use helldeck::{
    Catalog,
    platform::{self as pf, Config, Keyboard, Window},
    sync::Matcher,
    ui::{Action, Page, Panel, Tile},
};
use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};
fn devices() -> Result<(hidapi::HidApi, String)> {
    let hid = sd::new_hidapi()?;
    let mut found = sd::list_devices(&hid)
        .into_iter()
        .filter(|(k, _)| *k == Kind::Mk2);
    let (_, serial) = found.next().context("MK.2 (0fd9:0080) not connected")?;
    ensure!(
        found.next().is_none(),
        "multiple MK.2 decks; disconnect the non-game deck"
    );
    Ok((hid, serial))
}
fn deck_node() -> Result<PathBuf> {
    for entry in fs::read_dir("/sys/class/hidraw")? {
        let e = entry?;
        if fs::read_to_string(e.path().join("device/uevent"))?
            .lines()
            .any(|l| l.eq_ignore_ascii_case("HID_ID=0003:00000FD9:00000080"))
        {
            return Ok(PathBuf::from("/dev").join(e.file_name()));
        }
    }
    bail!("MK.2 hidraw node not found")
}
// Wine's system HID service also opens the Deck when HD2 is running. It was
// already coexisting with HellDeck before reload. Do not mistake it for a deck manager.
fn is_hd2_proton_hid(comm: &str, exe: &std::path::Path, env: &[u8]) -> bool {
    let vars = || env.split(|b| *b == 0);
    comm.trim() == "winedevice.exe"
        && matches!(
            exe.file_name().and_then(|x| x.to_str()),
            Some("wine64-preloader" | "wine-preloader")
        )
        && exe.to_string_lossy().contains("/steamapps/common/Proton")
        && vars().any(|s| s == b"SteamAppId=553850")
        && vars().any(|s| {
            s.strip_prefix(b"WINEPREFIX=").is_some_and(|v| {
                v.strip_suffix(b"/")
                    .unwrap_or(v)
                    .ends_with(b"/compatdata/553850/pfx")
            })
        })
}
fn approved_holder(pid: u32) -> Result<bool> {
    use std::os::unix::fs::MetadataExt;
    let p = PathBuf::from(format!("/proc/{pid}"));
    if fs::metadata(&p)?.uid() != fs::metadata("/proc/self")?.uid() {
        return Ok(false);
    }
    Ok(is_hd2_proton_hid(
        &fs::read_to_string(p.join("comm"))?,
        &fs::read_link(p.join("exe"))?,
        &fs::read(p.join("environ"))?,
    ))
}
fn idle_node() -> Result<PathBuf> {
    let node = deck_node()?;
    let r = Command::new("fuser")
        .arg(&node)
        .output()
        .context("fuser is required for the device-owner check")?;
    if r.status.success() {
        let ids = std::str::from_utf8(&r.stdout)?
            .split_whitespace()
            .map(str::parse::<u32>)
            .collect::<std::result::Result<Vec<_>, _>>()?;
        ensure!(
            !ids.is_empty() && ids.iter().all(|id| approved_holder(*id).unwrap_or(false)),
            "{} is busy: non-game HID owner(s) {:?}; close competing deck manager",
            node.display(),
            ids
        );
        eprintln!("Sharing MK.2 with Helldivers 2's Proton HID service");
    } else {
        ensure!(
            r.status.code() == Some(1) && r.stderr.is_empty(),
            "cannot verify owner of {}",
            node.display()
        );
    }
    Ok(node)
}
pub fn doctor() -> Result<()> {
    let (h, s) = devices()?;
    let node = idle_node()?;
    let d = StreamDeck::connect(&h, Kind::Mk2, &s)?;
    println!(
        "{} at {} (firmware {}, {} keys)\n/dev/uinput writable: {}\nFocused Helldivers 2 window: {}",
        d.product()?,
        node.display(),
        d.firmware_version()?,
        d.kind().key_count(),
        fs::OpenOptions::new()
            .write(true)
            .open("/dev/uinput")
            .is_ok(),
        pf::game().is_ok()
    );
    Ok(())
}
fn draw(d: &StreamDeck, c: &Catalog, p: &Panel, last: &mut Option<[Tile; 15]>) -> Result<()> {
    let tiles = p.tiles(c);
    if last.as_ref() == Some(&tiles) {
        return Ok(());
    }
    for (i, t) in tiles.iter().enumerate() {
        if last.as_ref().is_none_or(|prev| prev[i] != *t) {
            d.set_button_image(i as u8, image::DynamicImage::ImageRgb8(render::image(t, c)))?
        }
    }
    d.flush()?;
    *last = Some(tiles);
    Ok(())
}
fn save_slots(c: &Catalog, cfg: &mut Config, p: &mut Panel, ids: [usize; 4]) -> Result<()> {
    let mut new = cfg.clone();
    new.slots = ids.map(|i| Some(c.entries[i].id.clone()));
    new.save()?;
    *cfg = new;
    p.saved(ids);
    Ok(())
}
struct Job {
    cancel: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<Result<()>>>,
}
fn arm_panel(p: &mut Panel, active: Option<&Window>) {
    p.armed = true;
    observe_focus(p, active);
    p.ready_notice();
}

fn observe_focus(p: &mut Panel, active: Option<&Window>) -> bool {
    if !p.armed {
        return false;
    }
    let waiting = !active.is_some_and(pf::is_game);
    if p.waiting_focus != waiting {
        p.waiting_focus = waiting;
        p.ready_notice();
        eprintln!(
            "Focus: {}",
            if waiting {
                "input suspended; Arm retained"
            } else {
                "armed game returned; new presses eligible"
            }
        );
    }
    !waiting
}
impl Drop for Job {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::SeqCst);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}
pub fn run(c: &Catalog, preset: Option<&str>) -> Result<()> {
    let dir = PathBuf::from(
        std::env::var_os("XDG_RUNTIME_DIR").context("missing user runtime directory")?,
    );
    let lock = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(dir.join("helldeck.lock"))?;
    fs2::FileExt::try_lock_exclusive(&lock).context("another HellDeck instance owns the lock")?;
    let mut cfg = Config::load(c)?;
    let bind = pf::bindings(preset.unwrap_or(&cfg.preset))?;
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    ctrlc::set_handler(move || flag.store(true, Ordering::SeqCst))?;
    let mut tray = crate::tray::TrayState::start(stop.clone());
    let mut matcher = Arc::new(Matcher::local(c));
    while !stop.load(Ordering::SeqCst) {
        let (h, serial) = devices()?;
        idle_node()?;
        // The upstream get_reader() API requires &Arc<Self>; both handles stay on this thread.
        #[allow(clippy::arc_with_non_send_sync)]
        let d = Arc::new(StreamDeck::connect(&h, Kind::Mk2, &serial)?);
        let reader = d.get_reader();
        let mut p = Panel::new(cfg.slots.clone().map(|s| s.and_then(|id| c.id(&id))));
        let mut last = None;
        let mut keyboard: Option<Arc<Mutex<Keyboard>>> = None;
        let mut job: Option<Job> = None;
        type Scan = Result<(pf::Region, [helldeck::sync::SlotMatch; 4])>;
        let mut scan: Option<thread::JoinHandle<Scan>> = None;
        let mut last_focus = Instant::now();
        draw(&d, c, &p, &mut last)?;
        eprintln!(
            "MK.2: 15 keys written; paused. SYNC = equipped row; MISSIONS = objective calls."
        );
        while !stop.load(Ordering::SeqCst) {
            if job
                .as_ref()
                .is_some_and(|j| j.handle.as_ref().unwrap().is_finished())
            {
                let mut j = job.take().unwrap();
                let outcome = j
                    .handle
                    .take()
                    .unwrap()
                    .join()
                    .map_err(|_| anyhow::anyhow!("input worker stopped"))?;
                if let Err(e) = outcome {
                    eprintln!("{e:#}");
                    if !e.is::<pf::InputInterrupted>() {
                        p.armed = false;
                        keyboard = None;
                        p.notice = "INPUT ERR".into();
                    }
                } else {
                    eprintln!("Cast: keyboard sequence emitted (game acceptance unknown)");
                }
            }
            if scan.as_ref().is_some_and(|j| j.is_finished()) {
                let result = scan
                    .take()
                    .unwrap()
                    .join()
                    .map_err(|_| anyhow::anyhow!("sync worker stopped"))?;
                match result {
                    Ok((r, report)) => {
                        eprintln!("Sync row {}x{} at {},{}", r.w, r.h, r.x, r.y);
                        for (slot, decision) in report.iter().enumerate() {
                            decision.log(slot, c);
                        }
                        let draft = report.each_ref().map(|s| s.accepted);
                        eprintln!(
                            "Sync slots: {}",
                            draft
                                .map(|id| id.map_or("?", |i| c.entries[i].id.as_str()))
                                .join(", ")
                        );
                        let mut next = cfg.clone();
                        next.row = Some(r);
                        next.save()?;
                        cfg = next;
                        if let [Some(a), Some(b), Some(cc), Some(dd)] = draft {
                            save_slots(c, &mut cfg, &mut p, [a, b, cc, dd])?
                        } else {
                            p.review_scan(report);
                            eprintln!("Sync needs review. Choose missing slots, Apply or Cancel.")
                        }
                    }
                    Err(e) => {
                        eprintln!("Sync: {e:#}");
                        p.notice = "SYNC FAIL".into();
                    }
                }
            }
            if p.armed && last_focus.elapsed() >= Duration::from_millis(250) {
                last_focus = Instant::now();
                if !observe_focus(&mut p, pf::game().ok().as_ref())
                    && let Some(j) = &job
                {
                    j.cancel.store(true, Ordering::SeqCst);
                }
            }
            tray.show(if !p.armed {
                "Paused"
            } else if p.waiting_focus {
                "Waiting for game"
            } else if scan.is_some() || matches!(p.page, Page::Review | Page::Pick(..)) {
                "Editing"
            } else {
                "Armed"
            });
            draw(&d, c, &p, &mut last)?;
            let events = match reader.read(Some(Duration::from_millis(15))) {
                Ok(ev) => ev,
                Err(e) => {
                    eprintln!("Deck disconnected: {e}");
                    break;
                }
            };
            for ev in events {
                let DeviceStateUpdate::ButtonDown(k) = ev else {
                    continue;
                };
                let k = k as usize;
                if scan.is_some() || job.is_some() {
                    if p.armed
                        && ((k == 14 && p.page == Page::Home)
                            || (k == 13 && matches!(p.page, Page::Missions(_))))
                    {
                        if let Some(j) = &job {
                            j.cancel.store(true, Ordering::SeqCst);
                        }
                        eprintln!("Disarm: Pause pressed");
                        p.armed = false;
                        p.notice = "PAUSED".into()
                    }
                    continue;
                }
                let before = p.page;
                let was_armed = p.armed;
                let action = p.press(k, c);
                eprintln!(
                    "Deck key={} page={before:?} armed={was_armed} action={action:?} -> page={:?} status={}",
                    k + 1,
                    p.page,
                    p.notice
                );
                match action {
                    Action::None => (),
                    Action::Arm => {
                        if keyboard.is_none() {
                            match Keyboard::new(bind) {
                                Ok(kb) => keyboard = Some(Arc::new(Mutex::new(kb))),
                                Err(e) => {
                                    p.notice = "INPUT ERR".into();
                                    eprintln!("Arm: {e:#}");
                                    continue;
                                }
                            }
                        }
                        arm_panel(&mut p, pf::game().ok().as_ref());
                        eprintln!("Arm: virtual keyboard ready; {}", p.notice);
                    }
                    Action::Cast(id) => {
                        let active = pf::game().ok();
                        if !observe_focus(&mut p, active.as_ref()) {
                            eprintln!("Cast ignored: game not eligible; not queued");
                            continue;
                        }
                        let Some(w) = active else { continue };
                        let Some(kb) = keyboard.clone() else { continue };
                        eprintln!(
                            "Cast: sending {} ({})",
                            c.entries[id].id, c.entries[id].code
                        );
                        let code = c.entries[id].code.clone();
                        let cancel = Arc::new(AtomicBool::new(false));
                        let f = cancel.clone();
                        job = Some(Job {
                            cancel,
                            handle: Some(thread::spawn(move || {
                                kb.lock()
                                    .map_err(|_| anyhow::anyhow!("keyboard lock poisoned"))?
                                    .send(bind, &code, &w, &f)
                            })),
                        })
                    }
                    Action::Sync(force) => {
                        let row = cfg.row;
                        let m = matcher.clone();
                        p.notice = if force || row.is_none() {
                            "DRAG ROW"
                        } else {
                            "SCANNING"
                        }
                        .into();
                        scan = Some(thread::spawn(move || {
                            let (r, img) = pf::capture(row, force)?;
                            Ok((r, m.scan(&img)?))
                        }));
                    }
                    Action::Save(ids) => save_slots(c, &mut cfg, &mut p, ids)?,
                    Action::Teach(ids) => {
                        let result = (|| -> Result<_> {
                            let path = helldeck::hud::HudBook::path()?;
                            let mut book = helldeck::hud::HudBook::load(&path)?;
                            let mut names = vec![];
                            for (id, print) in p.confirmed_samples() {
                                let name = c.entries[id].id.as_str();
                                book.remember(name, print.clone());
                                names.push(name);
                            }
                            book.save(&path)?;
                            eprintln!("Learned confirmed HUD references: {}", names.join(", "));
                            Ok(())
                        })();
                        save_slots(c, &mut cfg, &mut p, ids)?;
                        match result {
                            Ok(()) => matcher = Arc::new(Matcher::local(c)),
                            Err(e) => {
                                p.notice = "TEACH ERR".into();
                                eprintln!("Loadout saved, HUD references not saved: {e:#}");
                            }
                        }
                    }
                }
            }
        }
        tray.show("Disconnected");
        drop(job);
        if stop.load(Ordering::SeqCst) {
            d.clear_all_button_images()?;
            d.flush()?;
            break;
        }
        drop(reader);
        drop(d);
        while !stop.load(Ordering::SeqCst) && deck_node().is_err() {
            thread::sleep(Duration::from_millis(350));
        }
    }
    Ok(())
}

#[cfg(test)]
mod focus_tests {
    use super::*;
    #[test]
    fn drop_transition_suppresses_input_without_revoking_arm() {
        let w = Window {
            pid: 42,
            address: "0xabc".into(),
            class: "steam_app_553850".into(),
            mapped: true,
            at: [0, 0],
            size: [1920, 1080],
        };
        let mut p = Panel::new([None; 4]);
        arm_panel(&mut p, None);
        assert!(p.armed, "Arm works before the game starts");
        assert!(!observe_focus(&mut p, None));
        assert_eq!(p.notice, "WAIT GAME");
        let other = Window {
            class: "terminal".into(),
            ..w.clone()
        };
        assert!(!observe_focus(&mut p, Some(&other)));
        let resized = Window {
            at: [12, 10],
            size: [2560, 1440],
            ..w.clone()
        };
        assert!(observe_focus(&mut p, Some(&resized)));
        assert_eq!(p.notice, "READY");
        let next = Window {
            pid: 43,
            ..w.clone()
        };
        assert!(
            observe_focus(&mut p, Some(&next)),
            "a relaunched game must not need another Arm"
        );
        assert!(p.armed);
        let c = Catalog::load().unwrap();
        p.press(14, &c); // explicit Pause still wins over focus changes
        assert!(!observe_focus(&mut p, Some(&w)));
    }
}
