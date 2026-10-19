# ReRAC Android release process

## Current Development release

Launcher **1.1.0-beta.3**, Android versionCode **19**, is the Gadgetron-style launcher, Mods and Updates presentation release. It is published under `launcher-v1.1.0-beta.3`; the signed Development feed advertises that exact APK. Source was merged into dev through PR #2. Earlier beta.1/code17 and beta.2/code18 remain unchanged historical updater-test releases.

**Actual ReRAC core remains 0.3.4/v16. Native Android 0.3.5 is not included.** The Rust workspace contains upstream 0.3.5, but its complete Android port source still needs recovery and rebuilding. See `android/native/INTEGRATION_STATUS.md`. Changing a label, feed field or Cargo version is not a native update.

Evidence is recorded in `android/release/validation-beta3.json`: 55 regression tests passed; exact signed APK/UI/data-preserving upgrade checks passed on Android 15 x86-64; the live beta.2-to-beta.3 HTTPS updater and actual Android installation confirmation passed. The installed APK matched its approved SHA-256. Only install-source permission was pre-granted in the disposable emulator. These tests did not run native gameplay, real user saves or a physical Thor companion display. Stable is unchanged.

## User update flow

On an installed 1.1 launcher, open Updates, select Development, Check for updates, Download update, then Install downloaded update. Approve Android's prompt and tap Open afterward. If Android asks to allow installation from ReRAC, grant it only when intentionally updating, return to the launcher and tap Install again. Install over the existing app; do not uninstall. Back up important saves before testing.

The app downloads only after approval. It verifies the signed metadata, full APK hash, app identity, signing certificate, versionCode and device compatibility. Updates are not applied while playing. Stable is the default; Development is opt-in. Switching back to Stable does not downgrade Android; wait for a newer Stable build.

## Candidate builds and release gates

1. Change the editable source and update `android/release/launcher-build.json` for the next candidate. Keep launcher version, increasing Android versionCode and actual core version separate. Never reuse a published versionCode for a different update.
2. `android-launcher.yml` builds the configured candidate rather than hard-coded obsolete beta.1/beta.2 outputs. The job is read-only, runs all regression tests and retains an unsigned APK, source commit, native-payload metadata and screenshots. The current retained-v16 builder rejects a false 0.3.5 claim. Source pushes alone do not publish updates.
3. Download the exact successful artifact and sign offline using the original release key. Never place the private key, password or signing backup in GitHub, a workflow artifact, logs or a source archive. Preserve package `org.rerac.android.uipreview` and the original certificate.
4. Verify APK signatures and 16-KiB native-library ZIP alignment. Generate exact-hash public signing approval using `android/release/public_signature.py` and apksigner with alignment preservation. Public signature blocks authorize only already-signed exact APK bytes; they cannot sign other code. The receipt and candidate run/source must agree. The helper currently accepts the 1.1 beta naming family; extend naming checks deliberately for another family.
5. Test the reconstructed exact signed bytes on Android before staging. Beta.3 used candidate run 39331452982, signed test 39333056672 and publication run 39334369573 on the isolated integration branch. Clean release commit 53e20ea572614d5ce4b670b6b37a35a9408f594f has identical application blobs to tested source d8692a45371cf9a2bd7de7cd66473558c47862a2. The original `android-test-release.yml` and test-approval.json retain the historic beta.1/beta.2 test; do not mistake that receipt for approval of a new APK.
6. Publish a new Development prerelease, never overwrite old APK assets. Verify the downloadable asset and its SHA-256 before replacing signed `development.json` on `update-channels`. Feed metadata must match the actual APK, not the shared Rust workspace. Do not change Stable during a Development test.
7. Exercise the live updater after staging. `android-updater-e2e.yml` now tests beta.2/code18 to beta.3/code19 using public HTTPS, signature checks and Android's actual installer. The test also hashes the installed base APK and checks private test data, mod/channel state and no repeat offer. For a future release update the previous-APK URL/hash and target versionCode/version/hash together. Use the existing script's explicit version arguments.
8. Test on actual Android hardware: imported game data, real saved games, touch layouts/controllers, cutscenes/audio, gameplay and the Thor companion screen. Stable promotion requires acceptance of those checks, a suitable new versionCode and a separately signed Stable envelope.

## Channel format

The launcher reads stable.json or development.json from update-channels. Each is an envelope with algorithm SHA256withRSA, base64 payload and base64 signature. Sign the exact UTF-8 payload bytes with the original key. The APK verifies with its installed certificate. A payload with schema1, channel stable and release null intentionally offers no Stable update.

A release payload contains schema/channel plus package_id, signer_sha256, launcher_version, core_version, version_code, bytes, min_sdk, abis, sha256, apk_url and notes. The URL must be this repository's HTTPS release APK asset, not an unsigned artifact, arbitrary host or source ZIP. Uploading an APK does not itself change a channel.

## GitHub permissions

Use read-only build/test jobs except the narrowly authorized publication step. GitHub may reject a release target with workflow-definition changes not approved on the default branch. Promote reviewed workflow definitions through an authorized maintainer action when necessary; never expose a personal token or weaken verification to bypass that check. Original main/v1.0 and Stable are not replaced by development work.

## Mods and native boundary

Only the existing Dual-Screen Mod is bundled. Its toggle writes the original single-byte dual-screen.setting and takes effect on next game launch. Mods-folder selection uses Android's document picker. Current external packages are validated configuration/resolution-profile mods, not arbitrary executable scripts, native libraries or general game-code replacement. Disabled choices persist across updates.

All beta.3 native libraries and original gameplay/importer DEX payloads remain byte-identical to v16. Rebuilding the actual 0.3.5 Android port is a separate unresolved gate, not an updater or presentation task. Its completion must preserve JNI entry points, display/input/cutscene behavior and saved-data paths and record ABI-specific native source/payload provenance.
