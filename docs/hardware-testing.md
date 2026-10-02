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

## Updates
- [ ] With a newer release published on GitHub, the "new version" banner appears a few seconds after start; "Install" downloads it, starts the installer and closes DualBridge; the new version starts afterwards
- [ ] Settings > About > "Check now" says "up to date" on the latest version

## Connection

For each of DualShock 4 (v1 / v2) and DualSense (and Edge if available):

- [ ] USB: the controller appears within a second or two, with the right
      model and "USB".
- [ ] Bluetooth: same, with "Bluetooth".
- [ ] Unplug / turn off: the card disappears; plugging back gives the same slot.
- [ ] Two or more controllers at once get slots 1, 2, 3... and different colors.
- [ ] Swapping slots with the arrows works.

- [ ] "Switch off" on a Bluetooth DS4 / DualSense turns it off and it leaves the app; "Switch all off" (page and tray menu) does every wireless one; the button is disabled on USB
- [ ] Battery saver turns every lightbar off (and back on); "switch off after N minutes" switches an untouched Bluetooth controller off, not one in use
- [ ] With DS4Windows (or DSX) running, the red warning appears within a few seconds and goes away once it is closed

## Input

- [ ] The live view matches sticks, triggers, buttons, D-pad, touchpad fingers.
- [ ] Bluetooth reports are "full" (gyro moves, battery shows a real value).
- [ ] Battery percentage and charging icon are plausible.
- [ ] Windows "Set up USB game controllers" (`joy.cpl`) shows an Xbox 360
      controller that follows the PlayStation controller.
- [ ] A game using XInput works; vibration from the game reaches the controller.
- [ ] With HidHide installed (exclusive mode is on by default), games no
      longer see the PlayStation controller: in a game that supports both
      kinds (Brawlhalla, for example) each player gets exactly one controller.
- [ ] Windows asks for administrator permission once when a new controller
      connects, and not again on the next launch.
- [ ] Turning exclusive mode off makes the PlayStation controller visible again.
- [ ] The latency shown on the card stays well under 1 ms on average.
- [ ] "Sharp directions" on the left stick: in Brawlhalla, side attacks while running never come out as down attacks; diagonals only at real diagonals; nothing below the push threshold

## Lighting

- [ ] The lightbar follows each effect (solid, breathing, rainbow, cycle,
      strobe, battery, off) and the brightness slider.
- [ ] DualSense: the blue startup light is replaced by our color; player LEDs
      show the slot number; the mic LED toggles with the mute button.
- [ ] Low-battery warning pulses red below the threshold.

## DualSense extras

- [ ] Adaptive trigger effects (resistance, gun trigger, vibration) are felt
      on L2/R2 over USB and Bluetooth.

## Switch Pro Controller
- [ ] Detected over USB and over Bluetooth (pairing: hold the sync button on top)
- [ ] USB: the controller is used over the cable (handshake), not Bluetooth; plugging it in while paired shows one controller, not two
- [ ] Sticks centered at rest and reaching the edges (calibration read from the controller)
- [ ] Buttons by position: bottom (B) = Xbox A, right (A) = Xbox B, left (Y) = X, top (X) = Y; ZL/ZR as full triggers; Home = Guide; Capture = Back
- [ ] Battery level shown; charging shown on the cable
- [ ] Player LEDs show the controller number; "Identify" blinks them
- [ ] Rumble in a game is felt, keeps going during long effects, and stops when it should
- [ ] "Switch off" turns the Pro Controller off over Bluetooth

## macOS

- [ ] The `.dmg` opens; the app starts after allowing it in Privacy & Security.
- [ ] macOS asks for Input Monitoring; after allowing, controllers appear.
- [ ] Lighting and battery work over USB and Bluetooth.
