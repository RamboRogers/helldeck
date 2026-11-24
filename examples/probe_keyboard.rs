//! Local Hyprland diagnostic: create only the virtual device, never emit an event.
use anyhow::{Result, ensure};
use helldeck::platform::{Keyboard, bindings};
use std::{
    process::Command,
    time::{Duration, Instant},
};
fn main() -> Result<()> {
    let preset = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "home-arrows".into());
    let _keyboard = Keyboard::new(bindings(&preset)?)?;
    let start = Instant::now();
    loop {
        let out = Command::new("hyprctl").args(["-j", "devices"]).output()?;
        let d: serde_json::Value = serde_json::from_slice(&out.stdout)?;
        if d["keyboards"].as_array().is_some_and(|v| {
            v.iter().any(|x| {
                x["name"]
                    .as_str()
                    .is_some_and(|n| n.contains("helldeck-stratagem-keys"))
            })
        }) {
            println!("Hyprland registered {preset} keyboard (zero keys emitted)");
            return Ok(());
        }
        ensure!(
            start.elapsed() < Duration::from_secs(3),
            "virtual keyboard not registered by Hyprland"
        );
        std::thread::sleep(Duration::from_millis(70));
    }
}
