package org.rerac.android;

import android.app.Activity;
import android.app.AlertDialog;
import android.content.Context;
import android.content.res.ColorStateList;
import android.graphics.Bitmap;
import android.graphics.BitmapShader;
import android.graphics.Canvas;
import android.graphics.Color;
import android.graphics.ColorFilter;
import android.graphics.Paint;
import android.graphics.Path;
import android.graphics.PixelFormat;
import android.graphics.Rect;
import android.graphics.Shader;
import android.graphics.Typeface;
import android.graphics.drawable.Drawable;
import android.os.Build;
import android.view.Gravity;
import android.view.View;
import android.view.ViewGroup;
import android.view.Window;
import android.view.WindowInsets;
import android.widget.ArrayAdapter;
import android.widget.Button;
import android.widget.CheckBox;
import android.widget.CheckedTextView;
import android.widget.LinearLayout;
import android.widget.ProgressBar;
import android.widget.ScrollView;
import android.widget.TextView;

/** A quiet monitor-panel launcher skin. No timers, imported assets or game calls. */
public final class LauncherAppearance {
    static final String TAG = "org.rerac.android.launcher-appearance";
    static final String HELP_TAG = "org.rerac.android.launcher-help-toggle";
    static final String DISPLAY_TAG = "org.rerac.android.launcher-display";
    static final String STATUS_TAG = "org.rerac.android.launcher-status";
    private static final String RESOLUTION_TAG = "org.rerac.android.rendering-resolution";

    // RC1 pause-menu packed colors decoded as RGB: navy, light blue, yellow.
    static final int NAVY = 0xff080810;
    static final int BLUE = 0xff88a8ff;
    static final int YELLOW = 0xffffff20;
    static final int BODY = 0xffbdc9e8;
    static final int MUTED = 0xff8796b8;
    private static final int DISABLED = 0xff747788;
    private static final int PANEL = 0;
    private static final int ROW = 1;
    private static final int PRIMARY = 2;
    private static final int BACKGROUND = 3;
    private static Bitmap grain;

    private LauncherAppearance() {}

    /** Restyles and reparents existing controls; Activity fields/listeners stay valid. */
    public static void attach(Activity activity) {
        View content = activity.findViewById(android.R.id.content);
        if (!(content instanceof ViewGroup) || content.findViewWithTag(TAG) != null) return;
        ViewGroup root = (ViewGroup) content;
        if (root.getChildCount() != 1 || !(root.getChildAt(0) instanceof ScrollView)) return;
        final ScrollView scroll = (ScrollView) root.getChildAt(0);
        if (scroll.getChildCount() != 1 || !(scroll.getChildAt(0) instanceof LinearLayout)) return;
        LinearLayout original = (LinearLayout) scroll.getChildAt(0);
        // Fail closed if a future launcher changes its view structure. The existing
        // resolution helper's smaller test/embedding layouts remain untouched.
        if (original.getChildCount() != 9) return;
        View[] views = new View[9];
        for (int i = 0; i < views.length; i++) views[i] = original.getChildAt(i);
        if (!plainText(views[0]) || !plainText(views[1]) || !(views[2] instanceof CheckBox)
                || !(views[3] instanceof Button) || !(views[4] instanceof Button)
                || !(views[5] instanceof ProgressBar) || !(views[6] instanceof Button)
                || !(views[7] instanceof Button) || !RESOLUTION_TAG.equals(views[7].getTag())
                || !plainText(views[8])) return;

        TextView title = (TextView) views[0];
        TextView status = (TextView) views[1];
        CheckBox dual = (CheckBox) views[2];
        Button play = (Button) views[3];
        Button load = (Button) views[4];
        ProgressBar progress = (ProgressBar) views[5];
        Button cancel = (Button) views[6];
        Button resolution = (Button) views[7];
        final TextView help = (TextView) views[8];

        if (activity.getActionBar() != null) activity.getActionBar().hide();
        Window window = activity.getWindow();
        window.setStatusBarColor(NAVY);
        window.setNavigationBarColor(NAVY);
        window.getDecorView().setSystemUiVisibility(window.getDecorView().getSystemUiVisibility()
                & ~View.SYSTEM_UI_FLAG_LIGHT_STATUS_BAR & ~View.SYSTEM_UI_FLAG_LIGHT_NAVIGATION_BAR);
        scroll.setFillViewport(true);
        scroll.setClipToPadding(false);
        scroll.setVerticalScrollBarEnabled(false);
        scroll.setBackground(new MonitorDrawable(activity, BACKGROUND));
        final int side = dp(activity, 16), top = dp(activity, 20), bottom = dp(activity, 24);
        scroll.setPadding(side, top, side, bottom);
        scroll.setOnApplyWindowInsetsListener(new View.OnApplyWindowInsetsListener() {
            @Override public WindowInsets onApplyWindowInsets(View view, WindowInsets insets) {
                view.setPadding(side + insets.getSystemWindowInsetLeft(),
                        top + insets.getSystemWindowInsetTop(),
                        side + insets.getSystemWindowInsetRight(),
                        bottom + insets.getSystemWindowInsetBottom());
                return insets.consumeSystemWindowInsets();
            }
        });
        scroll.requestApplyInsets();

        original.removeAllViews();
        original.setTag(TAG);
        original.setPadding(0, 0, 0, 0);
        original.setBackgroundColor(Color.TRANSPARENT);
        original.setGravity(Gravity.CENTER_HORIZONTAL);
        original.setOrientation(LinearLayout.VERTICAL);
        LinearLayout column = new BoundedColumn(activity);
        original.addView(column, matchWrap());

        LinearLayout header = new AdaptiveHeader(activity);
        header.setPadding(dp(activity, 4), 0, dp(activity, 4), dp(activity, 4));
        TextView context = label(activity, "RATCHET & CLANK", 13, BLUE);
        context.setLetterSpacing(0.09f);
        title.setText("ReRAC");
        text(title, 28, YELLOW);
        title.setTypeface(Typeface.create("sans-serif-condensed", Typeface.BOLD));
        title.setLetterSpacing(0.03f);
        title.setPadding(0, dp(activity, 2), 0, 0);
        heading(title);
        header.addView(title, matchWrap());
        header.addView(context, matchWrap());
        column.addView(header, spaced(activity, 0));
        LinearLayout left = vertical(activity);
        LinearLayout right = vertical(activity);
        column.addView(new AdaptiveColumns(activity, left, right), spaced(activity, 16));

        LinearLayout game = panel(activity, "GAME DATA");
        game.setTag(STATUS_TAG);
        text(status, 16, BODY);
        status.setPadding(0, dp(activity, 6), 0, dp(activity, 2));
        status.setAccessibilityLiveRegion(View.ACCESSIBILITY_LIVE_REGION_POLITE);
        game.addView(status, matchWrap());
        progress.setProgressTintList(ColorStateList.valueOf(YELLOW));
        progress.setIndeterminateTintList(ColorStateList.valueOf(BLUE));
        progress.setProgressBackgroundTintList(ColorStateList.valueOf(0xff303040));
        LinearLayout.LayoutParams progressParams = new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, dp(activity, 8));
        progressParams.topMargin = dp(activity, 14);
        progressParams.bottomMargin = dp(activity, 6);
        game.addView(progress, progressParams);
        styleButton(cancel, false);
        game.addView(cancel, spaced(activity, 10));
        left.addView(game, spaced(activity, 0));

        styleButton(play, true);
        play.setCompoundDrawablesRelativeWithIntrinsicBounds(new PointerDrawable(activity, false), null, null, null);
        play.setCompoundDrawablePadding(dp(activity, 12));
        left.addView(play, spaced(activity, 12));
        styleButton(load, false);
        left.addView(load, spaced(activity, 8));

        LinearLayout display = panel(activity, "DISPLAY");
        display.setTag(DISPLAY_TAG);
        dual.setText("Dual screen");
        text(dual, 18, BLUE);
        dual.setTextColor(colors(false));
        dual.setButtonTintList(colors(false));
        dual.setMinHeight(dp(activity, 52));
        dual.setMinimumHeight(dp(activity, 52));
        dual.setPadding(dp(activity, 4), dp(activity, 8), dp(activity, 8), dp(activity, 8));
        dual.setBackgroundTintList(null);
        dual.setBackground(new MonitorDrawable(activity, ROW));
        dual.setStateListAnimator(null);
        display.addView(dual, spaced(activity, 8));
        TextView explanation = label(activity,
                "Dual-screen support is managed in Mods.", 14, MUTED);
        explanation.setPadding(dp(activity, 4), dp(activity, 5), dp(activity, 4), dp(activity, 4));
        display.addView(explanation, matchWrap());
        styleButton(resolution, false);
        display.addView(resolution, spaced(activity, 12));
        right.addView(display, spaced(activity, 0));

        final Button helpToggle = new Button(activity);
        helpToggle.setTag(HELP_TAG);
        helpToggle.setText("Help & controls");
        styleButton(helpToggle, false);
        helpToggle.setCompoundDrawablesRelativeWithIntrinsicBounds(null, null,
                new PointerDrawable(activity, true), null);
        helpToggle.setCompoundDrawablePadding(dp(activity, 12));
        if (Build.VERSION.SDK_INT >= 30) helpToggle.setStateDescription("Collapsed");
        text(help, 15, BODY);
        help.setPadding(dp(activity, 16), dp(activity, 14), dp(activity, 16), dp(activity, 16));
        help.setBackground(new MonitorDrawable(activity, PANEL));
        help.setVisibility(View.GONE);
        helpToggle.setOnClickListener(new View.OnClickListener() {
            @Override public void onClick(View view) {
                boolean expanded = help.getVisibility() != View.VISIBLE;
                help.setVisibility(expanded ? View.VISIBLE : View.GONE);
                helpToggle.setSelected(expanded);
                if (Build.VERSION.SDK_INT >= 30) helpToggle.setStateDescription(expanded ? "Expanded" : "Collapsed");
            }
        });
        right.addView(helpToggle, spaced(activity, 12));
        right.addView(help, spaced(activity, 8));
        LauncherExtras.attach(activity, root, scroll, right, dual);
    }

    static TextView dialogTitle(Context context, String value) {
        TextView title = label(context, value, 22, BLUE);
        title.setPadding(dp(context, 20), dp(context, 20), dp(context, 20), dp(context, 12));
        heading(title);
        return title;
    }

    static ArrayAdapter<String> resolutionChoices(final Context context, String[] labels) {
        return new ArrayAdapter<String>(context, android.R.layout.select_dialog_singlechoice, labels) {
            @Override public View getView(int position, View convertView, ViewGroup parent) {
                CheckedTextView row = (CheckedTextView) super.getView(position, convertView, parent);
                text(row, 17, BLUE);
                row.setTextColor(colors(false));
                row.setCheckMarkTintList(colors(false));
                row.setSingleLine(false);
                row.setMinHeight(dp(context, 54));
                row.setPadding(dp(context, 20), dp(context, 10), dp(context, 16), dp(context, 10));
                row.setBackground(new MonitorDrawable(context, ROW));
                return row;
            }
        };
    }

    static void styleDialog(AlertDialog dialog) {
        Context context = dialog.getContext();
        Window window = dialog.getWindow();
        if (window != null) {
            window.setBackgroundDrawable(new MonitorDrawable(context, PANEL));
            int width = Math.min(dp(context, 520), context.getResources().getDisplayMetrics().widthPixels - dp(context, 32));
            window.setLayout(Math.max(dp(context, 240), width), ViewGroup.LayoutParams.WRAP_CONTENT);
        }
        if (dialog.getListView() != null) {
            dialog.getListView().setBackgroundColor(Color.TRANSPARENT);
            dialog.getListView().setDivider(null);
            dialog.getListView().setSelector(new android.graphics.drawable.ColorDrawable(Color.TRANSPARENT));
        }
        for (int which : new int[] {AlertDialog.BUTTON_NEGATIVE, AlertDialog.BUTTON_POSITIVE}) {
            Button button = dialog.getButton(which);
            if (button != null) {
                styleButton(button, which == AlertDialog.BUTTON_POSITIVE);
                button.setTextSize(16);
                button.setPadding(dp(context, 18), dp(context, 10), dp(context, 18), dp(context, 10));
                button.setGravity(Gravity.CENTER);
                button.setMinHeight(dp(context, 48));
                button.setMinimumHeight(dp(context, 48));
            }
        }
    }

    private static boolean plainText(View view) {
        return view instanceof TextView && !(view instanceof Button);
    }

    static void text(TextView view, float size, int color) {
        view.setTextSize(size);
        view.setTextColor(color);
        view.setTypeface(Typeface.create("sans-serif", Typeface.NORMAL));
        view.setFontFeatureSettings("kern");
        view.setIncludeFontPadding(true);
        view.setShadowLayer(dp(view.getContext(), 1), dp(view.getContext(), 1), dp(view.getContext(), 1), Color.BLACK);
        view.setSingleLine(false);
        view.setEllipsize(null);
        view.setGravity(Gravity.START | Gravity.CENTER_VERTICAL);
    }

    static void styleButton(Button button, boolean primary) {
        Context context = button.getContext();
        text(button, 18, primary ? YELLOW : BLUE);
        button.setTextColor(colors(primary));
        button.setAllCaps(false);
        button.setMinHeight(dp(context, 56));
        button.setMinimumHeight(dp(context, 56));
        button.setMinWidth(0);
        button.setMinimumWidth(0);
        button.setPadding(dp(context, 18), dp(context, 13), dp(context, 18), dp(context, 13));
        button.setBackgroundTintList(null);
        button.setBackground(new MonitorDrawable(context, primary ? PRIMARY : ROW));
        button.setStateListAnimator(null);
        button.setElevation(0);
        button.setFocusable(true);
    }

    private static ColorStateList colors(boolean primary) {
        return new ColorStateList(new int[][] {
                {-android.R.attr.state_enabled}, {android.R.attr.state_pressed},
                {android.R.attr.state_focused}, {android.R.attr.state_checked},
                {android.R.attr.state_selected}, {}
        }, new int[] {DISABLED, YELLOW, YELLOW, YELLOW, YELLOW, primary ? YELLOW : BLUE});
    }

    private static void heading(TextView title) {
        if (Build.VERSION.SDK_INT >= 28) title.setAccessibilityHeading(true);
    }

    static TextView label(Context context, String value, float size, int color) {
        TextView label = new TextView(context);
        text(label, size, color);
        label.setText(value);
        return label;
    }

    static LinearLayout vertical(Context context) {
        LinearLayout result = new LinearLayout(context);
        result.setOrientation(LinearLayout.VERTICAL);
        return result;
    }

    static LinearLayout panel(Context context, String title) {
        LinearLayout panel = vertical(context);
        panel.setPadding(dp(context, 16), dp(context, 14), dp(context, 16), dp(context, 14));
        panel.setBackground(new MonitorDrawable(context, PANEL));
        TextView heading = label(context, title, 12, BLUE);
        heading.setLetterSpacing(0.07f);
        heading(heading);
        panel.addView(heading, matchWrap());
        return panel;
    }

    static LinearLayout.LayoutParams matchWrap() {
        return new LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT);
    }

    static LinearLayout.LayoutParams spaced(Context context, int top) {
        LinearLayout.LayoutParams params = matchWrap();
        params.topMargin = dp(context, top);
        return params;
    }

    static int dp(Context context, float value) {
        return Math.round(value * context.getResources().getDisplayMetrics().density);
    }

    private static final class BoundedColumn extends LinearLayout {
        private final int maximum;
        BoundedColumn(Context context) {
            super(context);
            maximum = dp(context, 1040);
            setOrientation(VERTICAL);
        }
        @Override protected void onMeasure(int width, int height) {
            int available = MeasureSpec.getSize(width);
            if (MeasureSpec.getMode(width) != MeasureSpec.UNSPECIFIED && available > maximum)
                width = MeasureSpec.makeMeasureSpec(maximum, MeasureSpec.EXACTLY);
            super.onMeasure(width, height);
        }
    }

    private static boolean wide(Context context, int width) {
        return width >= dp(context, 680)
                && width / Math.max(1, context.getResources().getConfiguration().fontScale) >= dp(context, 600);
    }

    /** Uses horizontal space on landscape phones; larger text returns to a stack. */
    private static final class AdaptiveColumns extends LinearLayout {
        private final LinearLayout left, right;
        private Boolean horizontal;
        AdaptiveColumns(Context context, LinearLayout left, LinearLayout right) {
            super(context);
            this.left = left; this.right = right;
            setBaselineAligned(false);
            addView(left); addView(right);
        }
        @Override protected void onMeasure(int width, int height) {
            boolean next = wide(getContext(), MeasureSpec.getSize(width));
            if (horizontal == null || horizontal.booleanValue() != next) {
                horizontal = next;
                setOrientation(next ? HORIZONTAL : VERTICAL);
                LayoutParams a = new LayoutParams(next ? 0 : LayoutParams.MATCH_PARENT, LayoutParams.WRAP_CONTENT, next ? 1 : 0);
                LayoutParams b = new LayoutParams(next ? 0 : LayoutParams.MATCH_PARENT, LayoutParams.WRAP_CONTENT, next ? 1 : 0);
                if (next) b.leftMargin = dp(getContext(), 20);
                else b.topMargin = dp(getContext(), 20);
                left.setLayoutParams(a); right.setLayoutParams(b);
            }
            super.onMeasure(width, height);
        }
    }

    private static final class AdaptiveHeader extends LinearLayout {
        private Boolean horizontal;
        AdaptiveHeader(Context context) { super(context); setGravity(Gravity.CENTER_VERTICAL); }
        @Override protected void onMeasure(int width, int height) {
            boolean next = wide(getContext(), MeasureSpec.getSize(width));
            if (horizontal == null || horizontal.booleanValue() != next) {
                horizontal = next;
                setOrientation(next ? HORIZONTAL : VERTICAL);
                for (int i = 0; i < getChildCount(); i++) {
                    LayoutParams params = new LayoutParams(LayoutParams.WRAP_CONTENT, LayoutParams.WRAP_CONTENT);
                    if (next && i > 0) params.leftMargin = dp(getContext(), 16);
                    getChildAt(i).setLayoutParams(params);
                }
            }
            super.onMeasure(width, height);
        }
    }

    private static synchronized Bitmap grain() {
        if (grain != null) return grain;
        int[] pixels = new int[64 * 64];
        int seed = 0x52435231;
        for (int y = 0; y < 64; y++) for (int x = 0; x < 64; x++) {
            seed ^= seed << 13; seed ^= seed >>> 17; seed ^= seed << 5;
            int alpha = (seed & 3) == 0 ? 5 + ((seed >>> 8) & 7) : 0;
            pixels[y * 64 + x] = y % 4 == 0 ? Color.argb(18, 0, 0, 0)
                    : Color.argb(alpha, 180, 180, 180);
        }
        grain = Bitmap.createBitmap(pixels, 64, 64, Bitmap.Config.ARGB_8888);
        return grain;
    }

    /** Cached still grain and a thin rectangular bevel, redrawn only with its View. */
    static final class MonitorDrawable extends Drawable {
        private final Paint paint = new Paint(Paint.ANTI_ALIAS_FLAG);
        private final Paint noise = new Paint();
        private final int role;
        private final float line;
        private int alpha = 255;
        MonitorDrawable(Context context, int role) {
            this.role = role;
            line = Math.max(1, dp(context, 1));
            noise.setShader(new BitmapShader(grain(), Shader.TileMode.REPEAT, Shader.TileMode.REPEAT));
        }
        @Override public boolean isStateful() { return role == ROW || role == PRIMARY; }
        @Override protected boolean onStateChange(int[] states) { invalidateSelf(); return isStateful(); }
        @Override public void draw(Canvas canvas) {
            Rect r = getBounds();
            boolean enabled = true, active = false;
            for (int state : getState()) {
                if (state == -android.R.attr.state_enabled) enabled = false;
                if (state == android.R.attr.state_focused || state == android.R.attr.state_pressed) active = true;
            }
            if (isStateful()) {
                enabled = false;
                for (int state : getState()) if (state == android.R.attr.state_enabled) enabled = true;
            }
            paint.setStyle(Paint.Style.FILL);
            paint.setColor(role == BACKGROUND ? NAVY : active && enabled ? 0xff20212b : 0xff11121d);
            paint.setAlpha(alpha);
            canvas.drawRect(r, paint);
            noise.setAlpha(role == BACKGROUND ? alpha / 2 : alpha);
            canvas.drawRect(r, noise);
            if (role != BACKGROUND) {
                paint.setStyle(Paint.Style.STROKE);
                paint.setStrokeWidth(line);
                paint.setColor(active && enabled ? YELLOW : role == PRIMARY && enabled ? 0xff8d903a : 0xff34394e);
                paint.setAlpha(alpha);
                float inset = line * 0.5f;
                canvas.drawRect(r.left + inset, r.top + inset, r.right - inset, r.bottom - inset, paint);
                paint.setColor(0xff05050b);
                paint.setAlpha(alpha);
                canvas.drawLine(r.left + line, r.bottom - line * 1.5f,
                        r.right - line, r.bottom - line * 1.5f, paint);
            }
        }
        @Override public void setAlpha(int alpha) { this.alpha = alpha; invalidateSelf(); }
        @Override public void setColorFilter(ColorFilter filter) { paint.setColorFilter(filter); invalidateSelf(); }
        @Override public int getOpacity() { return PixelFormat.TRANSLUCENT; }
    }

    private static final class PointerDrawable extends Drawable {
        private final Paint paint = new Paint(Paint.ANTI_ALIAS_FLAG);
        private final Path path = new Path();
        private final int size;
        private final boolean downward;
        PointerDrawable(Context context, boolean downward) { size = dp(context, 16); this.downward = downward; }
        @Override public int getIntrinsicWidth() { return size; }
        @Override public int getIntrinsicHeight() { return size; }
        @Override public boolean isStateful() { return true; }
        @Override protected boolean onStateChange(int[] state) { invalidateSelf(); return true; }
        @Override public void draw(Canvas canvas) {
            Rect r = getBounds();
            boolean enabled = false, selected = false;
            for (int state : getState()) {
                if (state == android.R.attr.state_enabled) enabled = true;
                if (state == android.R.attr.state_selected) selected = true;
            }
            paint.setColor(!enabled ? DISABLED : downward ? BLUE : YELLOW);
            paint.setStyle(Paint.Style.STROKE);
            paint.setStrokeWidth(Math.max(1.5f, size / 8f));
            paint.setStrokeCap(Paint.Cap.SQUARE);
            path.reset();
            if (downward) {
                float first = selected ? 0.64f : 0.36f, middle = selected ? 0.36f : 0.64f;
                path.moveTo(r.left + size * .2f, r.top + size * first);
                path.lineTo(r.left + size * .5f, r.top + size * middle);
                path.lineTo(r.left + size * .8f, r.top + size * first);
            } else {
                path.moveTo(r.left + size * .35f, r.top + size * .2f);
                path.lineTo(r.left + size * .65f, r.top + size * .5f);
                path.lineTo(r.left + size * .35f, r.top + size * .8f);
            }
            canvas.drawPath(path, paint);
        }
        @Override public void setAlpha(int alpha) { paint.setAlpha(alpha); invalidateSelf(); }
        @Override public void setColorFilter(ColorFilter filter) { paint.setColorFilter(filter); invalidateSelf(); }
        @Override public int getOpacity() { return PixelFormat.TRANSLUCENT; }
    }
}
