# DualBridge

**Use your PlayStation controllers (DualShock 4 & DualSense) on Windows and macOS — with a clear interface, multi-controller support, and deep lightbar control.**

> **Status: early development, not yet tested on real controllers.** The first builds are available from the [Installers workflow](https://github.com/venatus57/DualBridge/actions/workflows/release.yml) (open the latest run, then *Artifacts*). See the [roadmap](docs/ROADMAP.md) for what's done and the [hardware test checklist](docs/hardware-testing.md) if you want to help.

DualBridge is an independent, open-source project inspired by DS4Windows. It is not affiliated with DS4Windows, Sony, Microsoft, or Apple.

## Planned features

- **PS4 (DualShock 4) and PS5 (DualSense)** controllers, over USB and Bluetooth — and the **Nintendo Switch Pro Controller**
- **Several controllers at once** — each one gets its own slot, color, and profile
- **Works in every game on Windows** by exposing a virtual Xbox controller
- **Minimal latency** — a dedicated high-priority input path, with a target of under 1 ms added by the software
- **Lightbar control with lots of options** — static color, breathing, rainbow, battery-level color, per-game profiles, DualSense player LEDs and mic LED
- **Simple interface** — a guided first-run setup and plain-language settings
- **Button remapping**, deadzones, trigger and stick curves, touchpad-as-mouse, and gyro aiming
- **Installers** — a `.exe` for Windows (installs the required drivers for you) and a `.dmg` for macOS

## Platform notes

| | Windows | macOS |
|---|---|---|
| Lightbar, battery, profiles | Planned | Planned |
| Virtual Xbox controller (every game) | Planned (ViGEmBus driver) | Not planned — requires Apple approval |
| Keyboard / mouse mapping | Planned | Planned |

On macOS, most games already support PS4 and PS5 controllers natively; DualBridge adds lighting, profiles, battery info, and keyboard/mouse mapping on top.

Release builds are not code-signed yet, so Windows SmartScreen and macOS Gatekeeper will show a warning the first time you run them.

## Tech stack

- **Core:** Rust (controller protocols, lighting engine, low-latency input loop)
- **App:** [Tauri 2](https://tauri.app) with a web-based UI
- **Builds:** GitHub Actions produce the Windows and macOS installers

## Development

Requirements: [Rust](https://rustup.rs) (stable), [Node.js](https://nodejs.org) 22, and on Linux the
[Tauri system libraries](https://tauri.app/start/prerequisites/) plus `libudev-dev`.

```sh
cd app
npm install
npm run tauri dev                     # run the app
DUALBRIDGE_DEMO=1 npm run tauri dev   # with two simulated controllers
npm run dev                           # UI only, in a browser, simulated backend
npm run tauri build                   # build the installer for this OS
```

From the repository root:

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Layout:

| Path | What it is |
|---|---|
| `crates/dualbridge-core` | Controller protocols, lighting engine, mapping, profiles. No OS dependencies. |
| `crates/dualbridge-hid` | Device discovery, hotplug, per-controller input/output threads, mock backend |
| `crates/dualbridge-virtual` | Virtual Xbox controller (ViGEmBus), HidHide, mock backend |
| `app/src-tauri` | Tauri 2 desktop app: engine thread, commands, tray, NSIS installer hooks |
| `app/ui` | Svelte + TypeScript interface (French and English) |

## Contributing

Issues and pull requests are welcome. Since the maintainers can't test every controller and OS combination, **hardware test reports are especially valuable** — please include your controller model, connection type (USB/Bluetooth), and OS version.

## License

[GPL-3.0](LICENSE)

---

## Français

**DualBridge** permet d'utiliser les manettes PlayStation (DualShock 4 et DualSense) et le Pro Controller de la Switch sur Windows et macOS, avec une interface simple, plusieurs manettes en même temps et beaucoup d'options pour la lumière.

> **Statut : en début de développement, pas encore testé sur de vraies manettes.** Les premières versions sont disponibles dans le [workflow Installers](https://github.com/venatus57/DualBridge/actions/workflows/release.yml) (dernière exécution, puis *Artifacts*). Voir la [feuille de route](docs/ROADMAP.md).

Projet indépendant et open source inspiré de DS4Windows, sans lien avec DS4Windows, Sony, Microsoft ou Apple.
