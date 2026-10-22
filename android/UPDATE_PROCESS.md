# Branch update process

Never create a GitHub Release, prerelease, tag-based publication, or Release asset for this project. Deliver builds only on `update-channel3`. Keep Stable unpromoted unless the owner explicitly authorizes a change.

1. Change editable source, increase Android versionCode, and build both native ABIs with `.github/workflows/build-branch.yml`. Native libraries embed the actual source revision and upstream pin. The workflow tests the native quick-select presentation and real equipment/menu controllers, checks native entry points, and retains the verified engine artifacts.
2. Remove the previous public signing receipt when preparing a new candidate. Pin that native run/source in android/native/engine-build.json. The staging job validates the engine/game jobs, builds the launcher, and stages an unsigned signing candidate. Download that exact candidate, validate its checksum and source identity, and sign it locally with the retained private key. Never upload the key or password. Publish only the public APK signing receipt as `android/release/branch-signatures.json`.
3. `.github/workflows/publish-branch.yml` reconstructs the exact locally signed bytes from that public receipt. It verifies the Android manifest, certificate, native hashes, ABI entry points and 16 KiB alignment. Unit and Android companion tests must pass before it commits the signed versioned ZIP, build report and checksums. It removes the temporary unsigned candidate.
4. Sign and update `development.json` only after the tested archive is hosted. Keep its versioned archive URL immutable. Directory names must use letters, digits, underscores or hyphens (display version 1.1 uses v1_1_r3). Run the signed feed through the existing updater policy before promotion. The launcher verifies the feed signature, bounded ZIP, extracted APK checksum, package, signing certificate and Android compatibility before offering installation.
5. `.github/workflows/verify-branch-update.yml` exercises the actual installed launcher downloading and installing the new APK through Android's confirmation screen. It checks the exact installed hash, app-data preservation, mod setting and selected channel, then checks that the update is not offered again.

## Current identity

Launcher 1.1, Android versionCode 39, ReRAC core 0.3.6. Package: `org.rerac.android.ui035test`. Signer SHA-256: `57188113b5796632a7d4db37622abf7c4d2fb3d9b0bccdc9830b01560ee0f81e`.

The display name is ReRAC 1.1. The package stays the same to preserve in-place installation from beta17 onward. Beta17's older downloader requires one manual installation over the existing app; do not uninstall.

## Validation scope

The native rendering tests use the production companion page helper and RAC1 renderer. They check the genuine cursor frame, size, location and pulse; selected and unselected alpha; extra-page ammo; absent items; and Clank commands. Equipment tests exercise actual hand put-away/deletion/creation and ready-state confirmation. Menu tests exercise actual Open/Back/Resume transitions.

Packaged Android tests cover both companion layouts, pointer ownership, contact activation, lost releases, stale map precedence, cancellation cleanup, native artwork without a synthetic Android overlay, pause screens and 16 taps delivered by Android's InputDispatcher to a secondary Presentation. Their selection transport uses a fixture; they do not constitute imported-game gameplay on a physical Thor.

The owner confirmed beta19 touch input on the Thor. Do not claim a physical 1.1 test until the owner confirms it. Preserve the contact-routing fix while changing presentation.
