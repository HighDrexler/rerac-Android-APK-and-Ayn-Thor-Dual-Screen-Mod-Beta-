# ReRAC Android signed update channels

These feeds are consumed by Launcher 1.1 and are verified against the original APK signing certificate. Do not hand-edit the base64 payload or signature. Sign the exact metadata offline and publish only after the referenced APK release asset exists and has passed its required tests.

Stable currently advertises no new build. Development advertises Launcher 1.1.0-beta.2, versionCode 18, ReRAC core 0.3.4/v16. The core 0.3.5 source merge is not included in these Android binaries.

For the update test, install beta.1 over v1.0, open Updates, select Development, check, download and approve the Android installation of beta.2. Do not uninstall. Beta.2 has the same features with a higher build number to exercise the update flow.

The build and release instructions are in `android/RELEASE_PROCESS.md` on dev. No private signing material belongs on this branch.
