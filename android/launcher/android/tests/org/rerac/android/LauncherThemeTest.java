package org.rerac.android;

import android.app.Activity;
import android.content.Intent;
import android.graphics.Bitmap;
import android.graphics.Canvas;
import android.view.View;
import android.view.ViewGroup;
import android.widget.*;
import java.io.File;
import java.io.FileOutputStream;
import org.junit.*;
import org.junit.runner.RunWith;
import org.robolectric.Robolectric;
import org.robolectric.RuntimeEnvironment;
import org.robolectric.android.controller.ActivityController;
import org.robolectric.annotation.Config;
import org.robolectric.annotation.GraphicsMode;
import static org.junit.Assert.*;

@RunWith(org.robolectric.RobolectricTestRunner.class)
@Config(sdk=34)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
public class LauncherThemeTest {
    @Before public void noNetwork(){LauncherExtras.prefs(RuntimeEnvironment.getApplication()).edit().putBoolean("automatic",false).commit();}
    static class Fixture {
        ActivityController<Activity> controller;Activity a;ScrollView scroll;Button play,load;View root;
        Fixture(){controller=Robolectric.buildActivity(Activity.class).setup();a=controller.get();
            LauncherExtras.prefs(a).edit().putBoolean("automatic",false).commit();
            scroll=new ScrollView(a);LinearLayout column=new LinearLayout(a);column.setOrientation(1);scroll.addView(column);
            TextView title=new TextView(a),status=new TextView(a),help=new TextView(a);CheckBox dual=new CheckBox(a);
            title.setText("ReRAC");status.setText("Game data ready. Saves are kept separately from imports.");
            help.setText("Use a supported disc image. Controls can be customized in-game.");
            play=new Button(a);play.setText("Play");load=new Button(a);load.setText("Import disc ISO or game-data ZIP");
            ProgressBar progress=new ProgressBar(a);progress.setVisibility(View.GONE);Button cancel=new Button(a);cancel.setVisibility(View.GONE);
            column.addView(title);column.addView(status);column.addView(dual);column.addView(play);column.addView(load);column.addView(progress);column.addView(cancel);column.addView(help);
            a.setContentView(scroll);RenderingResolution.attach(a);root=a.findViewById(android.R.id.content);
        }
        void layout(int w,int h){root.measure(View.MeasureSpec.makeMeasureSpec(w,View.MeasureSpec.EXACTLY),View.MeasureSpec.makeMeasureSpec(h,View.MeasureSpec.EXACTLY));root.layout(0,0,w,h);}
        void close(){controller.pause().stop().destroy();}
    }
    @Test public void mainControlsHaveIconsAndKeepCallbacks(){Fixture f=new Fixture();try{
        f.layout(1000,700);final int[] clicked={0};f.play.setOnClickListener(v->clicked[0]++);f.play.performClick();assertEquals(1,clicked[0]);
        assertNotNull(f.play.getCompoundDrawablesRelative()[0]);assertNotNull(f.load.getCompoundDrawablesRelative()[0]);
        Button mods=f.root.findViewWithTag(LauncherExtras.MODS),updates=f.root.findViewWithTag(LauncherExtras.UPDATES);
        assertTrue(mods.isFocusable());assertTrue(updates.isFocusable());assertNotNull(mods.getCompoundDrawablesRelative()[0]);
        assertTrue(f.play.getMinimumHeight()>=LauncherAppearance.dp(f.a,56));
        TextView footer=f.root.findViewWithTag(LauncherExtras.FOOTER);assertTrue(f.scroll.getBottom()<=footer.getTop());
    }finally{f.close();}}
    @Test @Config(qualifiers="w360dp-h800dp-mdpi") public void narrowLayoutRetainsEveryControlAndFooter(){Fixture f=new Fixture();try{
        f.layout(360,760);Button mods=f.root.findViewWithTag(LauncherExtras.MODS);assertTrue(mods.getWidth()>0);assertTrue(mods.getWidth()<=360);
        assertTrue(f.play.getWidth()<=360);TextView footer=f.root.findViewWithTag(LauncherExtras.FOOTER);assertTrue(footer.getTop()>=f.scroll.getBottom());
        assertTrue(f.load.getLineCount()>=1);
    }finally{f.close();}}
    @Test public void panelsReactToKeyboardFocusAndDisabledState(){Activity a=Robolectric.buildActivity(Activity.class).setup().get();try{
        LauncherTheme.Plate p=new LauncherTheme.Plate(a,1);p.setBounds(0,0,180,60);Bitmap normal=Bitmap.createBitmap(180,60,Bitmap.Config.ARGB_8888);
        p.setState(new int[]{android.R.attr.state_enabled});p.draw(new Canvas(normal));
        Bitmap focused=Bitmap.createBitmap(180,60,Bitmap.Config.ARGB_8888);p.setState(new int[]{android.R.attr.state_enabled,android.R.attr.state_focused});p.draw(new Canvas(focused));
        assertNotEquals(normal.getPixel(60,25),focused.getPixel(60,25));
        Bitmap disabled=Bitmap.createBitmap(180,60,Bitmap.Config.ARGB_8888);p.setState(new int[]{});p.draw(new Canvas(disabled));assertNotEquals(normal.getPixel(60,25),disabled.getPixel(60,25));
    }finally{a.finish();}}
    @Test public void updatesHaveNamedSectionsAndUnchangedActionLabels(){
        ActivityController<LauncherToolsActivity> c=Robolectric.buildActivity(LauncherToolsActivity.class,new Intent().putExtra("page","updates")).setup();
        try{View root=c.get().findViewById(android.R.id.content);assertEquals(1,count(root,"Check for updates"));assertEquals(1,count(root,"Download update"));assertEquals(1,count(root,"Install downloaded update"));
            assertEquals(1,count(root,"INSTALLED BUILD"));assertEquals(1,count(root,"UPDATE CHANNEL"));assertEquals(1,count(root,"DOWNLOAD & INSTALL"));
        }finally{c.pause().stop().destroy();}
    }
    static int count(View v,String text){int n=v instanceof TextView&&text.contentEquals(((TextView)v).getText())?1:0;if(v instanceof ViewGroup)for(int i=0;i<((ViewGroup)v).getChildCount();i++)n+=count(((ViewGroup)v).getChildAt(i),text);return n;}
    @Test public void renderActualAndroidThemeViews()throws Exception{Fixture f=new Fixture();try{
        f.layout(1100,680);File dir=new File("target/theme-renders");assertTrue(dir.isDirectory()||dir.mkdirs());
        Bitmap image=Bitmap.createBitmap(1100,680,Bitmap.Config.ARGB_8888);f.root.draw(new Canvas(image));
        try(FileOutputStream out=new FileOutputStream(new File(dir,"launcher-landscape.png"))){assertTrue(image.compress(Bitmap.CompressFormat.PNG,100,out));}
        f.layout(400,840);Bitmap portrait=Bitmap.createBitmap(400,840,Bitmap.Config.ARGB_8888);f.root.draw(new Canvas(portrait));
        try(FileOutputStream out=new FileOutputStream(new File(dir,"launcher-narrow.png"))){assertTrue(portrait.compress(Bitmap.CompressFormat.PNG,100,out));}
    }finally{f.close();}}
}
