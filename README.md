# ReRAC for Android — Launcher and Dual-Screen Mod

Android and AYN Thor adaptation of Ratchet & Clank (2002), based on [ReRAC](https://github.com/re-rac/rerac).

**[Launcher 1.1 development test APKs](https://github.com/HighDrexler/rerac-Android-APK-and-Ayn-Thor-Dual-Screen-Mod-Beta-/releases/tag/launcher-v1.1.0-beta-test)** · [Existing v1.0 release](https://github.com/HighDrexler/rerac-Android-APK-and-Ayn-Thor-Dual-Screen-Mod-Beta-/releases/tag/v1.0)

## Launcher 1.1 test

Install `ReRAC-Launcher-1.1.0-beta.1.apk` over the existing v1.0 app. **Do not uninstall.** Back up important saves before trying a development build.

The test adds an Updates page with Stable/Development channels, verified in-launcher downloads, an Android installation prompt, a small bottom launcher/core version label, and a Mods manager. Only the existing **AYN Thor Dual-Screen Mod** is bundled. Its toggle takes effect at the next game launch. Choose Mods folder uses Android's file picker; external configuration/resolution-profile mods are supported, not arbitrary native-code or script mods.

To exercise the update process: open **Updates → Development → Check for updates → Download update → Install downloaded update**. Allow installations from ReRAC in Android Settings when prompted, return to the app and confirm installation. The feed offers beta.2, an otherwise equivalent build with a higher Android versionCode. Stable does not advertise this experimental test.

**These APKs still contain ReRAC 0.3.4/v16 native libraries. They do not include the upstream 0.3.5 native game update.** The footer and update notes show the actual versions, not the Rust workspace version.

## Preserved Android functionality

The original v16 native libraries, importer and game-control DEX files are preserved. Existing features include the optional companion-screen maps/menus/quick selection, touch and analog-stick Quick Select, configurable mobile and DualShock-style layouts, hold-to-Crouch and Crouch + Jump controls, physical-controller support, camera options and rendering-resolution selection. The launcher retains its existing styling.

For a new installation, import a supported USA v1.00 (SCUS-97199) Ratchet & Clank disc ISO or compatible extracted game-data ZIP. Game assets are not supplied. In Launcher 1.1, enable the second display through Mods; use Setup in-game to customize mobile controls.

## Build and test status

The editable launcher extension and regression tests are under `android/launcher`. The recovery build compiles those Java sources, rebuilds the launcher DEX/manifest and packages the pinned public v16 APK's other components without changing them. This is a maintainable launcher rebuild path, **not** a complete from-source native port rebuild.

The signed development candidates passed all 50 Java policy, persistence, mod and UI tests, plus signature/alignment/payload checks and actual Android 15 x86-64 emulator installs with two data-preserving upgrades. Real network updater tests have a separate workflow and evidence artifact. Physical AYN Thor dual-screen output, ARM64 gameplay, imported game data and cutscenes still need device testing.

The Rust workspace on dev includes the merged upstream 0.3.5 source. The original native Android port modifications/build project remain to be recovered and reconciled before compiling a new 0.3.5 Android engine. Do not assume that `cargo build` on this branch reproduces the native libraries in the APK.

See [release and update instructions](android/RELEASE_PROCESS.md). Dev commits become user updates only after a successful build, signing, validation, release-asset publication and signed channel update. Private signing keys are not stored in GitHub or CI.

## Integrity

Package ID: `org.rerac.android.uipreview`, unchanged for in-place updates.

| APK | SHA-256 |
| --- | --- |
| Launcher 1.1.0-beta.1 | `09154d2abfc5d079e2b8d32418b2a91216f78e9be372ba708a8baaa0cd3eda0d` |
| Launcher 1.1.0-beta.2 | `474e2e3a75a334aff2691128e8520291ab59af3fcb2fc3e6532d9f5a6700444f` |
| Original v16/v1.0 | `cb13360fc1796e2c1e9ef00029eacbb853bd8e966837140878bd65df55f3b477` |

All three use the original release certificate. This is an unofficial community adaptation; the original project and ISC license remain credited to ReRAC.
