# ReRAC repository instructions

## Owner's distribution rule

- Never create or publish a GitHub Release for this project, including prereleases, beta releases, temporary build-input releases or release assets.
- Beta builds are unfinished test builds. Deliver updates only through the updater branch, currently `update-channel3`.
- Do not create release-publishing automation or use GitHub Releases as an artifact store.
- Keep APK signing, signed update metadata, checksum verification, increasing Android version codes and device acceptance checks.
- The installed beta.17 updater currently reads its manifest from `update-channel3` but restricts APK downloads to GitHub Release URLs. Branch-only artifact delivery requires changing that downloader before the next beta. Do not work around that limitation by creating another Release.
- Do not remove an active download or feed until its branch-only replacement works. Existing releases do not authorize creating additional ones.
- Do not ask the owner to relink Chat On Steroids.
