# DualBridge

**Use your PlayStation (DualShock 4, DualSense) and Switch Pro controllers in every PC game — with a clear interface, several controllers at once, and deep lightbar control.**

## Download

**[⬇ Download the latest version](https://github.com/venatus57/DualBridge/releases/latest)**: on Windows, take `DualBridge_x.y.z_x64-setup.exe`; on macOS, `DualBridge_x.y.z_universal.dmg`.

The Windows installer offers to install the two drivers DualBridge needs: ViGEmBus (virtual Xbox controller) and HidHide (stops games from seeing each controller twice). Builds are not code-signed yet: on Windows click "More info" then "Run anyway". Once installed, DualBridge tells you when a new version is out and installs it in one click.

DualBridge is an independent, open-source project inspired by DS4Windows. It is not affiliated with DS4Windows, Sony, Nintendo, Microsoft, or Apple.

## Features

- **PS4 (DualShock 4), PS5 (DualSense, DualSense Edge) and Switch Pro controllers**, over USB and Bluetooth
- **Works in every game on Windows**: each controller appears as an Xbox controller, and the real one is hidden so games never see it twice
- **Up to 8 controllers at once**, mixed models, each with its own slot, color and profile
- **Low latency**: a dedicated high-priority input thread per controller
- **Lighting**: per-controller colors, breathing, rainbow, color cycle, strobe, battery level, low-battery warning, DualSense player and mic LEDs
- **Profiles**: button remapping, stick dead zones and curves, **sharp directions with rapid trigger** (keyboard-like precision for fighting and platform games), trigger settings, DualSense adaptive triggers, vibration; switched automatically per game
- **Battery**: level per controller, battery saver, switch off from the app, automatic switch-off when idle
- **Automatic updates**, start with Windows, tray icon, French and English

Some features still need testing on more controllers; reports are welcome (see the [hardware test checklist](docs/hardware-testing.md)). On macOS there is no virtual controller (it needs Apple's approval), but most games support these controllers natively.

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
DUALBRIDGE_DEMO=1 npm run tauri dev   # with simulated controllers
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

**[⬇ Télécharger la dernière version](https://github.com/venatus57/DualBridge/releases/latest)** : sur Windows, prenez `DualBridge_x.y.z_x64-setup.exe` ; sur Mac, `DualBridge_x.y.z_universal.dmg`.

L'installateur Windows propose d'installer les deux pilotes nécessaires : ViGEmBus (manette Xbox virtuelle) et HidHide (évite que les jeux voient chaque manette en double). L'application n'est pas encore signée : sur Windows, cliquez sur « Informations complémentaires » puis « Exécuter quand même ». Ensuite, DualBridge vous prévient quand une nouvelle version sort et l'installe en un clic.

Si une manette apparaît en double dans un jeu, fermez DS4Windows ou DSX s'ils sont ouverts (DualBridge vous prévient), et désactivez Steam Input pour les manettes PlayStation dans Steam.

Projet indépendant et open source inspiré de DS4Windows, sans lien avec DS4Windows, Sony, Nintendo, Microsoft ou Apple.
