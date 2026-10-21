package org.rerac.tests;

import android.app.Activity;
import android.app.Instrumentation;
import android.app.Presentation;
import android.content.Context;
import android.content.Intent;
import android.hardware.display.DisplayManager;
import android.hardware.display.VirtualDisplay;
import android.media.ImageReader;
import android.graphics.PixelFormat;
import android.graphics.Bitmap;
import android.graphics.Canvas;
import android.graphics.RectF;
import android.os.Bundle;
import android.os.SystemClock;
import android.view.MotionEvent;
import android.view.View;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import org.json.JSONArray;
import org.json.JSONObject;

/** Exercise the packaged source controller through the actual companion View. */
public final class CompanionUiInstrumentation extends Instrumentation {
    private Class<?> wheel;
    private static void require(boolean ok,String why){if(!ok)throw new AssertionError(why);}
    private static MotionEvent twoPointers(long time,int action,float x,float y){
        MotionEvent.PointerProperties[] properties={new MotionEvent.PointerProperties(),new MotionEvent.PointerProperties()};
        MotionEvent.PointerCoords[] coords={new MotionEvent.PointerCoords(),new MotionEvent.PointerCoords()};
        for(int i=0;i<2;i++){properties[i].id=i;properties[i].toolType=MotionEvent.TOOL_TYPE_FINGER;coords[i].x=x+i*4;coords[i].y=y+i*4;coords[i].pressure=1;coords[i].size=1;}
        return MotionEvent.obtain(time,time+20,action,2,properties,coords,0,0,1,1,0,0,android.view.InputDevice.SOURCE_TOUCHSCREEN,0);
    }
    private void put(Object view,String name,Object value)throws Exception{
        Field f=wheel.getDeclaredField(name);f.setAccessible(true);f.set(view,value);
    }
    private Object get(Object view,String name)throws Exception{
        Field f=wheel.getDeclaredField(name);f.setAccessible(true);return f.get(view);
    }
    private void failedPollRecovery(Context context,Class<?> game)throws Exception{
        Constructor<?> ctor=wheel.getDeclaredConstructor(Context.class);ctor.setAccessible(true);
        View view=(View)ctor.newInstance(context);
        Method nativeWheel=game.getDeclaredMethod("nativeWheel");nativeWheel.setAccessible(true);
        String packet=(String)nativeWheel.invoke(null);
        // The retained wheel sees an unchanged packet; fail the independent
        // companion-map refresh after its own cache key has been written.
        put(view,"lastWheel",packet);put(view,"lastCompanion","");put(view,"enabled",true);
        put(view,"mapCommands",new java.util.ArrayList<Object>(){
            private static final long serialVersionUID=1L;
            @Override public void clear(){throw new NullPointerException("Injected transient companion refresh failure");}
        });
        Runnable poll=(Runnable)get(view,"poll");android.os.Handler handler=(android.os.Handler)get(view,"handler");
        try{
            poll.run();handler.removeCallbacks(poll);
            require("".equals(get(view,"lastWheel")),"Failed poll poisoned the unchanged wheel cache");
            require("".equals(get(view,"lastCompanion")),"Failed poll poisoned the companion cache");
            require(Boolean.FALSE.equals(get(view,"enabled")),"Partially refreshed wheel accepts input");
            put(view,"mapCommands",new java.util.ArrayList<Object>());
            // Prove that replaying the identical native packet reparses wheel
            // state, not just that the recovery helper reset a boolean.
            put(view,"pageSlots",null);
            poll.run();handler.removeCallbacks(poll);
            require(packet.equals(get(view,"lastWheel")),"Valid retry did not commit the wheel packet");
            require(get(view,"pageSlots")!=null,"Unchanged wheel packet was not reparsed after failure");
            require(new JSONObject(packet).optBoolean("enabled")==((Boolean)get(view,"enabled")),"Retry did not restore native wheel availability");
        }finally{handler.removeCallbacks(poll);}
    }
    private void pauseLayouts(Context context,ClassLoader loader)throws Exception{
        Class<?> screen=Class.forName("org.rerac.android.CompanionMenu",true,loader);
        Method render=screen.getDeclaredMethod("render",View.class,Canvas.class,JSONObject.class);render.setAccessible(true);
        View host=new View(context);host.layout(0,0,800,600);
        JSONArray rows=new JSONArray();String[] choices={"Weapons","Gadgets","Quick Select","Map","Help","Options","Goodies","Save Game","Load Game"};
        for(int i=0;i<choices.length;i++)rows.put(new JSONObject().put("label",choices[i]).put("enabled",true).put("selected",i==1).put("x",180).put("y",40+i*20));
        rows.getJSONObject(0).put("detail","Paradox! This message does not exist");
        JSONObject menu=new JSONObject().put("title","Paused").put("root",true).put("rows",rows).put("supported",true).put("canResume",true);
        JSONObject snap=new JSONObject().put("menu",true).put("page",123).put("pause",menu);
        Bitmap image=Bitmap.createBitmap(800,600,Bitmap.Config.ARGB_8888);
        require(Boolean.TRUE.equals(render.invoke(null,host,new Canvas(image),snap)),"Source-built pause root did not render");
        java.io.File dir=context.getExternalFilesDir(null);require(dir!=null,"No UI evidence directory");
        try(java.io.FileOutputStream out=new java.io.FileOutputStream(new java.io.File(dir,"pause-root.png"))){image.compress(Bitmap.CompressFormat.PNG,100,out);}
        menu.put("title","Help").put("root",false).put("descriptions",new JSONArray().put("Select a topic to read the game's instructions. The companion keeps the description readable beside the menu, with Back and Resume always visible."));
        rows=new JSONArray();for(String label:new String[]{"Controls","Weapons","Gadgets","Moves"})rows.put(new JSONObject().put("label",label).put("enabled",true).put("selected",label.equals("Moves")).put("x",180).put("y",100));menu.put("rows",rows);snap.put("page",124);
        require(Boolean.TRUE.equals(render.invoke(null,host,new Canvas(image),snap)),"Pause description layout did not render");
        try(java.io.FileOutputStream out=new java.io.FileOutputStream(new java.io.File(dir,"pause-detail.png"))){image.compress(Bitmap.CompressFormat.PNG,100,out);}
        menu.put("supported",false);
        require(Boolean.FALSE.equals(render.invoke(null,host,new Canvas(image),snap)),"Complex game pages lost the original renderer");
        menu.put("supported",true);rows.getJSONObject(0).put("label","Paradox! This message does not exist");
        require(Boolean.FALSE.equals(render.invoke(null,host,new Canvas(image),snap)),"Missing imported menu labels must retain the native renderer");
        JSONArray rootChoices=new JSONArray();for(String label:new String[]{"Weapons","Gadgets","Quick Select","Items","Help","Options","Goodies"})rootChoices.put(new JSONObject().put("label",label).put("enabled",true));
        menu.put("root",true);JSONArray panels=new JSONArray();for(int i=0;i<6;i++)panels.put(new JSONArray());snap.put("panels",panels);
        Method state=screen.getDeclaredMethod("state",View.class);state.setAccessible(true);
        for(int visible=6;visible<=7;visible++){
            JSONArray exposed=new JSONArray();for(int i=0;i<visible;i++)exposed.put(rootChoices.get(i));menu.put("rows",exposed);
            require(Boolean.TRUE.equals(render.invoke(null,host,new Canvas(image),snap)),"Cannot render native root visibility");
            Object value=state.invoke(null,host);Field hits=value.getClass().getDeclaredField("hits");hits.setAccessible(true);int cards=0;
            for(Object hit:(java.util.List<?>)hits.get(value)){Field row=hit.getClass().getDeclaredField("row");row.setAccessible(true);if(row.get(hit)!=null)cards++;}
            require(cards==visible,"Companion changed the native visible menu choices");
        }
        image.recycle();
    }
    private void secondaryStatus(Context context,ClassLoader loader)throws Exception{
        Activity host=startActivitySync(new Intent().setClassName(context,"org.rerac.android.LauncherActivity").addFlags(Intent.FLAG_ACTIVITY_NEW_TASK));
        final Throwable[] error={null};final ImageReader[] image={null};final VirtualDisplay[] display={null};final Presentation[] presentation={null};final View[] companion={null};final android.app.AlertDialog[] statusDialog={null};
        java.util.concurrent.CountDownLatch attached=new java.util.concurrent.CountDownLatch(1);final java.util.concurrent.CountDownLatch statusAttached=new java.util.concurrent.CountDownLatch(1);
        try{
            runOnMainSync(()->{try{
                image[0]=ImageReader.newInstance(800,600,PixelFormat.RGBA_8888,2);
                image[0].setOnImageAvailableListener(reader->{android.media.Image frame=reader.acquireLatestImage();if(frame!=null)frame.close();},new android.os.Handler(android.os.Looper.getMainLooper()));
                DisplayManager manager=(DisplayManager)host.getSystemService(Context.DISPLAY_SERVICE);
                display[0]=manager.createVirtualDisplay("Companion input dialog test",800,600,160,image[0].getSurface(),DisplayManager.VIRTUAL_DISPLAY_FLAG_PRESENTATION|DisplayManager.VIRTUAL_DISPLAY_FLAG_OWN_CONTENT_ONLY);
                require(display[0]!=null,"Cannot create secondary display");
                presentation[0]=new Presentation(host,display[0].getDisplay());
                Constructor<?> ctor=wheel.getDeclaredConstructor(Context.class);ctor.setAccessible(true);
                companion[0]=(View)ctor.newInstance(presentation[0].getContext());
                companion[0].addOnAttachStateChangeListener(new View.OnAttachStateChangeListener(){public void onViewAttachedToWindow(View view){attached.countDown();}public void onViewDetachedFromWindow(View view){}});
                presentation[0].setContentView(companion[0]);presentation[0].getWindow().addFlags(8);presentation[0].show();
            }catch(Throwable failure){error[0]=failure;attached.countDown();}});
            require(attached.await(5,java.util.concurrent.TimeUnit.SECONDS),"Secondary view did not attach");
            if(error[0]!=null)throw new AssertionError("Cannot create secondary fixture",error[0]);
            runOnMainSync(()->{try{
                View view=companion[0];view.layout(0,0,800,600);require(view.getWindowToken()!=null,"Companion view has no secondary window token");
                Class<?> screen=Class.forName("org.rerac.android.CompanionMenu",true,loader);
                Method render=screen.getDeclaredMethod("render",View.class,Canvas.class,JSONObject.class);render.setAccessible(true);
                JSONObject menu=new JSONObject().put("title","Paused").put("root",true).put("supported",true).put("rows",new JSONArray().put(new JSONObject().put("label","Options").put("enabled",true)));
                Bitmap bitmap=Bitmap.createBitmap(800,600,Bitmap.Config.ARGB_8888);
                require(Boolean.TRUE.equals(render.invoke(null,view,new Canvas(bitmap),new JSONObject().put("menu",true).put("page",123).put("pause",menu))),"Cannot render attached companion fixture");bitmap.recycle();
                Method state=screen.getDeclaredMethod("state",View.class);state.setAccessible(true);Object value=state.invoke(null,view);
                Method status=null;for(Method m:screen.getDeclaredMethods())if(m.getName().equals("status"))status=m;
                require(status!=null,"Input status handler unavailable");status.setAccessible(true);
                long now=SystemClock.uptimeMillis();MotionEvent up=MotionEvent.obtain(now-1000,now,MotionEvent.ACTION_UP,52,50,0);
                try{require(Boolean.TRUE.equals(status.invoke(null,view,up,value)),"Input status dialog failed on the real secondary window");}finally{up.recycle();}
                Field dialog= value.getClass().getDeclaredField("statusDialog");dialog.setAccessible(true);statusDialog[0]=(android.app.AlertDialog)dialog.get(value);
                require(statusDialog[0]!=null&&statusDialog[0].isShowing(),"No attached status dialog");
                View decor=statusDialog[0].getWindow().getDecorView();decor.addOnAttachStateChangeListener(new View.OnAttachStateChangeListener(){public void onViewAttachedToWindow(View v){statusAttached.countDown();}public void onViewDetachedFromWindow(View v){}});if(decor.isAttachedToWindow())statusAttached.countDown();
            }catch(Throwable failure){error[0]=failure;}});
            if(error[0]!=null)throw new AssertionError("Secondary-display status dialog failed",error[0]);
            require(statusAttached.await(5,java.util.concurrent.TimeUnit.SECONDS),"Status dialog did not attach");
            runOnMainSync(()->{try{
                require(statusDialog[0].getWindow().getDecorView().getDisplay().getDisplayId()==display[0].getDisplay().getDisplayId(),"Status dialog opened on the wrong display");
                require(statusDialog[0].getButton(android.app.AlertDialog.BUTTON_NEUTRAL).performClick(),"Cannot press Copy");
            }catch(Throwable failure){error[0]=failure;}});
            waitForIdleSync();if(error[0]!=null)throw new AssertionError("Status Copy failed",error[0]);
            android.content.ClipboardManager clipboard=(android.content.ClipboardManager)context.getSystemService(Context.CLIPBOARD_SERVICE);
            android.content.ClipData clip=clipboard.getPrimaryClip();require(clip!=null&&clip.getItemCount()==1,"Companion status was not copied");
            String copied=clip.getItemAt(0).coerceToText(context).toString();JSONObject report=new JSONObject(copied.substring(copied.indexOf('{')));
            require(report.has("lastTap")&&report.has("nativeSource")&&report.has("nativeWheelItems")&&report.has("view"),"Copied status lacks input evidence");
        }finally{runOnMainSync(()->{if(statusDialog[0]!=null)statusDialog[0].dismiss();if(presentation[0]!=null)presentation[0].dismiss();if(display[0]!=null)display[0].release();if(image[0]!=null){image[0].setOnImageAvailableListener(null,null);image[0].close();}host.finish();});}
    }
    @Override public void onCreate(Bundle args){start();}
    @Override public void onStart(){
        Bundle result=new Bundle();final Throwable[] failure={null};final int[] taps={0};
        try{
            Context context=getTargetContext();ClassLoader loader=context.getClassLoader();
            Class<?> game=Class.forName("org.rerac.android.GameActivity",true,loader);
            Class<?> core=Class.forName("org.rerac.android.NativeCore",true,loader);
            Method info=core.getDeclaredMethod("buildInfo");info.setAccessible(true);
            JSONObject build=new JSONObject((String)info.invoke(null));
            require("0.3.6".equals(build.getString("core_version")),"Unexpected native core");
            Method icon=game.getDeclaredMethod("nativePlanetIcon",String.class);icon.setAccessible(true);
            int[] pixels=(int[])icon.invoke(null,context.getFilesDir()+"/gamedata");
            require(pixels!=null && pixels.length==128*128+2 && pixels[0]==128 && pixels[1]==128,"Q Force icon unavailable before importing game data");
            require(pixels[2]==0,"Q Force icon lost transparent border");
            wheel=Class.forName("org.rerac.android.WeaponWheel",true,loader);
            runOnMainSync(()->{try{
                Constructor<?> ctor=wheel.getDeclaredConstructor(Context.class);ctor.setAccessible(true);
                View view=(View)ctor.newInstance(context);view.layout(0,0,800,600);
                int[] clicks={0};view.setOnClickListener(v->clicks[0]++);
                int[][] centres={{125,134},{179,156},{202,208},{179,260},{125,282},{71,260},{48,208},{71,156}};
                JSONArray[] pages={new JSONArray(),new JSONArray()};
                for(int page=0;page<2;page++)for(int slot=0;slot<8;slot++){
                    pages[page].put(new JSONObject().put("id",10+page*8+slot).put("x",centres[slot][0]).put("y",centres[slot][1]).put("enabled",true));
                }
                JSONArray draws=new JSONArray().put(new JSONArray("[1,20,108,210,200,0,0,2,2,127,127,127,128]"));
                JSONObject packet=new JSONObject().put("generation",41).put("enabled",true).put("menu",false).put("mode",0).put("transition",false).put("box",new JSONArray("[20,108,210,200]")).put("glyphs",new JSONArray());
                packet.put("pages",new JSONArray().put(new JSONObject().put("slots",pages[0]).put("draws",draws)).put(new JSONObject().put("slots",pages[1]).put("draws",draws)));
                Class<?> source=Class.forName("org.rerac.android.CompanionQuickSelect",true,loader);
                Class<?> bridge=Class.forName("org.rerac.android.CompanionQuickSelect$Bridge",true,loader);
                final int[] accepted={0},lastItem={0};
                Object transport=java.lang.reflect.Proxy.newProxyInstance(loader,new Class<?>[]{bridge},(proxy,method,args)->{
                    if(method.getName().equals("snapshot"))return packet.toString();
                    if(method.getName().equals("icon"))return new int[]{2,2,-1,-1,-1,-1};
                    if(method.getName().equals("select")){require(((Long)args[0])==41,"Source controller changed generation");accepted[0]++;lastItem[0]=(Integer)args[1];return (long)accepted[0];}
                    return null;
                });
                Method attach=source.getDeclaredMethod("attach",View.class,bridge);attach.setAccessible(true);attach.invoke(null,view,transport);
                // Poison every old wheel cache. The source controller must not
                // consult these fields for painting, paging or acceptance.
                put(view,"pageSlots",new JSONArray[]{new JSONArray(),new JSONArray()});put(view,"pageDraws",new JSONArray[]{new JSONArray(),new JSONArray()});
                put(view,"slots",new JSONArray());put(view,"draws",new JSONArray());put(view,"enabled",false);put(view,"lastWheel","broken poll cache");put(view,"menu",true);put(view,"mode",3);put(view,"transitioning",true);
                try{wheel.getDeclaredMethod("quickSelect",Canvas.class,RectF.class);throw new AssertionError("Old quickSelect painter is still packaged");}catch(NoSuchMethodException expected){}
                Bitmap target=Bitmap.createBitmap(800,600,Bitmap.Config.ARGB_8888);
                for(int scenario=0;scenario<3;scenario++)for(int page=0;page<(scenario<2?1:2);page++){
                    int layout=scenario==2?1:0;
                    put(view,"collectibleInfo",scenario==1?new JSONObject().put("level",0):new JSONObject());
                    java.util.ArrayList<Object> mapCommands=new java.util.ArrayList<>();
                    if(scenario==1){
                        Class<?> command=Class.forName("org.rerac.android.WeaponWheel$DrawCommand",true,loader);
                        Constructor<?> dc=command.getDeclaredConstructor(wheel,JSONArray.class);dc.setAccessible(true);
                        mapCommands.add(dc.newInstance(view,new JSONArray("[0,[[0,0],[512,0],[0,384],[512,384]],[[0,0],[1,0],[0,1],[1,1]],-1,[0,511,0,383],false,false]")));
                    }
                    put(view,"mapCommands",mapCommands);
                    put(view,"companionPage",layout);
                    ((RectF)get(view,"mapBox")).setEmpty();((RectF)get(view,"mapControls")).setEmpty();
                    view.draw(new Canvas(target));
                    require(Boolean.FALSE.equals(get(view,"menu"))&&((Integer)get(view,"mode"))==0,"Stale retained pause state disabled source gameplay");
                    if(page==1){
                        RectF pager=(RectF)get(view,"pagerBox");long now=SystemClock.uptimeMillis();
                        MotionEvent down=MotionEvent.obtain(now,now,MotionEvent.ACTION_DOWN,pager.centerX(),pager.centerY(),0);
                        MotionEvent up=MotionEvent.obtain(now,now+20,MotionEvent.ACTION_UP,pager.centerX(),pager.centerY(),0);
                        view.dispatchTouchEvent(down);view.dispatchTouchEvent(up);down.recycle();up.recycle();view.draw(new Canvas(target));
                    }
                    RectF box=new RectF((RectF)get(view,"wheelBox"));
                    require(!box.isEmpty(),"No wheel hit bounds for layout "+layout);
                    for(int slot=0;slot<8;slot++){
                        float x=box.left+(centres[slot][0]-20f)/210f*box.width();
                        float y=box.top+(centres[slot][1]-108f)/200f*box.height();
                        int before=accepted[0];long now=SystemClock.uptimeMillis();
                        MotionEvent down=MotionEvent.obtain(now,now,MotionEvent.ACTION_DOWN,x,y,0);
                        MotionEvent up=MotionEvent.obtain(now,now+20,MotionEvent.ACTION_UP,x,y,0);
                        view.dispatchTouchEvent(down);require(accepted[0]==before,"Wheel equipped before release");
                        packet.put("primaryAspect",slot%2==0?"16:9":"4:3");put(view,"menu",true);put(view,"mode",3);put(view,"transitioning",true);view.draw(new Canvas(target));
                        view.dispatchTouchEvent(up);down.recycle();up.recycle();
                        require(accepted[0]==before+1,"Source wheel tap missed: layout="+layout+" page="+page+" slot="+slot);
                        require(lastItem[0]==10+page*8+slot,"Source wheel selected a slot ordinal instead of the visible item");
                        taps[0]++;
                    }
                }
                RectF box=(RectF)get(view,"wheelBox");float cx=box.left+(125-20f)/210f*box.width(),cy=box.top+(134-108f)/200f*box.height();
                int before=accepted[0];long now=SystemClock.uptimeMillis();
                MotionEvent down=MotionEvent.obtain(now,now,MotionEvent.ACTION_DOWN,cx,cy,0),cancel=MotionEvent.obtain(now,now+20,MotionEvent.ACTION_CANCEL,cx,cy,0),up=MotionEvent.obtain(now,now+30,MotionEvent.ACTION_UP,cx,cy,0);
                view.dispatchTouchEvent(down);view.dispatchTouchEvent(cancel);view.dispatchTouchEvent(up);down.recycle();cancel.recycle();up.recycle();
                require(accepted[0]==before,"Cancelled source gesture equipped an item");
                now=SystemClock.uptimeMillis();down=MotionEvent.obtain(now,now,MotionEvent.ACTION_DOWN,cx,cy,0);
                MotionEvent second=twoPointers(now,MotionEvent.ACTION_POINTER_DOWN|(1<<MotionEvent.ACTION_POINTER_INDEX_SHIFT),cx,cy);
                MotionEvent firstUp=twoPointers(now,MotionEvent.ACTION_POINTER_UP,cx,cy);up=MotionEvent.obtain(now,now+50,MotionEvent.ACTION_UP,cx+4,cy+4,0);
                view.dispatchTouchEvent(down);view.dispatchTouchEvent(second);view.dispatchTouchEvent(firstUp);view.dispatchTouchEvent(up);
                down.recycle();second.recycle();firstUp.recycle();up.recycle();require(accepted[0]==before,"Multitouch source gesture equipped an item");
                target.recycle();
                failedPollRecovery(context,game);
                pauseLayouts(context,loader);
            }catch(Throwable error){failure[0]=error;}});
            if(failure[0]!=null)throw new AssertionError("Actual companion view failed",failure[0]);
            secondaryStatus(context,loader);
            require(taps[0]==32,"Incomplete hybrid/dedicated page coverage");
            Method snapshot=core.getDeclaredMethod("quickSelectSnapshot");snapshot.setAccessible(true);
            JSONObject receipt=new JSONObject((String)snapshot.invoke(null));require(receipt.has("pages")&&receipt.has("generation"),"Missing source quick-select protocol");
            Method select=core.getDeclaredMethod("quickSelect",long.class,int.class);select.setAccessible(true);
            require(((Long)select.invoke(null,0L,10))==0,"Stale generation accepted by native selection queue");
            Method choice=core.getDeclaredMethod("menuChoice",long.class,long.class,long.class,int.class);choice.setAccessible(true);
            require(Boolean.FALSE.equals(choice.invoke(null,0L,123L,456L,0)),"Stale menu choice accepted");
            JSONObject evidence=new JSONObject().put("status","passed").put("scope","Packaged source controller, identity transport fixture, native rejection ABI; no imported-game gameplay").put("companion_view","source-owned CompanionQuickSelect in original companion layout").put("hybrid_taps",16).put("hybrid_map_widgets",true).put("dedicated_taps",16).put("primary_aspect_independent",true).put("old_poll_independent",true).put("stale_menu_independent",true).put("release_cancel_multitouch",true).put("old_quick_select_removed",true).put("q_force_icon",true).put("pause_layouts",true).put("secondary_status_dialog",true).put("status_clipboard",true).put("source_sha",build.getString("source_sha")).put("physical_thor_tested",false).put("live_gameplay_equipment_tested",false);
            evidence.put("failed_poll_identical_packet_recovery",true);
            result.putString("stream",evidence.toString()+"\n");finish(Activity.RESULT_OK,result);
        }catch(Throwable error){
            java.io.StringWriter trace=new java.io.StringWriter();error.printStackTrace(new java.io.PrintWriter(trace));
            result.putString("stream",trace.toString()+"\n");finish(Activity.RESULT_CANCELED,result);
        }
    }
}
