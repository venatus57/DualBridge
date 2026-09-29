# Virtual controller: ViGEmBus status and alternatives

Most Windows games only read XInput (Xbox) controllers. DualBridge therefore
creates one virtual Xbox 360 controller per physical PlayStation controller.

## ViGEmBus

[ViGEmBus](https://github.com/nefarius/ViGEmBus) is a kernel-mode bus driver
that emulates Xbox 360 and DualShock 4 controllers. It is what DS4Windows,
Steam Input alternatives, and most similar tools use.

- **Status:** its author retired the project in 2023. The repository is
  archived, but the last release (v1.22.0) still installs and works on
  Windows 10 and 11, and it is signed by Microsoft, so it loads with Secure
  Boot enabled.
- **License:** BSD-3-Clause, so the installer can be redistributed unmodified
  with its license notice.
- **Risk:** no fixes for future Windows changes. If a Windows update breaks
  it, there is no upstream to fix it.

DualBridge talks to it through the [`vigem-client`](https://crates.io/crates/vigem-client)
crate (MIT), including rumble notifications from games.

## Alternatives considered

| Option | Pros | Cons |
|---|---|---|
| ViGEmBus (chosen) | Works everywhere today, signed, well known | Retired upstream |
| Steam Input | No driver needed | Only for games launched through Steam |
| Our own virtual HID driver (UMDF/KMDF) | Full control | Needs driver signing (EV certificate + Microsoft attestation), large effort |
| Windows.Gaming.Input injection APIs | Official | Not a real XInput device; many games ignore it |

The virtual controller code is isolated behind the `VirtualBackend` trait in
`crates/dualbridge-virtual`, so another backend can be added without touching
the rest of the app.

## HidHide

Games that understand PlayStation controllers would otherwise see both the
real controller and the virtual Xbox one. [HidHide](https://github.com/nefarius/HidHide)
is a filter driver that hides chosen devices from every application except an
allow list. DualBridge registers itself in that list, hides the controllers it
manages while "exclusive mode" is on (the default whenever HidHide is
installed), and makes them visible again when it closes.

Games launched through Steam can also get a second virtual pad from Steam
Input. If double input remains, turn off Steam Input for PlayStation
controllers, or for that game. It is driven through `HidHideCLI.exe`, which ships with the driver.

TODO before 1.0: confirm HidHide's redistribution terms (the installer bundles
it unmodified) and that its CLI flags (`--app-reg`, `--dev-hide`,
`--dev-unhide`, `--cloak-on`) are unchanged in the latest release.

## macOS

A virtual gamepad on macOS requires Apple's HID virtual device entitlement or
a DriverKit extension, both of which need Apple's approval. Most macOS games
already support PlayStation controllers through the Game Controller framework,
so DualBridge focuses on lighting, battery, and profiles there.
