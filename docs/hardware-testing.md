# Hardware test checklist

Development and CI run without controllers, so these checks are done by hand
on the installers built by the "Installers" workflow (Actions tab → latest run
→ Artifacts). Please report results in an issue with your controller model,
connection (USB / Bluetooth), and OS version.

## Install (Windows)

- [ ] The `.exe` installer runs (click "More info → Run anyway" on the
      SmartScreen warning).
- [ ] It offers to install ViGEmBus when missing, and ViGEmBus installs.
- [ ] It offers HidHide; installing it works (may need a restart).
- [ ] The first-run assistant opens and shows the driver as installed.

## Connection

For each of DualShock 4 (v1 / v2) and DualSense (and Edge if available):

- [ ] USB: the controller appears within a second or two, with the right
      model and "USB".
- [ ] Bluetooth: same, with "Bluetooth".
- [ ] Unplug / turn off: the card disappears; plugging back gives the same slot.
- [ ] Two or more controllers at once get slots 1, 2, 3... and different colors.
- [ ] Swapping slots with the arrows works.

## Input

- [ ] The live view matches sticks, triggers, buttons, D-pad, touchpad fingers.
- [ ] Bluetooth reports are "full" (gyro moves, battery shows a real value).
- [ ] Battery percentage and charging icon are plausible.
- [ ] Windows "Set up USB game controllers" (`joy.cpl`) shows an Xbox 360
      controller that follows the PlayStation controller.
- [ ] A game using XInput works; vibration from the game reaches the controller.
- [ ] With exclusive mode on, games no longer see the PlayStation controller.
- [ ] The latency shown on the card stays well under 1 ms on average.

## Lighting

- [ ] The lightbar follows each effect (solid, breathing, rainbow, cycle,
      strobe, battery, off) and the brightness slider.
- [ ] DualSense: the blue startup light is replaced by our color; player LEDs
      show the slot number; the mic LED toggles with the mute button.
- [ ] Low-battery warning pulses red below the threshold.

## DualSense extras

- [ ] Adaptive trigger effects (resistance, gun trigger, vibration) are felt
      on L2/R2 over USB and Bluetooth.

## macOS

- [ ] The `.dmg` opens; the app starts after allowing it in Privacy & Security.
- [ ] macOS asks for Input Monitoring; after allowing, controllers appear.
- [ ] Lighting and battery work over USB and Bluetooth.
