# ReRAC repository instructions

- Never create or publish a GitHub Release for this project, including prereleases, beta releases, temporary releases or release assets.
- Deliver unfinished beta builds and their signed update feed only through update-channel3. Do not use Releases as an artifact store.
- Beta18 implements raw.githubusercontent.com branch downloads with bounded signed ZIP transport, APK checksum, certificate and package validation. The archive contains exactly ReRAC.apk. No Release download addresses are accepted.
- Existing beta17 installs need one manual beta18 installation because their old downloader only accepted Release URLs. Use the same retained signing key and package so this installation preserves app data.
- Keep the private signing key private, increase versionCode for updates, verify native provenance and test the actual branch download and Android installation.
- Companion quick select is still failing on the owner's Thor. Never describe it as fixed or infer device acceptance from build, fixture or emulator tests.
- Do not ask the owner to relink Chat On Steroids or require Codex on their Thor.
