package org.rerac.tests;

import android.app.Activity;
import android.app.Instrumentation;
import android.content.Context;
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

/** Exercise the actual retained v16 companion view, not a copy of its hit test. */
public final class CompanionUiInstrumentation extends Instrumentation {
    private Class<?> wheel;
    private static void require(boolean ok,String why){if(!ok)throw new AssertionError(why);}
    private void put(Object view,String name,Object value)throws Exception{
        Field f=wheel.getDeclaredField(name);f.setAccessible(true);f.set(view,value);
    }
    private Object get(Object view,String name)throws Exception{
        Field f=wheel.getDeclaredField(name);f.setAccessible(true);return f.get(view);
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
                JSONArray draws=new JSONArray().put(new JSONArray().put(0));
                put(view,"pageSlots",pages);put(view,"pageDraws",new JSONArray[]{draws,draws});
                put(view,"enabled",true);put(view,"menu",false);put(view,"generation",0L);
                put(view,"originX",20f);put(view,"originY",108f);put(view,"boxW",210f);put(view,"boxH",200f);
                Method select=wheel.getDeclaredMethod("selectWheelPage");select.setAccessible(true);
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
                    put(view,"companionPage",layout);put(view,"wheelPage",page);
                    ((RectF)get(view,"mapBox")).setEmpty();((RectF)get(view,"mapControls")).setEmpty();
                    select.invoke(view);view.draw(new Canvas(target));
                    RectF box=new RectF((RectF)get(view,"wheelBox"));
                    require(!box.isEmpty(),"No wheel hit bounds for layout "+layout);
                    for(int slot=0;slot<8;slot++){
                        float x=box.left+(centres[slot][0]-20f)/210f*box.width();
                        float y=box.top+(centres[slot][1]-108f)/200f*box.height();
                        int before=clicks[0];long now=SystemClock.uptimeMillis();
                        MotionEvent down=MotionEvent.obtain(now,now,MotionEvent.ACTION_DOWN,x,y,0);
                        MotionEvent up=MotionEvent.obtain(now,now+20,MotionEvent.ACTION_UP,x,y,0);
                        view.dispatchTouchEvent(down);view.dispatchTouchEvent(up);down.recycle();up.recycle();
                        require(clicks[0]==before+1,"Bottom wheel tap missed: layout="+layout+" page="+page+" slot="+slot+" clicks="+(clicks[0]-before));
                        taps[0]++;
                    }
                }
                target.recycle();
            }catch(Throwable error){failure[0]=error;}});
            if(failure[0]!=null)throw new AssertionError("Actual companion view failed",failure[0]);
            require(taps[0]==32,"Incomplete hybrid/dedicated page coverage");
            JSONObject evidence=new JSONObject().put("status","passed").put("companion_view","actual retained WeaponWheel").put("hybrid_taps",16).put("hybrid_map_widgets",true).put("dedicated_taps",16).put("q_force_icon",true).put("source_sha",build.getString("source_sha")).put("physical_thor_tested",false).put("live_gameplay_equipment_tested",false);
            result.putString("stream",evidence.toString()+"\n");finish(Activity.RESULT_OK,result);
        }catch(Throwable error){
            java.io.StringWriter trace=new java.io.StringWriter();error.printStackTrace(new java.io.PrintWriter(trace));
            result.putString("stream",trace.toString()+"\n");finish(Activity.RESULT_CANCELED,result);
        }
    }
}
