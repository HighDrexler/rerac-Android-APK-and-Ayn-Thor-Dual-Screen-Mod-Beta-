# ReRAC Android — Launcher 1.1

Download `ReRAC-Android-1.1.apk.zip`, extract `ReRAC.apk`, and install it over the existing app. Keep the app installed. Beta18 and newer can instead use **Updates → Development → Check for updates**; beta17 needs one manual installation.

Version 1.1 (Android build 37) restores RAC1's native quick-select cursor, brightness, weapon name and ammo while preserving the working contact input from beta19. It removes the added Android circle/status overlay and superseded beta publication tooling.

This build uses the existing app package and signing certificate. Signed updates live only on `update-channel3`; no GitHub Release is used. The [build report](build.json) records the exact APK checksum and native source identity.

Validation passed: [signed APK and packaged companion tests](https://github.com/HighDrexler/rerac-Android-APK-and-Ayn-Thor-Dual-Screen-Mod-Beta-/actions/runs/39696747994), and [actual beta18 and beta19 upgrades through the live branch updater](https://github.com/HighDrexler/rerac-Android-APK-and-Ayn-Thor-Dual-Screen-Mod-Beta-/actions/runs/39698150330). Both upgrades installed the exact approved APK and preserved app data, the dual-screen setting and Development channel.

The Android input tests use a selection transport fixture; the native rendering tests use RAC1's production renderer. The owner confirmed physical Thor touch operation in beta19; a physical 1.1 run is not claimed by the automated checks.