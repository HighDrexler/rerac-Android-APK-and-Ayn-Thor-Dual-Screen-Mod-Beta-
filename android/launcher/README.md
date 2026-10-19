# Android launcher 1.1 recovery layer

This source builds the launcher changes over the checksum-pinned public v16 APK.
**It does not compile the native Android engine from the upstream Rust checkout.**
The resulting APK accurately reports ReRAC **0.3.4**. Upstream 0.3.5 on `dev` is
not included until the original Android native port is recovered and merged.

Implemented: signed Stable/Development update feeds, verified in-app APK downloads,
Android installation approval, separate version footer, existing dual-screen mod
control, user-selected Mods folder, and bounded resolution-profile configuration
mods. Original importer, touch controls, shaders and four native libraries are kept
byte-for-byte. No game data or signing secrets are included.

Build with JDK 21, Android SDK platform 36 and build-tools 36.0.0. Put android.jar
and the build-tools directory in a tools directory, then run:

```
python tools/android/build_launcher.py --baseline ReRAC-Mobile-v16.apk --tools /path/to/tools --output launcher-unsigned.apk
mvn test
```

The unsigned APK must be signed with the original private release key, then checked
with `apksigner verify --verbose --print-certs`. Never commit a keystore or password.
The expected public certificate digest is pinned in the source and build tools.
Both channels must use globally increasing Android versionCode values. Source
commits are not updates: only a validated, signed APK with a signed channel feed is
eligible. Stable is not promoted by development builds.

## Mods

The default folder is app-managed. Choose a writable folder through **Mods > Choose
Mods folder** to manage mod files in a normal document provider. The bundled
`dual-screen/mod.json` is created there. The existing one-byte
`files/dual-screen.setting` remains authoritative, preserving v16 choices.
Only the dual-screen mod is bundled. An additional configuration mod may use:

```json
{"schema":1,"id":"my-resolution","name":"My resolution","version":"1.0","kind":"resolution-profile","launcher_api":1,"core_versions":["0.3.4"],"settings":{"resolution":"720p"}}
```

Place it in its own subfolder as `mod.json`, refresh, then enable it. Only one
resolution profile may be active. Disabling restores the previous resolution unless
the user subsequently changed it. Unknown mod types, incompatible core versions,
duplicate IDs and malformed manifests are not enabled. Native/DEX/script plugins
are not supported by this recovery layer.

## Verification scope

The build checks the original payload and 16 KiB ZIP/ELF alignment. JVM tests cover
update validation, signatures, downgrade rejection, mod state, settings preservation,
provider restrictions and launcher layout. These checks do not claim gameplay or
AYN Thor hardware testing. Do not uninstall or clear app data to install an update.
