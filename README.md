# ReRAC for Android — Launcher 1.1

Android and AYN Thor adaptation of Ratchet & Clank (2002), based on [ReRAC](https://github.com/re-rac/rerac), with source-built ReRAC 0.3.6 for ARM64 and x86-64.

## Install or update

Use **Updates → Development → Check for updates** in beta18 or newer.

For a manual installation, [download launcher 1.1](builds/v1_1_r3/ReRAC-Android-1.1.apk.zip), extract the ZIP, and install `ReRAC.apk` over the existing app. Beta17 needs this one manual installation to enable branch downloads. Keep the existing app installed to preserve its data.

Launcher 1.1 uses Android versionCode 39 and the same package and signing certificate as beta17–beta19. Downloads and signed update metadata are hosted directly on `update-channel3`. No GitHub Release is used, and the Stable channel is not promoted.

## Companion screen

Quick select activates once on contact in both the Home and dedicated layouts. Visible wheel buttons take priority over stale map input, and a new touch recovers after a lost finger-up event. Cancellation releases held menu buttons and map drags.

The selected weapon now uses RAC1's original quick-select renderer: the native pulsing cursor sprite, selected-icon brightness, weapon name, and ammo. The Android circle and extra status overlay are removed. Extra pages use their own item assignments for ammo; Clank retains the original command presentation.

The owner confirmed working touch selection on the Thor in beta19. Version 1.1 adds the native presentation and cleanup; automated checks are described in [the update process](android/UPDATE_PROCESS.md).

Mobile pause-menu **Back** now sends Triangle, so a touch user can back out of submenus and resume from the root. The companion skill list again expands each skill to show the game’s own description. Repeated unchanged companion snapshots reuse parsed state to reduce UI allocation and CPU work; see [performance review](android/PERFORMANCE.md).

The map remains visible during ledge hangs, platforming, ambient quips and vendor interactions while the original weapon-change guards remain enforced. The native companion now reuses unchanged map images, retains static globe layers during fog updates and shares wheel working copies. Exact native pixel/draw comparisons passed on all 19 maps and both picture sets; [benchmark results and scope](android/PERFORMANCE.md) are recorded for build 39.

## Game data and setup

Import your own supported USA v1.00 (SCUS-97199) Ratchet & Clank disc ISO or compatible extracted game-data ZIP. Game assets are not supplied. Enable **AYN Thor Dual-Screen Mod** in Mods; its toggle takes effect at the next game launch. Use Setup in-game to customize controls.

The app package remains `org.rerac.android.ui035test` for in-place compatibility. Its displayed name is **ReRAC 1.1**.

## Source and verification

Android adapters are under `crates/rc-engine/src/android`; the original quick-select renderer is `crates/rc-game/src/menus/quick_select.rs`. Editable launcher and companion code is under `android/launcher`; branch build, signing reconstruction and test tools are under `android/release`.

The branch contains three workflows: build source, verify and host a locally signed APK, and verify the live branch updater. Old beta publication workflows and their superseded signing helpers have been removed. Earlier versioned downloads remain immutable for compatibility with cached signed feeds.

The signing key stays on the owner's PC. GitHub receives only public signing receipts, signed metadata, and signed builds. Checksums and native source identity are recorded in [the build report](builds/v1_1_r3/build.json).

This is an unofficial community adaptation. The original ReRAC project and ISC license remain credited; see [LICENSE](LICENSE).
