# ReRAC for Android — Launcher and Dual-Screen Mod

Android and AYN Thor adaptation of Ratchet & Clank (2002), based on [ReRAC](https://github.com/re-rac/rerac).

**[Download Development Beta14](https://github.com/HighDrexler/rerac-Android-APK-and-Ayn-Thor-Dual-Screen-Mod-Beta-/releases/tag/rerac-android-0.3.6-dev.5)** — Launcher **1.1.0-beta.14 / code30**, source-built **ReRAC 0.3.6** for ARM64 and x86-64.

Existing Development users: **Updates → Development → Check for Updates → Download update → Install downloaded update**. Updates install in place using the existing release certificate and package.

## Companion pause interface

The pause root and supported list pages present the game's actual choices as larger touch cards, with selection highlights, readable descriptions, Back and Resume. Choices identify the live menu widget and row. Maps, inventory grids and save confirmations retain the native renderer with focused panel reflow.

Beta14 also adds a copyable companion input report. After a failed item tap, **hold the companion Q Force icon, then tap Copy**. The report includes the installed version, the native input stage and equipment request state. It can be shared from the Thor without a PC.

**Companion quick-select touch on the physical Thor remains under investigation. Beta14 adds the pause redesign and diagnostics; it does not claim that remaining failure is fixed.** Beta13/code29 was confirmed to fail in both hybrid and dedicated companion layouts. Earlier injected-tap checks did not establish live equipment changes.

The build retains the Q Force emblem, vendor weapon-button Exit, crouch wrench THROW and native save/overwrite text corrections. Development remains the testing channel pending physical acceptance.

## Installation and game data

Install `ReRAC-Android-0.3.6-Launcher-1.1-beta14-DEV.apk` from the release page. In the launcher, enable the **AYN Thor Dual-Screen Mod** through Mods. Its toggle takes effect at the next game launch. Use Setup in-game to customize mobile controls.

For a new installation, import a supported USA v1.00 (SCUS-97199) Ratchet & Clank disc ISO or compatible extracted game-data ZIP. Game assets are not supplied.

The current Development package is `org.rerac.android.ui035test` (ReRAC 0.3.6 Test). It is a separate Android package from the early `org.rerac.android.uipreview` launcher previews.

## Build and validation

Editable native adapters are under `crates/rc-engine/src/android*`; native build/package tooling is under `android/native`. Source-built launcher and companion UI code is under `android/launcher`, with Development packaging and UI checks under `android/release`.

Beta14 uses native source [`afbf3d6`](https://github.com/HighDrexler/rerac-Android-APK-and-Ayn-Thor-Dual-Screen-Mod-Beta-/commit/afbf3d6d8c78004938d155bacece5249d5c45a0d). Both ABI builds passed [native run 39528247162](https://github.com/HighDrexler/rerac-Android-APK-and-Ayn-Thor-Dual-Screen-Mod-Beta-/actions/runs/39528247162). The final Java UI is separately rebuilt into the APK and checked by [startup run 39530043169](https://github.com/HighDrexler/rerac-Android-APK-and-Ayn-Thor-Dual-Screen-Mod-Beta-/actions/runs/39530043169) and [UI/input-receipt run 39530043079](https://github.com/HighDrexler/rerac-Android-APK-and-Ayn-Thor-Dual-Screen-Mod-Beta-/actions/runs/39530043079).

UI layout fixtures and rejected JNI input receipts are recorded as such; they are not live gameplay or physical Thor acceptance. Publication verifies the original signer, exact APK bytes, downloadable release asset and signed Development feed. See [release and update instructions](android/RELEASE_PROCESS.md). Private signing keys are not stored in GitHub or CI.

## Integrity

| APK | SHA-256 |
| --- | --- |
| Launcher 1.1.0-beta.14 / ReRAC 0.3.6 | `27a9fc02da5bfdf6994a106d04221f6903f9ebcb711bcf330502dc51e5d931e2` |
| Launcher 1.1.0-beta.13 / ReRAC 0.3.6 | `fee032d0bc49cf0c09108f33297e9c7ab538b5634a8f2106c276a548e869108e` |

This is an unofficial community adaptation; the original project and ISC license remain credited to ReRAC.
