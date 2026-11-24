use ksni::{MenuItem, ToolTip, Tray, blocking::TrayMethods, menu::StandardItem};
use std::{
    process::Command,
    sync::{
        Arc, LazyLock,
        atomic::{AtomicBool, Ordering},
    },
};

pub const CREDIT: &str = "Matthew Rogers · @matthewrogers";
static ICON: LazyLock<ksni::Icon> = LazyLock::new(|| {
    let image = image::load_from_memory(include_bytes!("../assets/brand/helldeck-64.png"))
        .expect("bundled tray PNG")
        .into_rgba8();
    let (w, h) = image.dimensions();
    let mut data = image.into_raw();
    for px in data.as_chunks_mut::<4>().0 {
        px.rotate_right(1);
    }
    ksni::Icon {
        width: w as i32,
        height: h as i32,
        data,
    }
});
struct Indicator {
    state: &'static str,
    stop: Arc<AtomicBool>,
}
impl Tray for Indicator {
    fn id(&self) -> String {
        "helldeck".into()
    }
    fn title(&self) -> String {
        format!("HellDeck — {}", self.state)
    }
    fn icon_name(&self) -> String {
        "helldeck".into()
    }
    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        vec![ICON.clone()]
    }
    fn tool_tip(&self) -> ToolTip {
        ToolTip {
            icon_name: self.icon_name(),
            title: self.title(),
            description: format!("Stream Deck MK.2 · {CREDIT}"),
            ..Default::default()
        }
    }
    fn activate(&mut self, _x: i32, _y: i32) {
        let _ = Command::new("notify-send")
            .args([
                "--app-name=HellDeck",
                "--icon=helldeck",
                &self.title(),
                CREDIT,
            ])
            .status();
    }
    fn menu(&self) -> Vec<MenuItem<Self>> {
        vec![
            StandardItem {
                label: self.title(),
                enabled: false,
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: CREDIT.into(),
                enabled: false,
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Quit HellDeck".into(),
                activate: Box::new(|s: &mut Self| s.stop.store(true, Ordering::SeqCst)),
                ..Default::default()
            }
            .into(),
        ]
    }
}
pub struct TrayState {
    handle: Option<ksni::blocking::Handle<Indicator>>,
    state: &'static str,
}
impl TrayState {
    pub fn start(stop: Arc<AtomicBool>) -> Self {
        let state = "Starting";
        let handle = Indicator { state, stop }
            .assume_sni_available(true)
            .spawn()
            .map_err(|e| eprintln!("Tray unavailable; deck still works: {e}"))
            .ok();
        Self { handle, state }
    }
    pub fn show(&mut self, state: &'static str) {
        if self.state != state {
            self.state = state;
            if let Some(h) = &self.handle {
                h.update(move |tray| tray.state = state);
            }
        }
    }
}
