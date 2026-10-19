# ReRAC Android: repeatable native port for 0.3.5+

## Canonical source of truth
The Android 1.0 release APK (`ReRAC-Mobile-v16.apk`) is the original Android port. It contains Android-specific native engine modifications that are **not** reproduced by upstream ReRAC's desktop Rust checkout. The current Launcher 1.1 build/updater reuses these v16 libraries and accurately reports core 0.3.4. Upstream shared source 0.3.5 on `dev` **does not** by itself mean an Android 0.3.5 APK exists.

## Completed recovery check
`native_abi_gate.py` independently extracts each `librc_engine.so` from a candidate APK, verifies ELF machine type and 21 dynamic functions (19 `GameActivity` JNI entries, `android_main`, `ANativeActivity_onCreate`). `v16-abi.json` records original v16 SHA-256 and ABI details. The original APK passes. CI can check any staged APK from a release tag. **This checks binary entry-point presence only, not JNI signatures or working gameplay.**

Run:

```bash
python3 android/native/native_abi_gate.py candidate.apk --baseline android/native/v16-abi.json --report native-check.json
```

## Remaining native implementation — required, not optional
1. Rebuild the Android platform entry points from actual v16 behavior and interfaces against the pinned upstream ReRAC 0.3.5 engine, preserving the original 19 JNI method **signatures**, not merely function names. Recover lifecycle, surfaces, graphics, touch/controller events, dual-screen display routing, resolution and orientation, SAF game-data access, audio and movie playback. Empty JNI stubs do **not** qualify.
2. Move Android adaptations into isolated Rust modules/patches with explicit upstream parent revision, and build native libraries for **both ARM64 and x86-64** using the Android NDK. Maintain `librc_extract.so`, shader compatibility and 16-KiB ZIP/ELF alignment.
3. Repackage under `org.rerac.android.uipreview`, preserving installed signing identity, data location, imported ISO, saved games and the launcher settings. Increment Android `versionCode`.
4. Run the ABI gate, Android build tests, in-place install/upgrade, real game/scene smoke tests, physical controllers, touch controls, movie/sound tests and companion screen on the AYN Thor. Only then should Development feed advertise `core_version: 0.3.5`. Never publish a version label without the corresponding engine binary.
5. Retain exact signed APK hashes and a release receipt. Keep Stable unchanged until the user approves physical-device test results. No keystore/password in GitHub.

## Repeatable 0.3.6+ path
Pin upstream ReRAC release SHA → merge on development → reapply **isolated Android adaptation layer** → compile both NDK ABI libraries and assets → run static ABI gate and behavioral device tests → sign new APK with original cert → publish new Development update manifest → optionally promote to Stable after approval. A GitHub source merge is not a device update; only a tested signed APK is.

The v16 source/evidence archive documents older native hook repairs. Those offsets are **specific to the original v16 binary** and must never be applied to a newly compiled 0.3.5 library. The recovery requires a genuine Android port, not binary relabeling or JNI no-op shims.
