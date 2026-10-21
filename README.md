# ReRAC for Android — Dual-Screen Mod

Android and AYN Thor dual-screen adaptation of **Ratchet & Clank (2002)**, based on the [ReRAC project](https://github.com/re-rac/rerac).

**[Download the current Android release (v1.0 Beta)](https://github.com/HighDrexler/rerac-Android-APK-and-Ayn-Thor-Dual-Screen-Mod-Beta-/releases/tag/v1.0)**

See the [editable launcher source on dev](https://github.com/HighDrexler/rerac-Android-APK-and-Ayn-Thor-Dual-Screen-Mod-Beta-/tree/dev/android/launcher) and [release/update instructions](https://github.com/HighDrexler/rerac-Android-APK-and-Ayn-Thor-Dual-Screen-Mod-Beta-/blob/dev/android/RELEASE_PROCESS.md).

## Android and dual-screen features

- Optional companion display for maps, menus, weapons, gadgets and quick selection, with gameplay continuing on the main screen.
- Touchscreen and analog-stick weapon Quick Select, including secondary-screen interaction.
- Native-style PS2 menus with game icons, fonts and static-screen effects.
- Optimized mobile control preset and optional Standard DualShock-style control layout.
- Reposition, resize and hide controls; save settings separately for each layout.
- Dedicated hold-to-Crouch and sequenced Crouch + Jump controls.
- Physical-controller support, configurable camera input, and adjustable rendering resolution.
- Android ISO/game-data importer, launcher, and mobile cutscene-processing improvements.

## Getting started

Install the APK from the ZIP linked above. In the launcher, import a supported original **USA v1.00 (SCUS-97199)** Ratchet & Clank ISO or compatible extracted game-data ZIP. Choose the rendering resolution and enable **Dual screen** in Display settings for a companion display (in Launcher 1.1, use **Mods** instead). Use **Setup** in-game to select and customize the mobile controls.

No copyrighted game content is supplied. Saves are stored separately from imported game data. Android gameplay and cutscene performance can vary with device and graphics drivers.

## Release integrity

- Published v1.0 ZIP SHA-256: `327e92b042e135dba115f332104ee947978d6aca9b3a4fa56c8e48e29bf623cc`
- Signed v16 APK SHA-256: `cb13360fc1796e2c1e9ef00029eacbb853bd8e966837140878bd65df55f3b477`
- Android application ID: `org.rerac.android.uipreview` (retained for in-place updates).
- Launcher 1.1 beta hashes are included with its prerelease assets.

## Source status

This branch retains the original ReRAC engine source and reviewed Android workflow definitions. The dev branch contains the editable Launcher 1.1 recovery project, regression tests and pinned-v16 APK rebuild path. The complete native Android port is **not yet reconstructed as an independently rebuildable source tree**. The published v16 APK remains the native baseline. Do not assume that compiling the Rust workspace reproduces its Android libraries or that a workspace version bump updates the packaged native game.

Original ReRAC project and license: [re-rac/rerac](https://github.com/re-rac/rerac). This adaptation is an unofficial community project.
