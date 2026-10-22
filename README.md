# ReRAC for Android — Dual-Screen Mod

Android and AYN Thor dual-screen adaptation of **Ratchet & Clank (2002)**, based on the [ReRAC project](https://github.com/re-rac/rerac).

**[Download the current Android release (v1.0 Beta)](https://github.com/HighDrexler/rerac-Android-APK-and-Ayn-Thor-Dual-Screen-Mod-Beta-/releases/tag/v1.1)**

The current Android launcher and native build source are maintained on [update-channel3](https://github.com/HighDrexler/rerac-Android-APK-and-Ayn-Thor-Dual-Screen-Mod-Beta-/tree/update-channel3). See its [launcher source](https://github.com/HighDrexler/rerac-Android-APK-and-Ayn-Thor-Dual-Screen-Mod-Beta-/tree/update-channel3/android/launcher) and [branch update instructions](https://github.com/HighDrexler/rerac-Android-APK-and-Ayn-Thor-Dual-Screen-Mod-Beta-/blob/update-channel3/android/UPDATE_PROCESS.md).

## Android and dual-screen features

- Optional companion display for maps, menus, weapons, gadgets and quick selection, with gameplay continuing on the main screen.
- Touchscreen and analog-stick weapon Quick Select, including secondary-screen interaction.
- Optimized mobile control preset and optional Standard DualShock-style control layout.
- Reposition, resize and hide controls; save settings separately for each layout.
- Dedicated hold-to-Crouch and sequenced Crouch + Jump controls.
- Physical-controller support, configurable camera input, and adjustable rendering resolution.
- Android ISO/game-data importer, launcher, and mobile cutscene-processing improvements.

## Getting started

Install the APK from the ZIP linked above. In the launcher, import a supported original **USA v1.00 (SCUS-97199)** Ratchet & Clank ISO or compatible extracted game-data ZIP. Choose the rendering resolution and enable **Dual screen** in Display settings for a companion display (in Launcher 1.1, use **Mods** instead). Use **Setup** in-game to select and customize the mobile controls.

No copyrighted game content is supplied. Saves are stored separately from imported game data. Android gameplay and cutscene performance can vary with device and graphics drivers.

Original ReRAC project and license: [re-rac/rerac](https://github.com/re-rac/rerac). This adaptation is an unofficial community project.

## Native source on main

`main` retains the native source base and its build/test inputs. Its Cargo version describes that source, not the current Android launcher. Build the runtime and extractor with `cargo build --release --locked -p rc-engine -p rc-extract --bins`; the runtime also needs the shaders in `crates/rc-engine/assets`.

See [game-data setup](docs/workflows/game-data.md), [testing](docs/workflows/testing.md), [native packaging](tools/package/README.md) and [runtime switches](docs/workflows/dev-switches.md). File-format references are under `docs/formats`. Historical plans, test-request logs and retired investigation tools remain available in Git history.
