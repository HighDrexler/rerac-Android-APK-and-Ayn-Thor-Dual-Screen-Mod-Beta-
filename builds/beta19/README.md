# ReRAC Android beta19

Companion quick select now activates once on finger-down. Visible wheel hits take
priority over retained map controls and drag state; a new touch also recovers if
Android did not deliver the preceding finger-up event. A gold marker acknowledges
the tap immediately. Equipment success is reported only after the native game
confirms it.

Beta19 uses versionCode 35 and the same release-2026 signing certificate as
beta17 and beta18. Beta18 users can select Development in the launcher updater.
Beta17 users must install this APK manually once because that older downloader
does not support branch archives. Install over the existing app; do not uninstall.

The download is `ReRAC-Android-beta19.apk.zip`, containing only `ReRAC.apk`.
The launcher downloads, verifies and extracts it automatically. For manual
installation, extract the ZIP and open `ReRAC.apk`.

Validation includes an APK before/after regression for map state swallowing wheel
taps, contact/pointer and lost-release checks, and 16 Android system-routed taps
on an actual secondary Presentation window. Those Android tests use a selection
transport fixture. Separately, a local headless run of the actual Novalis level,
using the owner's existing game data and production native controller, completed
all 16 tested weapon selections. No physical Thor run is claimed by these tests.

Builds and signed update metadata live only on `update-channel3`. No GitHub
Release is used.
