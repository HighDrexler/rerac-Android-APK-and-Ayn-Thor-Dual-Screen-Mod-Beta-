# Actual native-core integration status

Launcher 1.1.0-beta.3 is a Gadgetron-inspired presentation and update-UI release. Its actual ReRAC core remains 0.3.4/v16. Do not present this as a completed native 0.3.5 update.

The shared/desktop source has incorporated upstream 756ff56ba0ef72b9fa94e91e7bdc3623fe286be0 (0.3.5-alpha). This does not rebuild the Android port. The recovered v16 source ZIPs contain native repair helpers/history rather than the complete customized Android source checkout. Required recovery includes crates/rc-engine/src/android.rs, touch_input.rs, modified main.rs/gameplay.rs/menu_render.rs, movie/audio integration and the actual Android native build setup. Preserve uncommitted workspace modifications; do not reset or replace them with upstream.

Existing Java requires android_main/ANativeActivity_onCreate plus JNI nativeCompanion, nativeDisplayRate, nativeDualEnabled, nativeIcon, nativeInput, nativeMapView, nativeMapVisible, nativeMenuButton, nativeMenuGesture, nativeMenuMode, nativeMenuTouch, nativePause, nativePlanetIcon, nativeSecondary, nativeSelect, nativeSetDual, nativeTouchInput, nativeTouchMode and nativeWheel. Empty compatibility stubs are not an acceptable port.

Retained librc_engine.so SHA-256 values:
- arm64-v8a: 67a8859411c621edb834e7689ff520010349161b13d0ff31a97f1ea2ee2d037f
- x86_64: 4297a53c0fc783046fe0517789d9e52fddb78281bf3ad7cbce85d859d03e00d5

Completion requires actual 0.3.5 Android builds from the preserved port source, matching shaders, recorded source/ABI/payload provenance, existing JNI compatibility and 16-KiB alignment, same-signer in-place installation, then real-device tests for game saves, touch/controller input, menus/quick select, mobile cutscenes/audio and the Thor companion display. Only then may the signed Development feed advertise core 0.3.5. Stable must not be promoted by this work.

The updater already carries separate launcher/core versions. Its metadata must always describe the APK being downloaded, not the Rust version present elsewhere on a GitHub branch. Never put private signing keys/passwords into source control or CI.
