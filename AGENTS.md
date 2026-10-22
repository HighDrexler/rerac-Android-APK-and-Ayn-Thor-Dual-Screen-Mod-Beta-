# ReRAC repository instructions

- Never create or publish GitHub Releases, prereleases, tags for publication, temporary Releases or Release assets. Deliver builds and signed metadata only on `update-channel3`.
- Current launcher identity is 1.1, Android versionCode 37, core 0.3.6, package org.rerac.android.ui035test. Preserve the existing release-2026 signing key and package for in-place updates. Stable is not promoted.
- The owner confirmed working companion quick-select touch on physical Thor in beta19. Preserve contact-once activation, stale-map precedence, lost-UP recovery and retained-menu cancellation cleanup.
- Use RAC1's production quick-select renderer for selection presentation. `companion_wheel.rs` selects the equipped item/current Clank command and supplies page-local ammo assignments. Do not add Android cursor shapes or status overlays.
- Keep the signing key/password private. Only public signing blocks and signed update metadata may be committed. Increase versionCode for each new APK and verify actual native provenance.
- Preserve feed signatures, bounded ZIP extraction, checksum, certificate, package and compatibility checks. Beta17 needs one manual installation over the existing app; never tell users to uninstall.
- Never overwrite an archive URL referenced by a published signed feed. Retain earlier versioned archives while cached feeds may use them.
- The workflows are build-branch.yml, publish-branch.yml and verify-branch-update.yml. See android/UPDATE_PROCESS.md. Historical beta publication scripts/receipts were removed; use Git history for old investigations.
- Distinguish native rendering/equipment tests, Android transport-fixture input tests and physical device acceptance. Do not claim a physical 1.1 test until the owner confirms it.
- Do not ask the owner to relink Chat On Steroids, install Codex on the Thor or shuttle diagnostic text between devices.
- Updater version directory names allow letters, digits, underscores and hyphens only. Display version 1.1 uses builds/v1_1; never put a dotted version in an update URL directory. Validate feeds with the existing updater policy before promotion.
