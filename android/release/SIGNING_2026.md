# Android signing baseline from beta.17

The owner approved a new release identity on October 21, 2026. Its public
configuration is in `release-identity-2026.json`:

- Package: `org.rerac.android.ui035test`
- Certificate SHA-256: `57188113b5796632a7d4db37622abf7c4d2fb3d9b0bccdc9830b01560ee0f81e`
- Signed feed branch: `update-channel3`
- First release: `rerac-android-0.3.6-dev.8`, beta.17, versionCode 33

No private signing material is included in the repository or publication
artifacts. Keep the same release identity for every subsequent update.

## Updating users

A one-time fresh installation is required when changing from an old certificate;
uninstalling may delete private saves/settings/game data. See
[BETA17_INSTALL.md](BETA17_INSTALL.md). Once beta.17 is installed, normal
higher-version updates signed with the new key use the GitHub Development flow.
The new Stable envelope contains `release:null`. Older `update-channel2` feeds
remain available unchanged.

## Publishing the next version

1. Preserve this signing identity and package ID. Increase the versionCode above
   33 and update the launcher version. Build and test both native architectures
   against a pinned upstream revision, retaining the ABI/source provenance gates.
2. Compile launcher Java with `parallel_package.source_built_dex` using the public
   signer above and `parallel_package.DEV_BRANCH='update-channel3'`.
   `parallel_package.transform` builds a source-built candidate by setting
   `parallel_package.EXPECTED_SIGNER` to this public signer before calling it.
   Its hard-coded defaults describe the legacy identity. `rekey_baseline.py`
   deliberately accepts only the immutable beta.17 migration artifact.
3. Sign the aligned APK offline with apksigner `--alignment-preserved`. Verify
   its certificate, manifest, native hashes and 16 KiB ZIP alignment.
4. Sign the exact release envelope using RSA/SHA-256 with the same key. Set
   `sign_035_feed.EXPECTED_CERT` to this public signer when reusing that legacy
   helper; preserve its package/core/version/hash/size validation.
5. Publish an immutable GitHub Release APK. Compare a complete public download
   with the signed hash before updating `update-channel3/development.json`.
   Keep Stable unchanged until physical device acceptance is complete.
6. Publication receipts may contain only public APK signing blocks, checksums
   and signed feeds. `verify_beta17_baseline.py` demonstrates signature and
   payload verification. Test an actual launcher download and Android
   installation from a lower version signed with the same key.

The beta.17 updater test uses a dedicated lower-code fixture signed with the new
key. It proves future same-key updating, not in-place migration from an old-key
production APK.
