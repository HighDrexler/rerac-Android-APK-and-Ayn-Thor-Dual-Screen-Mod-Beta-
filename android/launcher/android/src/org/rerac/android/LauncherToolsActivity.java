package org.rerac.android;

import android.app.Activity;
import android.app.AlertDialog;
import android.content.ClipData;
import android.content.Intent;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;
import android.provider.Settings;
import android.view.View;
import android.widget.*;
import java.io.File;
import java.util.Locale;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;

/** Updates and configuration mods, isolated from the running game process. */
public final class LauncherToolsActivity extends Activity {
    private static final int FOLDER=4181;
    private final ExecutorService worker=Executors.newSingleThreadExecutor();
    private UpdateClient.Task task;
    private int generation;
    private boolean destroyed,busy;
    private String page="updates";
    private LinearLayout body;
    private TextView status,details;
    private Button check,download,install,cancel;
    private ProgressBar progress;
    private UpdatePolicy.Release release,ready;

    @Override public void onCreate(Bundle saved){super.onCreate(saved);show(getIntent());}
    @Override protected void onNewIntent(Intent intent){super.onNewIntent(intent);setIntent(intent);show(intent);}
    @Override protected void onDestroy(){destroyed=true;cancelWork();worker.shutdownNow();super.onDestroy();}
    private void cancelWork(){generation++;if(task!=null)task.cancel();task=null;busy=false;}
    private void show(Intent intent){
        cancelWork();release=null;ready=null;
        page="mods".equals(intent.getStringExtra("page"))?"mods":"updates";
        if(getActionBar()!=null)getActionBar().hide();
        getWindow().setStatusBarColor(LauncherAppearance.NAVY);getWindow().setNavigationBarColor(LauncherAppearance.NAVY);
        LinearLayout host=LauncherAppearance.vertical(this);host.setBackground(new LauncherAppearance.MonitorDrawable(this,3));
        ScrollView scroll=new ScrollView(this);scroll.setFillViewport(true);body=LauncherAppearance.vertical(this);
        body.setPadding(dp(20),dp(16),dp(20),dp(20));scroll.addView(body,new ScrollView.LayoutParams(-1,-2));
        host.addView(scroll,new LinearLayout.LayoutParams(-1,0,1));host.addView(LauncherExtras.footer(this),new LinearLayout.LayoutParams(-1,-2));
        LauncherExtras.insets(host);setContentView(host);host.requestApplyInsets();
        label("ReRAC  /  "+("mods".equals(page)?"Mods":"Updates"),25,LauncherAppearance.YELLOW);
        Button back=button("Back to launcher",false);back.setOnClickListener(v->finish());
        if("mods".equals(page))showMods();else showUpdates();
    }
    private int dp(int value){return LauncherAppearance.dp(this,value);}
    private TextView label(String text,int size,int color){TextView t=LauncherAppearance.label(this,text,size,color);body.addView(t,LauncherAppearance.spaced(this,12));return t;}
    private Button button(String title,boolean primary){Button b=new Button(this);b.setText(title);LauncherAppearance.styleButton(b,primary);body.addView(b,LauncherAppearance.spaced(this,10));return b;}
    private void report(Exception e){status.setText(ModStore.message(e));}
    private boolean current(int id){return !destroyed && !isFinishing() && id==generation;}
    private void active(boolean value){
        busy=value;
        if(check!=null)check.setEnabled(!value);
        if(download!=null)download.setEnabled(!value && release!=null && ready==null);
        if(install!=null)install.setEnabled(!value && ready!=null);
        if(cancel!=null)cancel.setVisibility(value?View.VISIBLE:View.GONE);
        if(progress!=null)progress.setVisibility(value?View.VISIBLE:View.GONE);
    }
    private void showUpdates(){
        LauncherBuild build=LauncherBuild.read(this);
        label("Installed: Launcher "+build.launcher+"\nReRAC core "+build.core+"\n"+build.provenance,16,LauncherAppearance.BODY);
        label("Check for an update, download it, then approve installation in Android. You do not need to uninstall or import your game again.",15,LauncherAppearance.MUTED);
        RadioGroup channels=new RadioGroup(this);channels.setOrientation(RadioGroup.HORIZONTAL);
        RadioButton stable=new RadioButton(this),dev=new RadioButton(this);stable.setId(View.generateViewId());dev.setId(View.generateViewId());
        stable.setText("Stable");dev.setText("Development");stable.setTextColor(LauncherAppearance.BLUE);dev.setTextColor(LauncherAppearance.BLUE);
        channels.addView(stable);channels.addView(dev);body.addView(channels,LauncherAppearance.spaced(this,8));
        channels.check("development".equals(LauncherExtras.channel(this))?dev.getId():stable.getId());
        label("Stable is recommended. Development contains early test builds. Changing back to Stable never installs an older APK; wait for a newer Stable release.",14,LauncherAppearance.MUTED);
        channels.setOnCheckedChangeListener((group,id)->{
            cancelWork();release=null;ready=null;details.setText("");
            LauncherExtras.prefs(this).edit().putString("channel",id==dev.getId()?"development":"stable").apply();
            status.setText(id==dev.getId()?"Development selected. These builds may contain regressions.":"Stable selected.");active(false);
        });
        CheckBox automatic=new CheckBox(this);automatic.setText("Check when opening the launcher");automatic.setTextColor(LauncherAppearance.BLUE);
        automatic.setChecked(LauncherExtras.prefs(this).getBoolean("automatic",true));
        automatic.setOnCheckedChangeListener((b,value)->LauncherExtras.prefs(this).edit().putBoolean("automatic",value).apply());
        body.addView(automatic,LauncherAppearance.spaced(this,4));
        label("Automatic checks are limited to once every six hours. Nothing downloads or installs without your approval.",13,LauncherAppearance.MUTED);
        status=label("Ready to check "+LauncherExtras.channel(this)+" updates.",16,LauncherAppearance.BLUE);status.setAccessibilityLiveRegion(View.ACCESSIBILITY_LIVE_REGION_POLITE);
        details=label("",16,LauncherAppearance.BODY);
        check=button("Check for updates",false);check.setOnClickListener(v->checkUpdates());
        download=button("Download update",true);download.setEnabled(false);download.setOnClickListener(v->downloadUpdate());
        install=button("Install downloaded update",true);install.setEnabled(false);install.setOnClickListener(v->installUpdate());
        progress=new ProgressBar(this,null,android.R.attr.progressBarStyleHorizontal);progress.setMax(1000);progress.setVisibility(View.GONE);body.addView(progress,LauncherAppearance.spaced(this,12));
        cancel=button("Cancel",false);cancel.setVisibility(View.GONE);cancel.setOnClickListener(v->{cancelWork();status.setText("Cancelled. Your installed app was not changed.");active(false);});
    }
    private void checkUpdates(){
        cancelWork();ready=null;release=null;details.setText("");status.setText("Checking signed update information\u2026");active(true);
        final int id=generation;final String channel=LauncherExtras.channel(this);final UpdateClient.Task work=new UpdateClient.Task();task=work;
        worker.execute(()->{
            try{
                final UpdatePolicy.Release r=UpdateClient.check(getApplicationContext(),channel,work);
                runOnUiThread(()->{if(!current(id))return;
                    LauncherExtras.prefs(this).edit().putLong("available_"+channel,r==null?0:r.code).putLong("success_"+channel,System.currentTimeMillis()).apply();
                    if(r==null){status.setText("No update is currently published on "+channel+".");active(false);return;}
                    details.setText("Available: Launcher "+r.launcher+"\nReRAC "+r.core+"  \u00b7  "+size(r.bytes)+"\n\n"+r.notes);
                    if(r.code<=LauncherBuild.read(this).code){status.setText("You already have this version or a newer build. No downgrade is needed.");active(false);return;}
                    try{UpdatePolicy.compatible(r,LauncherBuild.read(this).code,Build.VERSION.SDK_INT,Build.SUPPORTED_ABIS);release=r;status.setText("A signed update is available.");}
                    catch(Exception e){report(e);}active(false);
                });
            }catch(Exception e){runOnUiThread(()->{if(current(id)){report(e);active(false);}});}
        });
    }
    private void downloadUpdate(){
        if(release==null || busy)return;
        final UpdatePolicy.Release chosen=release;cancelWork();final int id=generation;final UpdateClient.Task work=new UpdateClient.Task();task=work;
        progress.setIndeterminate(false);progress.setProgress(0);status.setText("Downloading\u2026 Keep this screen open until verification completes.");active(true);
        worker.execute(()->{
            try{
                UpdateClient.download(getApplicationContext(),chosen,work,(done,total)->runOnUiThread(()->{if(current(id)){progress.setProgress((int)(done*1000/total));status.setText("Downloading "+size(done)+" / "+size(total));}}));
                runOnUiThread(()->{if(current(id)){ready=chosen;status.setText("Download verified. Tap Install and approve the Android confirmation.");active(false);}});
            }catch(Exception e){runOnUiThread(()->{if(current(id)){report(e);active(false);}});}
        });
    }
    private void installUpdate(){
        if(ready==null || busy)return;
        if(!getPackageManager().canRequestPackageInstalls()){
            new AlertDialog.Builder(this).setTitle("Allow ReRAC updates")
                .setMessage("Android requires permission to install an APK downloaded by this launcher. Enable Allow from this source, return here, then tap Install again. Do not uninstall ReRAC.")
                .setNegativeButton("Cancel",null).setPositiveButton("Open Android settings",(d,w)->{
                    try{startActivity(new Intent(Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES,Uri.parse("package:"+getPackageName())));}
                    catch(Exception e){report(e);}
                }).show();return;
        }
        final UpdatePolicy.Release chosen=ready;cancelWork();final int id=generation;final UpdateClient.Task work=new UpdateClient.Task();task=work;
        progress.setIndeterminate(true);status.setText("Rechecking the APK before installation\u2026");active(true);
        worker.execute(()->{
            try{
                UpdateClient.verifyReady(getApplicationContext(),UpdateClient.verifiedFile(this),chosen,work);
                runOnUiThread(()->{if(!current(id))return;active(false);
                    try{
                        Uri uri=Uri.parse("content://"+getPackageName()+".updates/verified.apk");
                        Intent installIntent=new Intent(Intent.ACTION_VIEW).setDataAndType(uri,"application/vnd.android.package-archive")
                                .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION);
                        installIntent.setClipData(ClipData.newRawUri("ReRAC update",uri));
                        startActivity(installIntent);status.setText("Complete Android's installation prompt. Cancelling leaves your current version installed.");
                    }catch(Exception e){report(e);}
                });
            }catch(Exception e){runOnUiThread(()->{if(current(id)){ready=null;report(e);active(false);}});}
        });
    }
    private static String size(long bytes){return String.format(Locale.ROOT,"%.1f MiB",bytes/(1024.0*1024.0));}

    private void showMods(){
        label("Changes apply the next time you launch the game. Existing saves and control layouts are kept.",15,LauncherAppearance.MUTED);
        String folder=ModStore.preferences(this).getString(ModStore.TREE,"");
        label(folder.isEmpty()?"Mods folder: app-managed":"Mods folder: "+Uri.parse(folder).getLastPathSegment(),14,LauncherAppearance.MUTED);
        Button choose=button("Choose Mods folder",false);choose.setOnClickListener(v->{
            Intent pick=new Intent(Intent.ACTION_OPEN_DOCUMENT_TREE).addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION|Intent.FLAG_GRANT_WRITE_URI_PERMISSION|Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION|Intent.FLAG_GRANT_PREFIX_URI_PERMISSION);
            try{startActivityForResult(pick,FOLDER);}catch(Exception e){report(e);}
        });
        Button rescan=button("Refresh mod list",false);rescan.setOnClickListener(v->show(getIntent()));
        label("Only the Dual-Screen Mod is bundled. Put each additional mod in its own folder containing mod.json. This build also supports resolution-profile configuration mods; scripts and native-code mods are not loaded.",14,LauncherAppearance.MUTED);
        status=label("Reading mods\u2026",16,LauncherAppearance.BLUE);
        final int id=generation;worker.execute(()->{
            try{ModStore.Scan scan=ModStore.scan(getApplicationContext());runOnUiThread(()->{if(current(id))renderMods(scan);});}
            catch(Exception e){runOnUiThread(()->{if(current(id))report(e);});}
        });
    }
    private void renderMods(ModStore.Scan scan){
        status.setText(scan.warnings.isEmpty()?"Installed mods":"Folder warnings:\n"+android.text.TextUtils.join("\n",scan.warnings));
        String active=ModStore.preferences(this).getString("active_profile","");boolean found=false;
        for(ModDefinition mod:scan.mods){
            if(mod.id.equals(active))found=true;
            Switch toggle=new Switch(this);toggle.setText(mod.name+"  "+mod.version);toggle.setTextColor(LauncherAppearance.BLUE);
            toggle.setMinHeight(dp(52));boolean enabled=ModStore.enabled(this,mod);toggle.setChecked(enabled);
            toggle.setEnabled(mod.problem==null || enabled);body.addView(toggle,LauncherAppearance.spaced(this,14));
            String extra=mod.id.equals("dual-screen")?(ModStore.secondDisplay(this)?"Second display detected.":"A second active display is required. The enabled preference is kept while disconnected."):"ID: "+mod.id;
            label(mod.description+"\n"+extra+(mod.problem==null?"":"\n"+mod.problem),14,LauncherAppearance.MUTED);
            toggle.setOnCheckedChangeListener(new CompoundButton.OnCheckedChangeListener(){boolean resetting;
                @Override public void onCheckedChanged(CompoundButton button,boolean value){
                    if(resetting)return;
                    try{ModStore.setEnabled(LauncherToolsActivity.this,mod,value);status.setText(mod.name+(value?" enabled.":" disabled.")+" Applies on the next game launch.");if(mod.problem!=null && !value)button.setEnabled(false);}
                    catch(Exception e){resetting=true;button.setChecked(ModStore.enabled(LauncherToolsActivity.this,mod));resetting=false;report(e);}
                }
            });
        }
        if(!active.isEmpty() && !found){
            label("An enabled profile is no longer in this folder: "+active,14,LauncherAppearance.YELLOW);
            Button disable=button("Disable missing profile",false);disable.setOnClickListener(v->{try{ModStore.disableProfile(this);show(getIntent());}catch(Exception e){report(e);}});
        }
    }
    @Override protected void onActivityResult(int request,int result,Intent data){
        super.onActivityResult(request,result,data);
        if(request!=FOLDER || result!=RESULT_OK || data==null || data.getData()==null)return;
        Uri tree=data.getData();int flags=data.getFlags()&(Intent.FLAG_GRANT_READ_URI_PERMISSION|Intent.FLAG_GRANT_WRITE_URI_PERMISSION);
        try{getContentResolver().takePersistableUriPermission(tree,flags);}catch(Exception e){report(e);return;}
        status.setText("Preparing the selected Mods folder\u2026");final int id=generation;
        worker.execute(()->{try{ModStore.choose(getApplicationContext(),tree);runOnUiThread(()->{if(current(id))show(getIntent());});}
            catch(Exception e){runOnUiThread(()->{if(current(id))report(e);});}});
    }
}
