# Maintaining the Android core

Launcher 1.1 contains source-built ReRAC 0.3.6 engines for ARM64 and x86-64. The editable Android adapters are under `crates/rc-engine/src/android` and related input, audio and movie modules. `upstream.lock` pins the reviewed upstream revision; each native library embeds its actual source revision and Android target.

For an upstream update, reconcile those adapters with the new core, update the upstream pin and workspace version together, and build both ABIs. Preserve NativeActivity lifecycle, JNI signatures, controller/touch input, secondary-display routing, SAF data access, resolution/orientation, audio and movie playback. Keep shaders compatible with the rebuilt engine and retain 16 KiB ZIP/ELF alignment.

Run `native_abi_gate.py` with `--require-source-companion` on the candidate APK. This checks entry-point presence and architecture; it does not establish working gameplay. Run equipment/menu/rendering regressions and packaged Android tests, then test gameplay, media and device behavior affected by the port.

Preserve package `org.rerac.android.ui035test` and the retained signing certificate. Increase Android versionCode. A changed version label is not a native upgrade: record the real compiled source and exact native hashes.

Build and deliver only through `update-channel3`, following [the branch update process](../UPDATE_PROCESS.md). Do not create GitHub Releases or promote Stable without the owner's explicit instruction. Keep private signing material local.

The original v16 ABI reference and generic hook/package tools are retained for integration work. Binary offsets for that bootstrap apply only to its pinned input, never to a newly compiled engine.
