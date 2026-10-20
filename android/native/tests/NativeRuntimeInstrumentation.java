package org.rerac.tests;

import android.app.Activity;
import android.app.Instrumentation;
import android.accessibilityservice.AccessibilityService;
import android.content.Context;
import android.content.Intent;
import android.os.Bundle;
import android.os.SystemClock;
import java.io.*;
import java.lang.reflect.Method;
import java.util.UUID;
import org.json.JSONObject;

/** Runs only as a separately installed, same-signer instrumentation APK. No game assets. */
public final class NativeRuntimeInstrumentation extends Instrumentation {
    private Bundle args;
    private Class<?> game;
    private int calls;
    private static void require(boolean ok,String message){if(!ok)throw new AssertionError(message);}
    @Override public void onCreate(Bundle arguments){args=arguments==null?new Bundle():arguments;start();}
    private Object call(String name,Class<?>[] types,Object...values)throws Exception{
        Method m=game.getDeclaredMethod(name,types);m.setAccessible(true);calls++;return m.invoke(null,values);
    }
    private static String read(File file)throws Exception{
        try(InputStream in=new FileInputStream(file);ByteArrayOutputStream out=new ByteArrayOutputStream()){
            byte[] b=new byte[4096];int n;while((n=in.read(b))>=0){out.write(b,0,n);require(out.size()<131072,"Oversized report");}return out.toString("UTF-8");
        }
    }
    private static JSONObject report(File file)throws Exception{return new JSONObject(read(file));}
    private static void write(File file,String text)throws Exception{try(OutputStream out=new FileOutputStream(file)){out.write(text.getBytes("UTF-8"));}}
    @Override public void onStart(){
        Bundle result=new Bundle();JSONObject evidence=new JSONObject();
        try{
            Context c=getTargetContext();ClassLoader loader=c.getClassLoader();
            game=Class.forName("org.rerac.android.GameActivity",true,loader);
            Class<?> coreClass=Class.forName("org.rerac.android.NativeCore",true,loader);
            Method info=coreClass.getDeclaredMethod("buildInfo");info.setAccessible(true);
            JSONObject core=new JSONObject((String)info.invoke(null));
            String expected=args.getString("core","0.3.5");
            require(expected.equals(core.getString("core_version")),"Native binary is not the requested core");
            if(args.containsKey("source"))require(args.getString("source").equals(core.getString("source_sha")),"Native source differs from candidate");
            Class<?>[] none=new Class<?>[0],bool={boolean.class},integer={int.class};
            require(new JSONObject((String)call("nativeWheel",none)).has("pages"),"Invalid wheel protocol");
            require(new JSONObject((String)call("nativeCompanion",none)).has("prims"),"Invalid companion protocol");
            call("nativeSetDual",bool,true);require(Boolean.TRUE.equals(call("nativeDualEnabled",none)),"Dual enable did not reach native state");
            call("nativeSetDual",bool,false);require(Boolean.FALSE.equals(call("nativeDualEnabled",none)),"Dual disable did not reach native state");
            call("nativeDisplayRate",new Class<?>[]{float.class},120f);
            call("nativeInput",new Class<?>[]{int.class,float.class,float.class,float.class,float.class},64,1f,0f,0f,0f);
            call("nativeTouchInput",new Class<?>[]{int.class,float.class,float.class,float.class,float.class,int.class},8,0f,-1f,0f,0f,1);
            call("nativeMapView",new Class<?>[]{float.class,float.class,float.class,boolean.class},1.6f,.5f,.5f,true);
            call("nativeMapVisible",bool,true);call("nativeMenuButton",integer,0);
            call("nativeMenuGesture",new Class<?>[]{float.class,float.class,float.class,float.class,float.class},0f,0f,0f,0f,1f);
            call("nativeMenuMode",none);call("nativeMenuTouch",new Class<?>[]{long.class,float.class,float.class},0L,0f,0f);
            call("nativeIcon",integer,-1);call("nativePlanetIcon",new Class<?>[]{String.class},new File(c.getFilesDir(),"gamedata").getPath());
            call("nativeSecondary",bool,false);call("nativeSelect",new Class<?>[]{int.class,int.class,long.class},-1,-1,0L);
            call("nativeTouchMode",none);call("nativePause",bool,true);call("nativePause",bool,false);
            call("nativeInput",new Class<?>[]{int.class,float.class,float.class,float.class,float.class},0,0f,0f,0f,0f);
            call("nativeTouchInput",new Class<?>[]{int.class,float.class,float.class,float.class,float.class,int.class},0,0f,0f,0f,0f,0);
            evidence.put("original_jni_methods",19).put("jni_calls",calls).put("core",core);
            File report=new File(c.getFilesDir(),"native-probe.report.json");
            if(report.exists())require(report.delete(),"Cannot remove prior test report");
            String nonce=UUID.randomUUID().toString().replace("-","");
            write(new File(c.getFilesDir(),"native-probe.request"),nonce);
            Intent intent=new Intent().setClassName(c,"org.rerac.android.GameActivity").addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
            c.startActivity(intent);
            long deadline=SystemClock.uptimeMillis()+180000;JSONObject nativeResult=null;
            while(SystemClock.uptimeMillis()<deadline){
                if(report.isFile()){
                    JSONObject r=report(report);
                    require(nonce.equals(r.getString("nonce")),"Stale native report");
                    if(r.optBoolean("gpu_capture")&&r.optInt("shader_files_loaded")==r.optInt("shader_files_expected")&&r.optLong("ticks")>=30){nativeResult=r;break;}
                }
                SystemClock.sleep(500);
            }
            require(nativeResult!=null,"NativeActivity did not render and read APK shaders");
            require(core.getString("source_sha").equals(nativeResult.getJSONObject("core").getString("source_sha")),"Game process loaded another engine");
            require(!nativeResult.isNull("backend"),"Graphics backend was not reported");
            evidence.put("native_runtime",nativeResult);
            require(getUiAutomation().performGlobalAction(AccessibilityService.GLOBAL_ACTION_HOME),"Cannot background NativeActivity");
            SystemClock.sleep(1800);JSONObject paused=report(report);
            require(paused.getBoolean("paused"),"Native lifecycle did not pause");
            long ticks=paused.getLong("ticks");SystemClock.sleep(1200);
            require(report(report).getLong("ticks")==ticks,"Simulation continued while backgrounded");
            c.startActivity(intent);SystemClock.sleep(3000);
            JSONObject resumed=report(report);require(!resumed.getBoolean("paused")&&resumed.getLong("ticks")>ticks,"Native lifecycle did not resume");
            evidence.put("pause_resume",true).put("status","passed").put("gameplay_tested",false).put("physical_dual_screen_tested",false);
            write(new File(c.getFilesDir(),"native-instrumentation.json"),evidence.toString(2));
            result.putString("stream",evidence.toString(2)+"\n");finish(Activity.RESULT_OK,result);
        }catch(Throwable failure){
            try{evidence.put("status","failed").put("error",failure.toString());write(new File(getTargetContext().getFilesDir(),"native-instrumentation.json"),evidence.toString(2));}catch(Exception ignored){}
            result.putString("stream",failure.toString()+"\n");finish(Activity.RESULT_CANCELED,result);
        }
    }
}
