ReRAC Android 0.3.6 / Launcher 1.1.0-beta.17 (versionCode 33)

This Development test build replaces companion quick select with the rebuilt
source-based controller. It retains the companion layout, artwork, maps and
menus, plus the earlier touch THROW and contextual Triangle fixes. Select an
item by tapping and releasing on the companion screen. Test both Home and the
dedicated Quick Select page, including switching Port Options from 4:3 to 16:9.
Physical AYN Thor gameplay acceptance remains pending.

This release establishes a NEW signing key and signed GitHub update feed.
Android will reject installing it over an APK signed with the old key.
Existing users must preserve any saves/settings they want to keep before
uninstalling the old app, then install this downloaded APK and reimport their
own supported game data as needed. Uninstalling can delete app-private saves,
settings and imported data. Do not uninstall until your backup is verified.
This migration does not transfer old app data automatically.

After this one-time migration, future APKs signed with the new release key
update normally through Updates > Development > Check for updates. The new
Stable channel is intentionally empty while device testing is outstanding.
Older launchers keep their existing feed; they cannot perform this key change
as an ordinary in-place update.

The new certificate SHA-256 is:
57188113b5796632a7d4db37622abf7c4d2fb3d9b0bccdc9830b01560ee0f81e

The APK contains the exact tested ARM64/x86_64 native engine from source
0ab6c9ca4be8d2f8f671a457c069a5f181e6d9da. The migration recompiles the
launcher trust configuration and updates metadata; game/native payload hashes
remain unchanged. Private signing material is retained locally and is never
published in this repository, release assets or Actions artifacts.
