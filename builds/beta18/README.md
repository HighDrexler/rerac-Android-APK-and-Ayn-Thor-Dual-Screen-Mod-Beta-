# Beta18: branch-only updater

Download ReRAC-Android-beta18.apk.zip from this directory, extract ReRAC.apk, and install it over beta17. It uses the same signing key; do not uninstall beta17. Earlier old-key installs still require the previously described migration.

Beta17 cannot perform this transition automatically because its downloader accepts only Release addresses. This is one manual installation. Subsequent updates download directly from update-channel3, verify the signed archive and APK, unpack automatically, and open Android's normal installation confirmation.

This beta changes updater delivery only. The companion quick-select failure on the Thor remains unresolved; this build does not claim to fix it.

No GitHub Release was created. The full downloaded APK SHA-256 is 4239609e07bb09a727f254f461ad4c87b2ecbd03907ce652fced6089ef9d30cb.
