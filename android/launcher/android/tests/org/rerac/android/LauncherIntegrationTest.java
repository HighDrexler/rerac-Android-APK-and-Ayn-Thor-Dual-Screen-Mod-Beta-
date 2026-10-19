package org.rerac.android;

import android.app.Activity;
import android.content.Context;
import android.content.pm.PackageInfo;
import android.net.Uri;
import android.view.View;
import android.widget.*;
import org.junit.*;
import org.junit.runner.RunWith;
import org.robolectric.Robolectric;
import org.robolectric.RuntimeEnvironment;
import org.robolectric.Shadows;
import org.robolectric.annotation.Config;
import java.io.File;
import java.nio.file.Files;
import static org.junit.Assert.*;

@RunWith(org.robolectric.RobolectricTestRunner.class)
@Config(sdk=34,manifest="android/test-manifest.xml",assetDir="android/test-assets")
public class LauncherIntegrationTest {
    @Before public void setup(){LauncherExtras.prefs(RuntimeEnvironment.getApplication()).edit().putBoolean("automatic",false).commit();}
    @Test public void dualScreenUsesOriginalFileAndSurvivesReopen()throws Exception{
        Context c=RuntimeEnvironment.getApplication();ModStore.setDual(c,true);assertTrue(ModStore.dualEnabled(c));
        assertArrayEquals(new byte[]{1},Files.readAllBytes(new File(c.getFilesDir(),"dual-screen.setting").toPath()));
        ModStore.setDual(c,false);assertFalse(ModStore.dualEnabled(c));assertArrayEquals(new byte[]{0},Files.readAllBytes(new File(c.getFilesDir(),"dual-screen.setting").toPath()));
    }
    @Test public void profileTogglePreservesUnrelatedSettings()throws Exception{
        Context c=RuntimeEnvironment.getApplication();File f=new File(c.getFilesDir(),"settings.toml");Files.write(f.toPath(),"music = 0.8\nresolution = 1080p\n".getBytes("UTF-8"));
        ModDefinition m=ModDefinition.parse(UpdatePolicyTest.bytes(ModDefinitionTest.profile()),"0.3.4",false);
        ModStore.setEnabled(c,m,true);assertEquals("720p",ResolutionFile.effective(f));assertTrue(ModStore.enabled(c,m));
        ModStore.setEnabled(c,m,false);assertEquals("1080p",ResolutionFile.effective(f));assertFalse(ModStore.enabled(c,m));assertTrue(new String(Files.readAllBytes(f.toPath()),"UTF-8").contains("music = 0.8"));
    }
    @Test public void manualSettingsOverrideIsNotLostOnDisable()throws Exception{
        Context c=RuntimeEnvironment.getApplication();File f=new File(c.getFilesDir(),"settings.toml");ResolutionFile.select(f,"1080p");
        ModDefinition m=ModDefinition.parse(UpdatePolicyTest.bytes(ModDefinitionTest.profile()),"0.3.4",false);ModStore.setEnabled(c,m,true);
        ResolutionFile.select(f,"416p");ModStore.setEnabled(c,m,false);assertEquals("416p",ResolutionFile.effective(f));
    }
    @Test public void profileConflictsRejected()throws Exception{
        Context c=RuntimeEnvironment.getApplication();ModDefinition one=ModDefinition.parse(UpdatePolicyTest.bytes(ModDefinitionTest.profile()),"0.3.4",false);
        ModDefinition two=ModDefinition.parse(UpdatePolicyTest.bytes(ModDefinitionTest.profile().put("id","another-profile")),"0.3.4",false);
        ModStore.setEnabled(c,one,true);UpdatePolicyTest.rejects(()->ModStore.setEnabled(c,two,true));ModStore.setEnabled(c,one,false);
    }
    @Test public void providerRejectsPathsAndWrites()throws Exception{
        UpdateApkProvider provider=Robolectric.buildContentProvider(UpdateApkProvider.class).create().get();
        UpdatePolicyTest.rejects(()->provider.openFile(Uri.parse("content://org.rerac.android.uipreview.updates/../files/settings.toml"),"r"));
        UpdatePolicyTest.rejects(()->provider.openFile(Uri.parse("content://org.rerac.android.uipreview.updates/verified.apk"),"w"));
        UpdatePolicyTest.rejects(()->provider.openFile(Uri.parse("content://another.provider/verified.apk"),"r"));
    }
    @Test public void existingControlsRemainAndFooterHasOwnSpace(){
        Activity a=Robolectric.buildActivity(Activity.class).setup().get();LauncherExtras.prefs(a).edit().putBoolean("automatic",false).commit();
        ScrollView scroll=new ScrollView(a);LinearLayout original=new LinearLayout(a);original.setOrientation(1);scroll.addView(original);
        TextView title=new TextView(a),status=new TextView(a),help=new TextView(a);CheckBox dual=new CheckBox(a);
        Button play=new Button(a),load=new Button(a),cancel=new Button(a);ProgressBar progress=new ProgressBar(a);
        original.addView(title);original.addView(status);original.addView(dual);original.addView(play);original.addView(load);original.addView(progress);original.addView(cancel);original.addView(help);
        final int[] clicks={0};play.setOnClickListener(v->clicks[0]++);a.setContentView(scroll);RenderingResolution.attach(a);
        View root=a.findViewById(android.R.id.content);root.measure(View.MeasureSpec.makeMeasureSpec(960,View.MeasureSpec.EXACTLY),View.MeasureSpec.makeMeasureSpec(540,View.MeasureSpec.EXACTLY));root.layout(0,0,960,540);
        TextView footer=root.findViewWithTag(LauncherExtras.FOOTER);assertNotNull(footer);assertTrue(footer.getText().toString().contains("ReRAC"));
        assertNotNull(root.findViewWithTag(LauncherExtras.MODS));assertNotNull(root.findViewWithTag(LauncherExtras.UPDATES));assertEquals(View.GONE,dual.getVisibility());
        play.performClick();assertEquals(1,clicks[0]);assertNotNull(play.getParent());assertNotNull(load.getParent());assertNotNull(progress.getParent());
        assertTrue(scroll.getBottom()<=footer.getTop());RenderingResolution.attach(a);assertSame(footer,root.findViewWithTag(LauncherExtras.FOOTER));
        a.finish();
    }
    @Test public void modFolderOnlyBundlesDualScreen()throws Exception{
        Context c=RuntimeEnvironment.getApplication();ModStore.Scan scan=ModStore.scan(c);assertEquals(1,scan.mods.size());assertEquals("dual-screen",scan.mods.get(0).id);
    }
}
