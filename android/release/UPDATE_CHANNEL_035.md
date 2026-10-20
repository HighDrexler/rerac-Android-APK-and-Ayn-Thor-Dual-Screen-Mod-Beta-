# Android 0.3.5+ in-launcher updates

The independently installable Android package is **org.rerac.android.ui035test**, not the original **org.rerac.android.uipreview** application. Existing original-app data is never overwritten by the new package.

## Experimental beta.6 update verification

**Beta.5 is the installed release candidate.** Beta.6 is a Development/Experimental-only update using the same Android package ID and signing certificate, with versionCode **22** (beta.5 is 21). It retains ReRAC **0.3.5** and tests cutscene-performance improvements. Beta.6 must be available through **Updates > Development > Check for updates** in beta.5 rather than requiring manual installation.

The source improvements and Android production-ABI tests are on the `experimental` Git branch. The `update-channel2/development.json` RSA-signed release feed is separate from `update-channel2/stable.json` and from the original 0.3.4 application's updater. GitHub Releases must actually host the signed beta.6 APK before changing the Development feed: announcing an absent asset causes update downloads to fail.

A matching signed beta.6 APK exists, plus a locally signed, exact-hash manifest. The publishing script `Publish-ReRAC-beta6-InLauncher.ps1` verifies the approved APK and private signer, creates the immutable GitHub Development prerelease, downloads it back and compares hashes, and only then updates the signed Development feed. The script also configures encrypted GitHub Actions signing secrets for future 0.3.6 and later releases. The GitHub connection in this chat cannot upload binary release assets or set those protected secrets, so owner-side publication is a real remaining step, not another game-engine rebuild.

Do not promote experimental FMV changes to Stable merely because the launcher starts in an emulator. Stable should follow **tested official ReRAC core-version updates**, after real Android and AYN Thor acceptance.

## Required one-time owner setup

All future APKs must use the **identical private signing certificate** used for the initial 0.3.5 Test beta.4 and corrected beta.5. The verified certificate SHA-256 is **e2253eb7370c495ef0265efcdbe6bfbe09832070558fe9ed7047e7a2361ad75e**. Do not upload private key material to repository files or public releases. The GitHub publishing workflow expects the following private GitHub Actions secrets:

- **RERAC_035_KEYSTORE_B64**: Base64 representation of the original ReRAC-035-update-signing.p12 used for beta.4 and beta.5.
- **RERAC_035_KEYSTORE_PASSWORD**: Password to unlock that private PKCS12 keystore.

In a trusted Windows PowerShell session, with the private signing files in the current folder and GitHub CLI already authenticated:

    $repo = "HighDrexler/rerac-Android-APK-and-Ayn-Thor-Dual-Screen-Mod-Beta-"
    $keyB64 = [Convert]::ToBase64String([IO.File]::ReadAllBytes((Resolve-Path ".\ReRAC-035-update-signing.p12")))
    $keyB64 | gh secret set RERAC_035_KEYSTORE_B64 --repo $repo
    Remove-Variable keyB64
    (Get-Content -Raw ".\ReRAC-035-keystore-password.txt").Trim() | gh secret set RERAC_035_KEYSTORE_PASSWORD --repo $repo
    gh secret list --repo $repo

Keep the keystore and password privately backed up in encrypted storage. If the signing identity is lost, Android cannot install a new build over the existing test app without changing its identity or uninstalling.

## For upstream ReRAC 0.3.6, 0.3.7 and beyond

1. Merge/pin upstream changes into the Android integration branch and update the pinned upstream revision. Resolve any Android-specific engine API conflicts. Run **android-native-build.yml** for both ARM64 and x86-64. Compiled source stamps and ABI exports are checked, shaders are matched, and Rust native binaries are symbol-stripped.
2. Run **Android 0.3.5+ parallel APK startup regression** from GitHub Actions with the successful native build run ID, the intended new launcher version and an increased Android versionCode. This rebuilds launcher Java via javac/D8, produces a separate Android package signed by a temporary test key, and verifies it boots without crashing on an Android emulator. A failing gate must block publication.
3. Run **Publish ReRAC Android 0.3.5+ Development APK** from GitHub Actions on main, providing that same native build run ID, the matching successful smoke run ID, a new tag such as rerac-android-0.3.6-dev.1, the same launcher version, and increased versionCode. The workflow retrieves the signing key only from protected Actions secrets, rejects the wrong certificate, and publishes an immutable signed GitHub release APK.
4. After verifying the downloadable APK hash, the publishing workflow RSA-signs the exact Development feed payload and updates only the **update-channel2/development.json** branch. Users on beta.5 or later can choose **Updates > Development > Check for updates > Download > Install**; Android still requests installation approval. Existing app data remains in place.
5. Require physical gameplay and AYN Thor checks (game import, saves, controls, audio, movies, second display) before promising Stable. A green emulator startup or native diagnostic alone does not prove gameplay.

The signed **update-channel2/stable.json** and **update-channel2/development.json** currently contain release:null: the new app is configured to check its own channels, but no 0.3.6 update will appear until a new signed release has been published. The original app and its feed are deliberately untouched.

Never hand-edit DEX string tables: the previous binary rewrite caused the beta.4 crash. The corrected sidecar packaging source compiles Java/D8 instead. Native engine binaries require a real upstream rebuild rather than changing version labels.

Signing secrets have **not** been configured from this session: GitHub connector does not expose GitHub Actions secret writes, so this remains the one owner-controlled prerequisite before automatic future release publication.
