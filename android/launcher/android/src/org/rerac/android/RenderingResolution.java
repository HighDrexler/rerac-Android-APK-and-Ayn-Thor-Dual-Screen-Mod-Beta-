package org.rerac.android;

import android.app.Activity;
import android.app.AlertDialog;
import android.content.Context;
import android.content.DialogInterface;
import android.util.Log;
import android.view.View;
import android.view.ViewGroup;
import android.view.ContextThemeWrapper;
import android.widget.Button;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.Toast;
import java.io.File;
import java.io.IOException;

/** Shares the engine's existing resolution setting with the Android launcher. */
public final class RenderingResolution {
    private static final String TAG = "ReRAC-Resolution";
    private static final String CONTROL_TAG = "org.rerac.android.rendering-resolution";
    private static final String[] VALUES = {"416p", "720p", "1080p", "1440p", "2160p", "window"};
    private static final String[] LABELS = {"416p", "720p", "1080p", "1440p", "2160p", "Screen resolution"};

    private RenderingResolution() {}

    /** Must run before NativeActivity.onCreate starts the native game process. */
    public static void prepare(Context context) {
        try {
            ResolutionFile.defaultIfUnset(file(context));
        } catch (IOException | SecurityException failure) {
            // A settings write must never stop the game or replace an unreadable file.
            Log.w(TAG, "Could not initialize rendering resolution", failure);
        }
    }

    /** Called after the launcher's existing content view has been installed. */
    public static void attach(final Activity activity) {
        prepare(activity);
        View content = activity.findViewById(android.R.id.content);
        if (!(content instanceof ViewGroup)) return;
        if (content.findViewWithTag(CONTROL_TAG) != null) {
            refresh(activity);
            LauncherAppearance.attach(activity);
            return;
        }
        ViewGroup root = (ViewGroup) content;
        if (root.getChildCount() != 1 || !(root.getChildAt(0) instanceof ScrollView)) return;
        ScrollView scroll = (ScrollView) root.getChildAt(0);
        if (scroll.getChildCount() != 1 || !(scroll.getChildAt(0) instanceof LinearLayout)) return;
        LinearLayout layout = (LinearLayout) scroll.getChildAt(0);

        final Button button = new Button(activity);
        button.setTag(CONTROL_TAG);
        button.setAllCaps(false);
        button.setOnClickListener(new View.OnClickListener() {
            @Override public void onClick(View view) { choose(activity); }
        });
        // Keep import controls, progress and status together. The resolution control
        // precedes the existing help paragraph, without changing any game controls.
        layout.addView(button, Math.max(0, layout.getChildCount() - 1),
                new LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT,
                        ViewGroup.LayoutParams.WRAP_CONTENT));
        refresh(activity);
        LauncherAppearance.attach(activity);
    }

    /** Re-read on returning from the game: Port Options remains the same authority. */
    public static void refresh(Activity activity) {
        LauncherExtras.refresh(activity);
        View content = activity.findViewById(android.R.id.content);
        if (content == null) return;
        View control = content.findViewWithTag(CONTROL_TAG);
        if (!(control instanceof Button)) return;
        Button button = (Button) control;
        try {
            int current = index(ResolutionFile.effective(file(activity)));
            button.setText("Rendering resolution: " + LABELS[current]);
        } catch (IOException | SecurityException failure) {
            button.setText("Rendering resolution");
            Log.w(TAG, "Could not read rendering resolution", failure);
        }
    }

    private static void choose(final Activity activity) {
        final int selected;
        try {
            selected = index(ResolutionFile.effective(file(activity)));
        } catch (IOException | SecurityException failure) {
            showFailure(activity, failure);
            return;
        }
        final int[] choice = {selected};
        final Context dialogContext = new ContextThemeWrapper(activity, android.R.style.Theme_Material_Dialog_Alert);
        AlertDialog dialog = new AlertDialog.Builder(dialogContext)
                .setCustomTitle(LauncherAppearance.dialogTitle(dialogContext, "Rendering resolution"))
                .setSingleChoiceItems(LauncherAppearance.resolutionChoices(dialogContext, LABELS), selected, new DialogInterface.OnClickListener() {
                    @Override public void onClick(DialogInterface dialog, int which) { choice[0] = which; }
                })
                .setNegativeButton("Cancel", null)
                .setPositiveButton("Apply", new DialogInterface.OnClickListener() {
                    @Override public void onClick(DialogInterface dialog, int which) {
                        try {
                            ResolutionFile.select(file(activity), VALUES[choice[0]]);
                            refresh(activity);
                        } catch (IOException | SecurityException failure) {
                            showFailure(activity, failure);
                        }
                    }
                })
                .show();
        LauncherAppearance.styleDialog(dialog);
    }

    private static void showFailure(Activity activity, Exception failure) {
        Log.w(TAG, "Could not save rendering resolution", failure);
        Toast.makeText(activity, "Could not save rendering resolution. Existing settings were kept.",
                Toast.LENGTH_LONG).show();
    }

    private static File file(Context context) {
        return new File(context.getFilesDir(), "settings.toml");
    }

    private static int index(String value) {
        for (int i = 0; i < VALUES.length; i++) if (VALUES[i].equals(value)) return i;
        return VALUES.length - 1;
    }
}
