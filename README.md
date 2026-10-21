# ReRAC for Android — Launcher and Dual-Screen Mod

Android and AYN Thor adaptation of Ratchet & Clank (2002), based on [ReRAC](https://github.com/re-rac/rerac).

**[Download Development Beta15](https://github.com/HighDrexler/rerac-Android-APK-and-Ayn-Thor-Dual-Screen-Mod-Beta-/releases/tag/rerac-android-0.3.6-dev.6)** — Launcher **1.1.0-beta.15 / code31**, source-built **ReRAC 0.3.6** for ARM64 and x86-64.

Existing Development users: **Updates → Development → Check for Updates → Download update → Install downloaded update**. Updates install in place using the existing release certificate and package.

## Companion restoration

This update uses the owner's working [v1.0 release](https://github.com/HighDrexler/rerac-Android-APK-and-Ayn-Thor-Dual-Screen-Mod-Beta-/releases/tag/v1.0) as its behavior reference. The retained companion view is unchanged apart from the source-built pause layer. Its native quick-select handler now restores the original direct equipment request, interaction-state admission, Drone handling, ownership/icon checks, usage statistics and ammo limits. The introduced dependencies on pause-menu state and gameplay item-definition lookup are removed. Both hybrid and dedicated layouts share this handler.

The pause root and supported list pages show the game's live choices as larger touch cards, with selection highlights, descriptions, Back and Resume. Signed menu labels (including Port Options), the active message table, locked-menu visibility and real list callbacks are preserved. Complete native menu snapshots no longer incorrectly claim they have been filtered to focused panels. Complex maps, inventory grids and confirmations retain native rendering and input.

The working secondary-display input report remains available: after a failed item tap, pause, **hold the companion Q Force icon, then tap Copy**. The report contains the installed build, native input result, native/view item IDs and equipment state; no PC is required.

Automated tests now exercise accepted selections through the actual hand-swap code and menu choices through actual Open/Back/Resume transitions. **Physical AYN Thor acceptance remains pending.** Earlier UI click tests established hit testing and rejection receipts only; those are not counted as live equipment success.

The build retains the Q Force emblem, vendor weapon-button Exit, crouch wrench THROW and save/overwrite text corrections. Development remains the testing channel; Stable is not promoted.

## Installation and game data

Install `ReRAC-Android-0.3.6-Launcher-1.1-beta15-DEV.apk` from the release page. Enable the **AYN Thor Dual-Screen Mod** through Mods; its toggle takes effect at the next game launch. Use Setup in-game to customize mobile controls.

For a new installation, import a supported USA v1.00 (SCUS-97199) Ratchet & Clank disc ISO or compatible extracted game-data ZIP. Game assets are not supplied.

The Development package is `org.rerac.android.ui035test` (ReRAC 0.3.6 Test), separate from the early `org.rerac.android.uipreview` launcher previews.

## Build and validation

Editable native adapters are under `crates/rc-engine/src/android*`; native build/package tooling is under `android/native`. Source-built launcher and companion UI code is under `android/launcher`, with Development packaging and UI checks under `android/release`.

Beta15 uses native source [`5b4e2a5`](https://github.com/HighDrexler/rerac-Android-APK-and-Ayn-Thor-Dual-Screen-Mod-Beta-/commit/5b4e2a5be854315394a25448251ae91c03e15284). Evidence: [native and equipment/menu regressions](https://github.com/HighDrexler/rerac-Android-APK-and-Ayn-Thor-Dual-Screen-Mod-Beta-/actions/runs/39535171827), [packaging/startup](https://github.com/HighDrexler/rerac-Android-APK-and-Ayn-Thor-Dual-Screen-Mod-Beta-/actions/runs/39535243049), and [companion UI/secondary-window checks](https://github.com/HighDrexler/rerac-Android-APK-and-Ayn-Thor-Dual-Screen-Mod-Beta-/actions/runs/39535243100). The actual production selection and menu helpers are imported by `crates/rc-game/tests/android_companion_selection.rs` and `crates/rc-game/tests/companion_menu.rs`; tests do not maintain separate copies of their implementation.

Publication verifies the original signer, exact APK bytes, downloadable release asset and signed Development feed. See [release and update instructions](android/RELEASE_PROCESS.md). Private signing keys are not stored in GitHub or CI.

## Integrity

| APK | SHA-256 |
| --- | --- |
| Launcher 1.1.0-beta.15 / ReRAC 0.3.6 | `285e622323d391875472aa644e032e6b0e8e46f851f7d402cc92edda66eb7d0f` |
| Launcher 1.1.0-beta.14 / ReRAC 0.3.6 | `27a9fc02da5bfdf6994a106d04221f6903f9ebcb711bcf330502dc51e5d931e2` |

This is an unofficial community adaptation; the original project and ISC license remain credited to ReRAC.
