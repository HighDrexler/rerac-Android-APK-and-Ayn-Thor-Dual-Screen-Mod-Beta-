package org.rerac.android;

import android.content.Context;
import android.content.pm.PackageInfo;
import android.os.Build;
import java.io.IOException;
import java.io.InputStream;
import org.json.JSONObject;

/** The APK's identity, not the version of a different source checkout. */
final class LauncherBuild {
    static final String PACKAGE = "org.rerac.android.uipreview";
    static final String REPOSITORY = "HighDrexler/rerac-Android-APK-and-Ayn-Thor-Dual-Screen-Mod-Beta-";
    static final String SIGNER = "25c5eebb05a013bf3fb84706f9ad87904b8d4d8a99ba7857d62e665d2b94e24a";
    static final String ASSET = "rerac-build.json";
    final String launcher, core, provenance;
    final long code;

    private LauncherBuild(String launcher, long code, String core, String provenance) {
        this.launcher = launcher; this.code = code; this.core = core; this.provenance = provenance;
    }

    static LauncherBuild read(Context c) {
        String launcher = "Unknown", core = "Unknown", provenance = "Metadata unavailable";
        long code = 0;
        try {
            PackageInfo p = c.getPackageManager().getPackageInfo(c.getPackageName(), 0);
            launcher = p.versionName == null ? "Unknown" : p.versionName;
            code = Build.VERSION.SDK_INT >= 28 ? p.getLongVersionCode() : p.versionCode;
            try (InputStream in = c.getAssets().open(ASSET)) {
                JSONObject o = new JSONObject(UpdatePolicy.utf8(UpdatePolicy.readBounded(in, 16384)));
                // A mismatched asset must not advertise a different core or launcher.
                if (o.getInt("schema") == 1 && c.getPackageName().equals(o.getString("package_id"))
                        && launcher.equals(o.getString("launcher_version")) && code == o.getLong("version_code")) {
                    core = UpdatePolicy.text(o, "core_version", 64);
                    provenance = UpdatePolicy.text(o, "core_provenance", 160);
                }
            }
        } catch (Exception failure) {
            android.util.Log.w("ReRAC-Launcher", "Build metadata unavailable", failure);
        }
        return new LauncherBuild(launcher, code, core, provenance);
    }

    String footer() { return "Launcher " + launcher + "  \u00b7  ReRAC " + core; }
}
