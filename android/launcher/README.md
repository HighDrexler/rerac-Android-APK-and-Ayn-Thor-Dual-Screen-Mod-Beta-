# Android launcher 1.1

Editable launcher, updater, mods and companion UI live in `android/src/org/rerac/android`.

`CompanionQuickSelect` composes the native quick-select draw list inside the retained companion layout and owns touch routing. `QuickSelectModel` shares one transform between visible artwork and hit testing. Equipment activates once on finger-down. The native game confirms success; the Android layer does not paint its own selection cursor or status label over RAC1's presentation.

Unchanged quick-select packets reuse parsed model state; failures clear the cache so identical valid packets recover input.

Map, skill-list and complex-menu components retain their original rendering. Skill descriptions come from the native skill page’s description label table, rather than the list sublabel. Their refresh failures cannot disable wheel input. Cancellation releases retained menu repeats and drag state.

`UpdateClient`, `UpdatePolicy` and `UpdateArchive` implement signed, bounded downloads from `update-channel3`, archive extraction and APK identity verification. Preserve these checks.

Build and delivery are documented in [the branch update process](../UPDATE_PROCESS.md). Launcher 1.1 is Android versionCode 39 and uses the existing package/signing certificate.

Run the JVM tests with Maven after provisioning Android SDK 36. Companion instrumentation runs against the packaged APK in the branch verification workflow. The source compiler and DEX preparation tools are under `../release`; their temporary outputs are not application assets.
