# Actual native-core integration status

## Source-built candidate exists; release is blocked

The integration/native-core-035 branch now has editable Android native runtime, input, companion-display, movie/audio and build integration. Another Android source archive or APK is not a prerequisite to compiling this candidate.

Launcher 1.1.0-beta.4 / versionCode 20 packages actual ReRAC 0.3.5 native libraries compiled for ARM64 and x86-64 from source 5e9b1292f5cf3f2ae0e27ee85edf604047f6cd37, based on upstream 756ff56ba0ef72b9fa94e91e7bdc3623fe286be0. It is not the v16 engine relabeled. Build run 39350794010 passed, with 20 native-adapter tests and 55 launcher tests. The unsigned APK SHA-256 is 4b9c3e2924ebfdd4012c9b304b30ae0f1705f3a77bcda6842c8364346e7efcd4.

Android 15 x86-64 preflight run 39354404118 passed: the actual NativeActivity initialized Vulkan, executed the 19 original JNI signatures, loaded all 24 packaged shader files, captured native graphics, survived background/resume, preserved private test data and launcher/mod/channel state, matched the installed APK hash and rejected downgrade. This was a diagnostic scene, not game content or gameplay. The earlier preflight run failed because its Python driver expected a raw instrumentation result while requesting Android's formatted output; raw mode was fixed and the full test rerun passed.

## Signing and testing boundaries

The preflight used a disposable emulator-only certificate for both prior and candidate APKs. It did NOT validate the original release signer, a real user's in-place upgrade, the live updater, real saved games, gameplay, cutscenes/audio or the physical Thor companion display. Those checks remain mandatory before release acceptance. See ../release/native-preflight.json for exact runs, hashes and scope.

The original private signing key corresponding to certificate SHA-256 25c5eebb05a013bf3fb84706f9ad87904b8d4d8a99ba7857d62e665d2b94e24a is not available in the current accessible workspace or inspected release/source archives. A compiled APK contains the public certificate, not that private key. Do not substitute a new signer, change package identity, bypass updater verification, or tell users to uninstall/clear data. Do not put private keys or passwords in GitHub or artifacts.

Live Development remains Launcher 1.1.0-beta.3 with ReRAC 0.3.4/v16. Stable is unchanged. Native preflight is not signing approval; native-approval.json must only authorize exact original-signer APK bytes after actual signing. No 0.3.5 update has been staged for installed users.

## Future upstream integration

The source adapters and NDK build are now editable and reproducible inputs rather than v16 binary offsets. For later upstream versions, pin the new upstream revision, reconcile Android changes, compile both architectures with matching shaders and embedded provenance, run native and game/device regression tests, sign with the retained release identity, and stage an exact-hash signed Development feed. A source merge or a passing diagnostic alone is not a completed user update.
