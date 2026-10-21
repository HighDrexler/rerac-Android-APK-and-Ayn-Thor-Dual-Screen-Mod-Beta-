# Android native integration

Launcher 1.1 uses source-built ReRAC 0.3.6 engines for ARM64 and x86-64. The native libraries embed their actual source revision, upstream revision, ABI and minimum Android API. The exact hashes and revision are in `builds/1.1/build.json`.

The current branch build is `.github/workflows/build-branch.yml`; signed verification is `.github/workflows/publish-branch.yml`. Both operate on `update-channel3`. See [the update process](../UPDATE_PROCESS.md). Historical beta publication workflows and their receipts were removed; Git history preserves the investigation record.

The native companion uses the production equipment controller and the original RAC1 quick-select draw implementation. `companion_wheel.rs` prepares page-local assignments and selects the equipped item or active Clank command. It does not duplicate cursor artwork or the game's animation math in Android.

Generic native hook, ABI and packaging tools remain available under this directory for future upstream integration. Preserve ABI entry points, media/input adapters, shaders, signing identity and 16 KiB native-library alignment when updating the core.
