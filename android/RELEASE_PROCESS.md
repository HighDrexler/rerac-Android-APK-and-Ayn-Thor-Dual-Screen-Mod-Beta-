# ReRAC Android release process

## Current development candidate

Launcher 1.1.0-beta.1 (Android versionCode 17) adds Updates, Mods and the small launcher/core footer to the published v16 Android app. Beta.2 (versionCode 18) is a second, otherwise equivalent candidate for exercising in-place updates.

**Both candidates still contain the original ReRAC 0.3.4/v16 native engine.** The repository's Rust workspace contains merged upstream 0.3.5 source, but that does not mean the Android APK contains 0.3.5. The complete native Android build project is still required before upgrading the native engine. Never change the APK core label to 0.3.5 without replacing and validating the actual native port.

## User update flow

Install the first 1.1 candidate over the existing v1.0 app; do not uninstall. Open Updates. Stable is the default; Development is an explicit early-access choice. Choose Check for updates, Download update, then Install downloaded update. Android may ask to allow installations from ReRAC. Enable that permission only when intentionally updating, return to the launcher, tap Install again and approve Android's installation prompt.

The app downloads only after approval. It verifies signed metadata, the complete download hash, package identity, signing certificate, Android version code and device compatibility. Updates are not applied while playing. Preserve backups of important saves when testing development builds. Switching from Development back to Stable does not downgrade Android; wait for a newer Stable build.

## What makes a dev change available to users

A source commit is not an update. The sequence is:

1. Commit the launcher changes and increment the launcher version and Android versionCode. Keep the embedded core version accurate. Maintain one increasing versionCode sequence across both channels.
2. Run Android launcher candidate CI. All regression tests and native-payload preservation checks must pass. The current recovery pipeline produces unsigned beta.1/beta.2 APKs; adjust those version arguments for a new release.
3. Download the exact successful artifact. Sign offline with the original private release key. Do not put the private key, password or signing backup in this repository, a workflow artifact, a source archive or a log. Keep package ID `org.rerac.android.uipreview` and the original certificate.
4. Verify signatures and 16 KiB native-library ZIP alignment. Generate an exact-hash public signing approval with `android/release/public_signature.py` using `apksigner --alignment-preserved true`. Public signature blocks cannot sign different APKs. Update the approved candidate run/commit and receipt together. This helper currently validates the 1.1 beta naming scheme; extend its filename validation deliberately for future version series.
5. Run Android signed test release CI. It validates the successful candidate run, reconstructs only the approved signed bytes, verifies them again, and performs real emulator installs, UI checks and data-preserving upgrades. Only a successful test may stage a Development prerelease.
6. Confirm the GitHub release APK asset is downloadable and matches its approved SHA-256. Only then update the signed `development.json` on the `update-channels` branch. Do not point a feed at an unsigned workflow artifact or a missing release asset.
7. Test on actual Android hardware, including the AYN Thor companion screen, imported game data, saves, touch layouts, cutscenes and gameplay. Promote a signed build to Stable only after acceptance; publish a Stable-channel envelope and appropriate release notes. Do not serve an experimental build through Stable merely because a commit exists on dev.

## Channel publication

The launcher reads `stable.json` or `development.json` from the repository's `update-channels` branch. Each file is an envelope with algorithm `SHA256withRSA`, base64 payload and base64 signature. Sign the exact UTF-8 payload bytes with the original signing key. The APK verifies with its installed certificate. A payload of `{"schema":1,"channel":"stable","release":null}` intentionally advertises no new Stable build.

A release payload contains schema/channel and a release object with `package_id`, `signer_sha256`, `launcher_version`, `core_version`, `version_code`, `bytes`, `min_sdk`, `abis`, `sha256`, `apk_url` and `notes`. The URL must be this repository's HTTPS release asset, not an arbitrary host or a dev source ZIP. All signed fields must describe the actual APK. Uploading an APK does not automatically change a channel.

## GitHub workflow permissions

The default branch holds the reviewed workflow definitions while launcher changes remain on dev. GitHub may reject release creation with GITHUB_TOKEN when the target commit changes workflow definitions relative to the default branch. Promote reviewed workflow definitions through an authorized maintainer operation before staging that target; never expose a personal token or weaken build verification to work around a publication failure.

## Mod scope

Only the existing Dual-Screen Mod is bundled. Its toggle writes the original single-byte `dual-screen.setting`; it takes effect on the next game launch. Choose Mods folder uses Android's document picker. External folders currently support validated configuration/resolution-profile mods, not arbitrary native libraries, executable scripts or generic game-code replacement. Disabled mods retain their choices across app updates.

## Validation boundaries

The Java suite covers update-policy validation, metadata signatures, settings preservation, mod compatibility, restricted APK serving and launcher UI integration. The emulator smoke suite covers the actual signed APKs and two in-place updates. These checks do not prove native gameplay, physical dual-screen output or ARM64 performance. ADB installation checks do not prove the network-download/Android-installer flow; record that as a separate end-to-end test.
