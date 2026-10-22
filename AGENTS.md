# ReRAC repository instructions

- Never create or publish GitHub Releases, prereleases, tags for publication, temporary Releases or Release assets. Deliver builds and signed metadata only on `update-channel3`.
- Current launcher identity is 1.1, Android versionCode 39, core 0.3.6, package org.rerac.android.ui035test. Preserve the existing release-2026 signing key and package for in-place updates. Stable is not promoted.
- The owner confirmed working companion quick-select touch on physical Thor in beta19. Preserve contact-once activation, stale-map precedence, lost-UP recovery and retained-menu cancellation cleanup.
- Use RAC1's production quick-select renderer for selection presentation. `companion_wheel.rs` selects the equipped item/current Clank command and supplies page-local ammo assignments. Do not add Android cursor shapes or status overlays.
- Keep the signing key/password private. Only public signing blocks and signed update metadata may be committed. Increase versionCode for each new APK and verify actual native provenance.
- Preserve feed signatures, bounded ZIP extraction, checksum, certificate, package and compatibility checks. Beta17 needs one manual installation over the existing app; never tell users to uninstall.
- Never overwrite an archive URL referenced by a published signed feed. Retain earlier versioned archives while cached feeds may use them.
- The workflows are build-branch.yml, publish-branch.yml and verify-branch-update.yml. See android/UPDATE_PROCESS.md. Historical beta publication scripts/receipts were removed; use Git history for old investigations.
- Distinguish native rendering/equipment tests, Android transport-fixture input tests and physical device acceptance. Do not claim a physical 1.1 test until the owner confirms it.
- Do not ask the owner to relink Chat On Steroids, install Codex on the Thor or shuttle diagnostic text between devices.
- Updater version directory names allow letters, digits, underscores and hyphens only. Display version 1.1 uses builds/v1_1_r3; never put a dotted version in an update URL directory. Validate feeds with the existing updater policy before promotion.

- Keep companion world/map visibility independent of equipment eligibility. The world flag keeps gameplay and vendor maps visible during temporary hand/body/interaction locks; do not relax the original selection guards to fix visibility.
- Companion map pixels are cached by exact fog bytes, source identity, palette, level and Map-o-Matic set. Camera, hero, markers and missions still refresh. Keep companion image keys separate from the primary pause map. Preserve byte-for-byte image/draw equivalence and the per-publication wheel working copy.
- Reproduce host performance measurements with the companion_benchmark example and record scope honestly: native desktop CPU/allocation figures are not Thor FPS, GPU, battery or resident-RAM measurements. Never upload extracted game assets.
