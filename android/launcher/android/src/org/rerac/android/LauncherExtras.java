package org.rerac.android;

import android.app.Activity;
import android.content.Context;
import android.content.Intent;
import android.content.SharedPreferences;
import android.os.Handler;
import android.os.Looper;
import android.view.Gravity;
import android.view.View;
import android.view.ViewGroup;
import android.view.WindowInsets;
import android.widget.Button;
import android.widget.CheckBox;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.TextView;
import java.lang.ref.WeakReference;

/** Small additive controls; the original launcher and native input stay intact. */
final class LauncherExtras {
    static final String FOOTER="org.rerac.android.version-footer", UPDATES="org.rerac.android.updates", MODS="org.rerac.android.mods";
    static SharedPreferences prefs(Context c){return c.getSharedPreferences("launcher-updates-v1",Context.MODE_PRIVATE);}
    static String channel(Context c){String s=prefs(c).getString("channel","stable");return "development".equals(s)?s:"stable";}
    private LauncherExtras(){}

    static void attach(Activity a,ViewGroup root,ScrollView scroll,LinearLayout right,CheckBox dual) {
        dual.setVisibility(View.GONE); // Kept attached: original onResume/listeners continue to synchronize it.
        Button mods=new Button(a);mods.setTag(MODS);mods.setText("Mods");LauncherAppearance.styleButton(mods,false);
        mods.setOnClickListener(v->open(a,"mods"));right.addView(mods,1,LauncherAppearance.spaced(a,8));
        Button updates=new Button(a);updates.setTag(UPDATES);updates.setText("Updates");LauncherAppearance.styleButton(updates,false);
        updates.setOnClickListener(v->open(a,"updates"));right.addView(updates,2,LauncherAppearance.spaced(a,8));
        // Reserve real layout space for the footer; never draw it over Play or help.
        root.removeView(scroll);scroll.setOnApplyWindowInsetsListener(null);
        scroll.setBackgroundColor(android.graphics.Color.TRANSPARENT);
        scroll.setPadding(LauncherAppearance.dp(a,16),LauncherAppearance.dp(a,16),LauncherAppearance.dp(a,16),LauncherAppearance.dp(a,12));
        LinearLayout host=LauncherAppearance.vertical(a);host.setBackground(new LauncherAppearance.MonitorDrawable(a,3));
        host.addView(scroll,new LinearLayout.LayoutParams(-1,0,1));
        TextView footer=footer(a);host.addView(footer,new LinearLayout.LayoutParams(-1,-2));
        insets(host);root.addView(host,new ViewGroup.LayoutParams(-1,-1));host.requestApplyInsets();refresh(a);
    }
    static TextView footer(Context c){
        TextView v=LauncherAppearance.label(c,LauncherBuild.read(c).footer(),11,LauncherAppearance.MUTED);
        v.setTag(FOOTER);v.setGravity(Gravity.CENTER);v.setPadding(8,5,8,7);v.setTextIsSelectable(true);return v;
    }
    static void insets(View root){
        root.setOnApplyWindowInsetsListener((v,w)->{
            v.setPadding(w.getSystemWindowInsetLeft(),w.getSystemWindowInsetTop(),w.getSystemWindowInsetRight(),w.getSystemWindowInsetBottom());
            return w.consumeSystemWindowInsets();
        });
    }
    static void open(Activity a,String page){
        a.startActivity(new Intent(a,LauncherToolsActivity.class).putExtra("page",page)
                .addFlags(Intent.FLAG_ACTIVITY_CLEAR_TOP|Intent.FLAG_ACTIVITY_SINGLE_TOP));
    }
    static void refresh(Activity a){
        View root=a.findViewById(android.R.id.content);if(root==null)return;
        TextView footer=root.findViewWithTag(FOOTER);if(footer!=null)footer.setText(LauncherBuild.read(a).footer());
        Button mods=root.findViewWithTag(MODS);if(mods!=null)mods.setText("Mods  \u00b7  Dual screen "+(ModStore.dualEnabled(a)?"On":"Off"));
        Button updates=root.findViewWithTag(UPDATES);if(updates==null)return;
        long available=prefs(a).getLong("available_"+channel(a),0);
        updates.setText(available>LauncherBuild.read(a).code?"Updates  \u00b7  Available":"Updates");
        autoCheck(a);
    }
    private static void autoCheck(Activity activity){
        Context app=activity.getApplicationContext();SharedPreferences p=prefs(app);long now=System.currentTimeMillis();
        String channel=channel(app);long last=p.getLong("attempt_"+channel,0);
        if(!p.getBoolean("automatic",true) || (now>=last && now-last<6L*60*60*1000))return;
        p.edit().putLong("attempt_"+channel,now).apply();WeakReference<Activity> ref=new WeakReference<Activity>(activity);
        Thread worker=new Thread(()->{
            try{
                UpdatePolicy.Release r=UpdateClient.check(app,channel,new UpdateClient.Task());
                p.edit().putLong("available_"+channel,r==null?0:r.code).putLong("success_"+channel,System.currentTimeMillis()).apply();
                new Handler(Looper.getMainLooper()).post(()->{Activity a=ref.get();if(a!=null && !a.isFinishing() && !a.isDestroyed())refresh(a);});
            }catch(Exception ignored){ /* Offline or unpublished feed: never block gameplay or show a misleading success. */ }
        },"ReRAC-update-check");worker.setDaemon(true);worker.start();
    }
}
