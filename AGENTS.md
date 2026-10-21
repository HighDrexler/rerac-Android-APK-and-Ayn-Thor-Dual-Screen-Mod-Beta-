# ReRAC repository instructions

- Never create or publish a GitHub Release for this project, including prereleases, beta releases, temporary releases or release assets.
- Deliver beta builds and signed update metadata only through `update-channel3`. Never use Releases as an artifact store.
- Beta18 and newer download bounded signed branch ZIPs containing exactly `ReRAC.apk`, then validate the APK checksum, certificate, package and Android compatibility. Preserve these checks.
- Beta17 needs a one-time manual installation of the current APK over the existing app because its downloader accepts only Release URLs. Use the retained signing key and package; do not tell users to uninstall.
- Keep the signing key private, increase versionCode and verify native provenance for every update.
- Beta19 (versionCode 35) changes companion equipment buttons to activate once on contact, gives visible acknowledgement, lets visible wheel hits take priority over stale retained map controls/drag state, clears cancellation independently of snapshot parsing, and recovers a new touch if the preceding UP was lost.
- Actions run 39689504070 reproduced a swallowed wheel tap on the actual beta18 APK and verified the corrected beta19 APK in the same scenario. It also passed 46 unit tests, the complete companion UI checks and 16 system-routed secondary-window taps. Those Android input tests use a selection transport fixture.
- Separately, the owner's local extracted Novalis data was used with the production native controller and full headless level simulation: all 16 requested weapon selections equipped. No game data was uploaded.
- These checks do not establish physical Thor acceptance. Report the specific tested defects and results; do not claim a physical Thor test occurred.
- Do not ask the owner to relink Chat On Steroids, install Codex on the Thor or shuttle diagnostic text between devices.
