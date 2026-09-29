# DualBridge

**Use your PlayStation controllers (DualShock 4 & DualSense) on Windows and macOS — with a clear interface, multi-controller support, and deep lightbar control.**

> **Status: early development.** Nothing is usable yet. Follow the [roadmap](docs/ROADMAP.md) to see what's planned and what's done.

DualBridge is an independent, open-source project inspired by DS4Windows. It is not affiliated with DS4Windows, Sony, Microsoft, or Apple.

## Planned features

- **PS4 (DualShock 4) and PS5 (DualSense)** controllers, over USB and Bluetooth
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

## Contributing

Issues and pull requests are welcome. Since the maintainers can't test every controller and OS combination, **hardware test reports are especially valuable** — please include your controller model, connection type (USB/Bluetooth), and OS version.

## License

[GPL-3.0](LICENSE)

---

## Français

**DualBridge** permet d'utiliser les manettes PlayStation (DualShock 4 et DualSense) sur Windows et macOS, avec une interface simple, plusieurs manettes en même temps et beaucoup d'options pour la lumière.

> **Statut : en début de développement.** Rien n'est encore utilisable. Voir la [feuille de route](docs/ROADMAP.md).

Projet indépendant et open source inspiré de DS4Windows, sans lien avec DS4Windows, Sony, Microsoft ou Apple.
