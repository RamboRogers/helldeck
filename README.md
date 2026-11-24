# HellDeck

**Why:** When your squad needs Reinforce, every second counts. Tap one key and HellDeck enters the stratagem code for you. You still aim and throw.

![HellDeck command pod](assets/brand/helldeck-64.png)

HellDeck turns an **Elgato Stream Deck MK.2** into a Helldivers 2 stratagem panel on Linux. Your four equipped stratagems sit on the top row, with Reinforce, Resupply, Hellbomb and SOS Beacon right below.

**Matthew Rogers** · [@matthewrogers on X](https://x.com/matthewrogers)

[Quick start](#quick-start) · [Device access](#device-access) · [First dive](#first-dive) · [Offline verification](#offline-verification) · [Troubleshooting](#troubleshooting)

![HellDeck on a Stream Deck MK.2 with Gatling, Flame, Rocket and Autocannon sentries across the top row](docs/helldeck-mk2.jpg)

## Quick start

The supported setup is **x86-64 Linux with Hyprland**, a local Wayland session and one 15-key MK.2 (`0fd9:0080`). Omarchy and Arch with Hyprland use the same steps. Other Linux distributions need the equivalent tools below. KDE, GNOME, X11 and other Stream Deck models do not have a live-game backend here. You can preview the panel without the game or a deck.

| What you want to do | What the host needs (Arch package names) |
| --- | --- |
| Run the panel and Sync | `hyprland`, `grim`, `slurp`, `psmisc` (`fuser`), `coreutils` (`timeout`), `systemd-libs` (`libudev`), glibc and the GCC runtime |
| Use the launcher and tray | A systemd user session, local session D-Bus, `desktop-file-utils`, `libnotify`; a StatusNotifier tray host to see the icon |
| Build from source | An installed Rust toolchain with Cargo (tested with 1.98.1), a C toolchain, `pkg-config` and libudev headers. On Arch: `rust` or an existing rustup toolchain, `base-devel`, `systemd-libs` |

1. **Copy or unpack the complete project** into a directory you will keep, then open a terminal there. Keep `.cargo/`, `vendor/` and `assets/` with it. The project supplies the Rust crates and artwork; it does not install OS packages, Rust, Steam or the game. On an air-gapped machine, provision missing host packages and their dependencies from trusted local media first.
2. **Build or use the included executable.** The offline x86-64 bundle contains `target/release/helldeck`; skip the build for that bundle. From a source-only copy, run:

   ```sh
   cargo build --release --frozen
   ```

3. **Try it without sending input**, then check the hardware. Close other deck managers and stop any running HellDeck instance before `doctor`.

   ```sh
   ./target/release/helldeck layout missions
   ./target/release/helldeck dry-run hellbomb
   ./target/release/helldeck doctor
   ```

   `doctor` should find the MK.2 and report `/dev/uinput writable: true`. An unfocused game is fine at this step. If access fails, see [Device access](#device-access).

4. **Install the launcher as your desktop user:**

   ```sh
   ./tools/install-desktop
   ./desktop/helldeck-launch
   ```

   On Omarchy you can also press **Super + Space**, type **HellDeck** and select the gold pod. The installer links to this folder, so don't move it afterward. The menu entry needs `~/.local/bin` on the launcher's `PATH`; the direct launcher command above does not. Both launch the same `helldeck.service`, paused, without enabling autostart. There is no separate app window. Quit from the tray or run `systemctl --user stop helldeck.service`.

## Device access

Run HellDeck as your normal desktop user. If `doctor` already reports writable access, **skip this section**. If `/dev/uinput` is missing, load the kernel module with `sudo modprobe uinput`. For a permission error on a systemd/logind desktop, inspect [the optional udev rules](desktop/70-helldeck.rules) before installing them:

```sh
sudo install -m 0644 desktop/70-helldeck.rules /etc/udev/rules.d/70-helldeck.rules
sudo udevadm control --reload-rules
sudo udevadm trigger --action=add --subsystem-match=misc --sysname-match=uinput
```

Unplug and reconnect the deck, then run `doctor` again from your active local session. The rules grant the active seat access to the matching Elgato HID device and to `/dev/uinput`. The latter permits **seat-wide keyboard input**; grant it only on a trusted desktop. Don't make all HID devices world-writable or run the panel with `sudo`.

## First dive

| Loadout 1 | Loadout 2 | Loadout 3 | Loadout 4 | Sync |
| --- | --- | --- | --- | --- |
| Reinforce | Resupply | Hellbomb | SOS Beacon | Missions |
| Edit | Set Row | — | Status | Arm / Pause |

1. **Match your game bindings.** The default is **Home (Press) + arrow keys**. If you use **Left Ctrl (Hold) + WASD**, set `"preset": "ctrl-wasd"` in `${XDG_CONFIG_HOME:-~/.config}/helldeck/config.json` before launching. A new file can contain just `{"preset":"ctrl-wasd"}`; if it exists, keep your saved slots and row. HellDeck doesn't change game settings.
2. **Sync your loadout.** With your four equipped icons visible in-game, press **Sync** and drag tightly around your row. Leave out names, boosters and teammates. Later syncs reuse the box. Use **Set Row** after moving your row or changing resolution or HUD scale. `Esc` cancels selection.
3. **Review uncertain matches.** Four confident, distinct matches update the deck together. Otherwise, tap a slot to choose an icon and press **Apply**, or **Cancel** to keep your loadout. **Use Found** takes only the recognized slots for this session without overwriting the saved profile. **Edit** lets you pick manually without a screenshot.
4. **Arm on the physical deck.** **WAIT GAME** means armed but not sending keys; **READY** means Helldivers 2 is focused. Presses outside the game are discarded, not queued. **Pause** stops input. A restart or reconnect starts paused. Focus is checked before each key-down, but Linux input is seat-wide: a focus switch between the check and the key-down is still possible.

**Missions** has 12 fixed shortcuts across two pages. Sync changes only the four loadout slots. The mission Hellbomb and equippable Portable Hellbomb are different entries. The game decides availability and cooldowns; HellDeck doesn't read game memory or interact with anti-cheat. Check the game's rules before using macros.

![Mission page on the 15-key MK.2: ten mission shortcuts above the page controls and Arm key](docs/missions.png)

## Offline verification

HellDeck makes no HTTP or HTTPS requests. It reads the catalog, key art, bitmap font and tray image from the executable. Sync captures only the selected row and matches it in memory; it neither saves nor uploads screenshots. The desktop launcher also restricts the service and its children to **Unix and kernel netlink sockets**. This leaves Wayland, D-Bus and device discovery working without IPv4 or IPv6 sockets. The restriction applies to HellDeck, not to Steam or Helldivers 2. Running the raw binary directly does not install the launcher's systemd restriction.

| Local data | Where to inspect it |
| --- | --- |
| 109 supported entries: 95 loadout, 12 mission, 2 utility | [Catalog](assets/catalog.json) and [72×72 icon atlas](assets/atlas.png). Each entry has a nonblank tile; the two drill missions share artwork. This is a catalog snapshot, not every stratagem in the game. |
| Rebuild inputs | [111 source SVGs](assets/source/icons/), [input table](assets/source/stratagems.md), [reviewed additions](assets/catalog-overlay.json). No upstream checkout required. |
| Launcher, tray, Arm and Pause | [Brand artwork](assets/brand/). The desktop installer links the local SVG and PNG; there are no remote badges or images in this README. |
| Linux build dependencies | [Local Cargo registry](vendor/registry/) and [Cargo configuration](.cargo/config.toml). The crate archives match `Cargo.lock`; Cargo fails offline if files are missing. URLs in the lockfile, credits and source notes are identifiers or links, not runtime fetches. |

To repeat the build check on Linux with unprivileged user namespaces enabled, start in the project root with Rust already installed:

```sh
check=$(mktemp -d)
unshare -Urn env CARGO_HOME="$check/cargo" CARGO_TARGET_DIR="$check/target" cargo build --release --frozen
unshare -Urn env CARGO_HOME="$check/cargo" CARGO_TARGET_DIR="$check/target" cargo test --all-targets --frozen
```

This uses a fresh cache and build directory in a network namespace with no external route. No deck is required. To inspect the running desktop unit's socket policy, use `systemctl --user show helldeck.service -p RestrictAddressFamilies`. Expect `AF_UNIX` and `AF_NETLINK` only (the order may differ). A unit started before this policy was installed needs to be stopped and launched again.

To make a bundle for another x86-64 GNU/Linux desktop, run `./tools/package-linux`. It writes a `.tar.gz` with the binary, sources, local crates and artwork, plus a `.sha256` file, under `dist/`. Transfer both files, run `sha256sum -c helldeck-*-x86_64-linux.tar.gz.sha256` in the destination directory containing that one release, then extract the archive and follow the quick start. The checksum checks the copy, not the publisher's identity.

For artwork maintenance, `python3 tools/import_assets.py` rebuilds the catalog and atlas; `python3 tools/build_arm_keys.py` rebuilds the Arm/Pause artwork. Both read the bundled inputs and write into `assets/`. Only these maintenance commands require Pillow and `rsvg-convert` (Arch: `python-pillow`, `librsvg`).

## Troubleshooting

| Symptom | What to check |
| --- | --- |
| MK.2 missing or busy | Connect one MK.2. Close competing deck managers; don't run a second controller alongside the service. |
| **WAIT GAME** never changes | Check `hyprctl -j activewindow` with the game focused. The accepted classes are `steam_app_553850` and `helldivers2.exe` (case-insensitive). |
| Sync misses a slot | Try **Set Row**, then **Edit** if needed. `journalctl --user -u helldeck.service` shows candidate scores and the decision, not screenshot pixels. Tests use rendered fixtures; matching on your particular HUD is not guaranteed. |
| Launcher starts but no tray icon appears | Check `systemctl --user status helldeck.service`. A desktop without a StatusNotifier host can still run the physical panel. |
| Prebuilt binary reports a missing library or glibc version | Build from the included source on that machine. The binary is for x86-64 GNU/Linux, not musl or ARM. |

For a hardware-free check, run `./target/release/helldeck preview missions.png missions`. `catalog` lists IDs and codes; `recognize cropped-row.png` runs the matcher on your own cropped PNG without changing your profile.

## Credits and support

HellDeck is an independent fan tool by **Matthew Rogers** ([@matthewrogers](https://x.com/matthewrogers)). For a bug report, include the output of `./target/release/helldeck --version`, your compositor, deck model and failing command. Don't include game screenshots or personal config unless needed.

The [source notes](assets/SOURCES.md) credit the community icon artists and input tables. Bundling the artwork does not grant redistribution rights to game art; third-party dependencies retain their own licenses. HellDeck is not affiliated with Arrowhead, Sony or Elgato.
