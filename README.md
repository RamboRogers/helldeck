# HellDeck

![HellDeck command pod](assets/brand/helldeck-64.png)

**Stop fat-fingering Reinforce while a Charger is breathing down your neck.**

HellDeck turns an **Elgato Stream Deck MK.2** into a physical Helldivers 2 stratagem panel on Linux. Tap one glowing key and HellDeck types the stratagem code for you — you still aim, throw, and take the credit for Managed Democracy.

![HellDeck running on an Elgato Stream Deck MK.2](docs/helldeck-mk2.jpg)

By **Matthew Rogers** · [@matthewrogers on X](https://x.com/matthewrogers)

[What it does](#what-it-does) · [Setup](#setup) · [First dive](#first-dive) · [Troubleshooting](#troubleshooting) · [Advanced](#advanced)

---

## What it does

- **One-tap loadouts:** Your 4 equipped stratagems sit across the top row.
- **Always-ready essentials:** Reinforce, Resupply, Hellbomb, and SOS Beacon stay pinned right below — plus 12 mission objectives under **Missions**.
- **HUD Auto-Sync:** Tap **Sync** in-game to scan your 4 equipped icons onto the deck. Runs locally on the CPU — no cloud, no GPU, no model downloads.
- **Physical safety switch:** Starts paused. Tap **Arm** on the deck to go live; if you Alt-Tab out of Helldivers 2, it holds input (`WAIT GAME`) until the game is focused again.
- **100% local & offline:** No HTTP requests, no game memory reading, no anti-cheat hooks, and no saved screenshots. The desktop launcher even locks the service down to local Unix/netlink sockets only.

| Loadout 1 | Loadout 2 | Loadout 3 | Loadout 4 | **Sync** |
| :---: | :---: | :---: | :---: | :---: |
| Reinforce | Resupply | Hellbomb | SOS Beacon | **Missions** |
| **Edit** | **Set Row** | — | *Status* | **Arm / Pause** |

![Mission shortcuts page on the 15-key MK.2](docs/missions.png)

---

## Setup

**Supported rig:** x86-64 Linux running **Hyprland** (Wayland) — tested on Omarchy / Arch — with one 15-key **Elgato Stream Deck MK.2** (`0fd9:0080`). *(Other compositors and Stream Deck models don't have a live-game backend yet, though you can still preview layouts without hardware.)*

### 1. Host packages (Arch / Omarchy)

```sh
sudo pacman -S --needed hyprland grim slurp coreutils systemd-libs desktop-file-utils libnotify
```

*(Building from source? Also install `rust base-devel pkgconf`. A StatusNotifier tray host is optional for the tray icon.)*

### 2. Download or build

**Prebuilt x86-64 release (fastest)** — grabs the [latest release](https://github.com/RamboRogers/helldeck/releases/latest) with `target/release/helldeck` ready to run:

```sh
curl -fL https://github.com/RamboRogers/helldeck/releases/latest/download/helldeck-0.1.0-x86_64-linux.tar.gz | tar -xz && cd helldeck-0.1.0-x86_64-linux
```

**Or build from source** (offline-ready, crates vendored in `vendor/`):

```sh
git clone https://github.com/RamboRogers/helldeck.git && cd helldeck && cargo build --release --frozen
```

### 3. Check your deck & permissions

Close any other Stream Deck software, then run:

```sh
./target/release/helldeck doctor
```

You want to see your MK.2 detected and `/dev/uinput writable: true`.

<details>
<summary><strong>Permission error or missing <code>/dev/uinput</code>?</strong> Click for udev setup</summary>

Load `uinput` if missing (`sudo modprobe uinput`), review [`desktop/70-helldeck.rules`](desktop/70-helldeck.rules), and install it so your normal desktop user can access the MK.2 and virtual keyboard (don't run HellDeck with `sudo`):

```sh
sudo install -m 0644 desktop/70-helldeck.rules /etc/udev/rules.d/70-helldeck.rules
sudo udevadm control --reload-rules
sudo udevadm trigger --action=add --subsystem-match=misc --sysname-match=uinput
```

Unplug and reconnect the deck, then re-run `./target/release/helldeck doctor`.

> **Note:** `/dev/uinput` permits seat-wide virtual keyboard input — only grant it on a trusted desktop.

</details>

### 4. Launch it

Install the desktop entry and start the background service:

```sh
./tools/install-desktop
./desktop/helldeck-launch
```

- On **Omarchy**, press **Super + Space**, type **HellDeck**, and pick the gold command pod.
- There's no app window — HellDeck lives on your Stream Deck and in the system tray. Quit from the tray or run `systemctl --user stop helldeck.service`.
- Keep the project folder where it is after running `install-desktop` (the launcher symlinks into it).

---

## First dive

### 1. Match your in-game bindings

HellDeck defaults to **Home (Press) + Arrow keys** so you can call stratagems on the move.

Prefer **Left Ctrl (Hold) + WASD**? Set `"preset": "ctrl-wasd"` in `~/.config/helldeck/config.json` before launching (a new file can be just `{"preset": "ctrl-wasd"}`).

### 2. Sync your loadout

1. Open your loadout or mission screen so your 4 equipped icons are visible.
2. Tap **Sync** on the deck and drag a tight box around just those 4 icons (leave out names, boosters, and teammates).
3. Your top row updates automatically. Future syncs reuse that box — tap **Set Row** if you move your HUD or change resolution (`Esc` cancels).

### 3. Arm and drop

Tap **Arm** on the bottom-right key:

- **`WAIT GAME`** — armed, waiting for Helldivers 2 to be focused.
- **`READY`** — live. Tap a stratagem key to type the code.
- **`PAUSED`** — safe mode. HellDeck always starts paused, discards presses outside the game, and stops immediately when you tap **Pause**.

### 4. Fixing a missed icon (Edit & Teach)

- **Quick swap:** Tap **Edit** anytime to set any slot by hand.
- **Review screen:** If Sync hits a blank slot, tie, or duplicate, it opens **Review**. Tap a slot to pick the right icon, then hit **Apply** (or **Use Found** to keep just the recognized slots for this session without overwriting your saved profile).
- **Teach it:** Fixed a stubborn icon in Review? Hit **Teach** instead of **Apply** to save a local fingerprint for the slots you picked (no screenshots saved). To clear learned fingerprints, stop HellDeck and delete `~/.config/helldeck/hud-v1.json`.

---

## Troubleshooting

| Symptom | What to try |
| --- | --- |
| **MK.2 missing or busy** | Connect one MK.2 and close competing deck managers. Don't run two HellDeck instances at once. |
| **Stuck on `WAIT GAME`** | Focus Helldivers 2 and check `hyprctl -j activewindow`. Accepted window classes are `steam_app_553850` and `helldivers2.exe`. |
| **Sync misses a slot** | Re-drag tightly with **Set Row**, or fix it with **Edit** / **Teach**. Check `journalctl --user -u helldeck.service` for per-slot match scores. |
| **No tray icon** | Check `systemctl --user status helldeck.service`. The physical deck still works without a StatusNotifier tray host. |
| **Missing library / glibc error** | Build from source on that machine with `cargo build --release --frozen` (prebuilt binaries target x86-64 GNU/Linux). |

**Try it without hardware:**

```sh
./target/release/helldeck layout missions             # print the panel layout
./target/release/helldeck dry-run hellbomb            # preview the key sequence
./target/release/helldeck preview missions.png missions
./target/release/helldeck recognize cropped-row.png   # test Sync on a cropped PNG
./target/release/helldeck catalog                     # list all 109 IDs and codes
```

---

## Advanced

<details>
<summary><strong>Offline verification, packaging & artwork rebuilds</strong></summary>

- **Zero-network build check** (Linux with unprivileged user namespaces):
  ```sh
  check=$(mktemp -d)
  unshare -Urn env CARGO_HOME="$check/cargo" CARGO_TARGET_DIR="$check/target" cargo build --release --frozen
  unshare -Urn env CARGO_HOME="$check/cargo" CARGO_TARGET_DIR="$check/target" cargo test --all-targets --frozen
  ```
- **Verify socket sandbox:** `systemctl --user show helldeck.service -p RestrictAddressFamilies` should list `AF_UNIX` and `AF_NETLINK` only.
- **Build a portable x86-64 bundle:** run `./tools/package-linux` to write a `.tar.gz` and `.sha256` under `dist/`.
- **Rebuild bundled artwork & catalog:** `python3 tools/import_assets.py` (catalog + atlas from `assets/source/` and `assets/catalog-overlay.json`) and `python3 tools/build_arm_keys.py` (Arm/Pause keys). Both require Pillow and `rsvg-convert` (`python-pillow`, `librsvg`).
- **Bundled data:** 109 stratagems ([`assets/catalog.json`](assets/catalog.json), [`assets/atlas.png`](assets/atlas.png)), 111 source SVGs ([`assets/source/icons/`](assets/source/icons/)), brand art ([`assets/brand/`](assets/brand/)), and vendored Cargo crates ([`vendor/registry/`](vendor/registry/)).

</details>

---

## Credits

HellDeck is an independent fan project by **Matthew Rogers** ([@matthewrogers](https://x.com/matthewrogers)). Not affiliated with Arrowhead, Sony, or Elgato. Check the game's rules before using macros.

Community icon artists and stratagem code tables are credited in [`assets/SOURCES.md`](assets/SOURCES.md). For bug reports, include `./target/release/helldeck --version`, your compositor, deck model, and the failing command.
