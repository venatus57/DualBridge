# DualBridge roadmap

## Goals

1. Support DualShock 4 (PS4) and DualSense (PS5) controllers over USB and Bluetooth.
2. Handle several controllers at once, each with its own slot, color, and profile.
3. Make every game on Windows see the controller, through a virtual Xbox 360 (or DS4) controller.
4. Keep latency minimal: the software should add well under 1 ms between a HID report arriving and the virtual controller update.
5. Offer rich lightbar control that is easy to understand.
6. Be easier to use than DS4Windows: guided first-run setup, plain-language settings, sensible defaults.
7. Ship a Windows `.exe` installer (which installs the drivers) and a macOS `.dmg`.

## Architecture

```
crates/
  dualbridge-core/     Pure Rust, no OS dependencies, fully unit-tested on any OS
                       - input report parsing: DS4 USB/BT, DualSense USB/BT
                       - output report building: lightbar, rumble, player LEDs, mic LED,
                         adaptive triggers; Bluetooth CRC32
                       - normalized ControllerState model
                       - lighting effects engine (time -> color, deterministic)
                       - mapping engine: remaps, deadzones, curves, gyro-to-mouse
                       - profiles (serde, versioned on-disk format)
  dualbridge-hid/      Device discovery, hotplug, read/write via the `hidapi` crate;
                       a mock device backend for tests
  dualbridge-virtual/  Output backends behind one trait:
                       - Windows: ViGEmBus (Xbox 360 / DS4 targets) + HidHide
                       - macOS: keyboard/mouse output (CGEvent)
                       - Mock: records output for tests
app/
  src-tauri/           Tauri 2 backend: owns the controller manager, exposes commands/events
  ui/                  Frontend (Svelte + TypeScript + Vite)
```

### Latency rules

- One dedicated input thread per controller: blocking HID read, parse, map, push to the virtual controller, then read again. No allocation, locking against the UI, or logging on this path.
- Raise that thread's priority (Windows: MMCSS "Games" task via `AvSetMmThreadCharacteristics`).
- The UI never sits on the input path. It receives a throttled state snapshot (~60 Hz) through Tauri events.
- Lighting and rumble output run on a separate writer so output reports never delay input.
- Add a built-in latency meter (report-arrival-to-virtual-update time) that users can see in the app.

## Milestones

### M0 — Scaffold
- [x] Cargo workspace with the three crates and the Tauri app
- [x] CI (`.github/workflows/ci.yml`): `cargo fmt --check`, `clippy -D warnings`, and tests on Ubuntu, Windows, and macOS
- [x] Release workflow (`release.yml`) using `tauri-action`: builds the NSIS `.exe` and the `.dmg` on version tags and attaches them to a GitHub Release (pull requests and manual runs upload them as workflow artifacts)

### M1 — Controller protocols (dualbridge-core)
- [x] DS4 input parsing (USB report 0x01, BT report 0x11): sticks, buttons, triggers, touchpad (2 fingers), gyro/accel, battery/charging
- [x] DualSense input parsing (USB report 0x01, BT report 0x31): the same, plus mute button and trigger feedback status
- [x] Output reports: DS4 (USB 0x05, BT 0x11) and DualSense (USB 0x02, BT 0x31) with lightbar, rumble, player LEDs, mic LED, and adaptive triggers
- [x] Bluetooth CRC32 for output reports (and checked on input reports)
- [x] Gyro calibration from feature reports
- [x] Unit tests with synthesized report fixtures (`crates/dualbridge-core/tests/fixtures/`)
- [ ] Add fixtures captured from real controllers

### M2 — Devices (dualbridge-hid)
- [x] Enumerate by VID/PID: Sony 0x054C; DS4 v1 0x05C4, DS4 v2 0x09CC, DualSense 0x0CE6, DualSense Edge 0x0DF2
- [x] Detect USB vs Bluetooth
- [x] Hotplug (connect/disconnect) without restarting the app
- [x] Controller manager with up to 8 slots, stable slot assignment, and slot swapping
- [x] Battery level and charging state

### M3 — Lighting engine
- [x] Effects: static, breathing, rainbow cycle, color wave, strobe, battery gradient, low-battery pulse, per-slot default colors
- [x] Brightness control and a "lights off" option
- [x] DualSense player LEDs (slot number or custom pattern) and mic LED behavior
- [x] Per-profile and per-game lighting
- [x] Per-controller lighting (overrides the profile's)
- [x] Live preview in the UI

### M4 — Virtual controller (Windows)
- [x] ViGEmBus Xbox 360 target
- [ ] Optional ViGEmBus DS4 target
- [x] HidHide integration ("exclusive mode") so games don't see double input
- [x] Rumble passthrough from the game back to the controller
- [x] Latency meter
- [x] Check ViGEmBus's current upstream status (it was retired by its author) and evaluate alternatives before committing (see `docs/virtual-controller.md`)

### M5 — App and UI
- [x] Dashboard: one card per controller showing model, connection, battery, slot, color, and live input
- [x] Guided first-run setup: driver check and install, connect a controller, pick a color
- [x] Settings in plain language, with an "Advanced" section for power users
- [x] Profiles, with automatic per-game switching based on the foreground process (Windows)
- [x] Tray icon, start with the OS, minimize to tray
- [x] French and English localization

### M6 — Mapping
- [x] Button remapping to Xbox buttons
- [ ] Button remapping to keyboard and mouse
- [x] Stick and trigger deadzones and response curves
- [ ] Touchpad as mouse
- [ ] Gyro aiming (gyro to mouse or right stick)

### M6b — Nintendo controllers
- [x] Switch Pro Controller (USB and Bluetooth): setup sequence, full reports, stick calibration from SPI flash, HD rumble, player LEDs
- [ ] Joy-Con (single and paired) and the charging grip

### M7 — macOS
The shared code (lighting, battery, profiles) already builds for macOS; nothing here is verified on hardware yet.

- [ ] HID access, handling the Input Monitoring permission
- [ ] Lighting, battery, and profiles
- [ ] Keyboard/mouse mapping (Accessibility permission)
- [ ] Investigate a virtual gamepad: requires Apple's HID virtual-device entitlement or DriverKit, and may not be feasible

### M8 — Installers and release
- [x] NSIS installer hooks: install ViGEmBus and HidHide if missing
- [ ] Confirm HidHide's redistribution terms (ViGEmBus is BSD-3-Clause)
- [x] `.dmg` for macOS
- [x] Uninstaller removes app data on request (built into Tauri's NSIS uninstaller)
- [ ] Optional: code signing (Windows certificate, Apple Developer ID and notarization)

## Constraints

- **No hardware in development environments.** Cloud sessions and CI have no controllers attached. All protocol logic must be testable with fixtures and the mock backends. Anything that needs real hardware is verified by the maintainer from CI-built artifacts, and PRs must say what was *not* tested on hardware.
- **Licensing.** DualBridge is GPL-3.0.
  - DS4Windows (GPL-3.0): code may be reused with attribution.
  - SDL's `SDL_hidapi_ps4.c` / `SDL_hidapi_ps5.c` (zlib): compatible.
  - Linux `hid-playstation.c` is GPL-2.0-only: read it for protocol facts, but **do not copy its code**.
