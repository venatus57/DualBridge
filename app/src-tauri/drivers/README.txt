Driver installers bundled with the Windows installer
====================================================

The release workflow (.github/workflows/release.yml) downloads these files
here before building, and the NSIS installer offers to run them:

  ViGEmBus_Setup.exe  - https://github.com/nefarius/ViGEmBus
                        Virtual gamepad bus driver. BSD-3-Clause.
                        Copyright (c) Nefarius Software Solutions e.U.
  HidHide_Setup.exe   - https://github.com/nefarius/HidHide
                        HID device hiding filter driver.
                        Copyright (c) Nefarius Software Solutions e.U.

Both are third-party software distributed unmodified under their own
licenses. They are not part of DualBridge and are not covered by its
GPL-3.0 license. The installers are not committed to this repository.
