package org.rerac.android;

import android.graphics.Bitmap;
import android.graphics.Canvas;
import android.graphics.Color;
import android.graphics.ColorMatrixColorFilter;
import android.graphics.Paint;
import android.graphics.Path;
import android.graphics.Rect;
import android.graphics.RectF;
import android.view.MotionEvent;
import android.view.ScaleGestureDetector;
import android.view.View;
import java.lang.reflect.Field;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.util.HashMap;
import java.util.WeakHashMap;
import org.json.JSONArray;
import org.json.JSONObject;

/**
 * Source-owned companion wheel, input and composition. The pinned original
 * map, skill list and complex-menu painters remain the visual components.
 * Neither their poller nor its enabled/draws/cache fields govern this wheel.
 */
final class CompanionQuickSelect {
    private static final int BLUE=Color.rgb(136,168,255),GOLD=Color.rgb(255,255,32);
    private static final String[] NAV={"<","^","v",">","L1","R1","Back","Select","Cancel","Resume"};
    private static final String[] GAME_NAV={"Quick Select","Map","Pause"};
    private static final Class<?>[] NONE=new Class<?>[0];
    private static final WeakHashMap<View,Controller> STATES=new WeakHashMap<>();

    interface Bridge {
        String snapshot();
        int[] icon(int key);
        long select(long generation,int item);
    }
    private static final Bridge NATIVE=new Bridge(){
        public String snapshot(){return NativeCore.quickSelectSnapshot();}
        public int[] icon(int key){return (int[])game("nativeIcon",new Class<?>[]{int.class},key);}
        public long select(long generation,int item){return NativeCore.quickSelect(generation,item);}
    };

    static final class Controller {
        final QuickSelectModel model=new QuickSelectModel();
        final Bridge bridge;
        final Paint paint=new Paint(Paint.FILTER_BITMAP_FLAG);
        final Rect source=new Rect();
        final RectF target=new RectF();
        final Path pager=new Path();
        final HashMap<Integer,Bitmap> textures=new HashMap<>();
        final HashMap<Integer,ColorMatrixColorFilter> tints=new HashMap<>();
        long textureGeneration=-1;
        int layout=-1,dedicatedPage;
        String error="",presentationError="";
        Bitmap home;
        Controller(Bridge bridge){this.bridge=bridge;}
        void refresh(){
            try{
                QuickSelectModel.Snapshot next=QuickSelectModel.Snapshot.parse(bridge.snapshot());
                if(textureGeneration!=next.generation){textures.clear();textureGeneration=next.generation;dedicatedPage=0;}
                model.accept(next);error="";
            }catch(Exception|LinkageError failure){model.failed();error=failure.getClass().getSimpleName();}
        }
        void layout(int screen){if(layout!=screen){layout=screen;model.hide();model.showPage(screen==1?dedicatedPage:0);}}
        void nextPage(){model.nextPage();dedicatedPage=model.page;}
        Bitmap texture(int key){
            if(key==0)return null;Bitmap bitmap=textures.get(key);if(bitmap!=null)return bitmap;
            try{
                int[] data=bridge.icon(key);
                if(data==null||data.length<2||data[0]<1||data[1]<1||(long)data[0]*data[1]!=data.length-2)return null;
                bitmap=Bitmap.createBitmap(data,2,data[0],data[0],data[1],Bitmap.Config.ARGB_8888);textures.put(key,bitmap);return bitmap;
            }catch(RuntimeException|LinkageError failure){return null;}
        }
        void tint(int r,int g,int b,int a){
            int key=(r<<16)|(g<<8)|b;ColorMatrixColorFilter filter=tints.get(key);
            if(filter==null){filter=new ColorMatrixColorFilter(new float[]{r/128f,0,0,0,0,0,g/128f,0,0,0,0,0,b/128f,0,0,0,0,0,1,0});tints.put(key,filter);}
            paint.setShader(null);paint.setStyle(Paint.Style.FILL);paint.setColorFilter(filter);paint.setAlpha(Math.min(255,Math.max(0,a*255/128)));
        }
        void plain(int color){paint.setShader(null);paint.setColorFilter(null);paint.setAlpha(255);paint.setStyle(Paint.Style.FILL);paint.setColor(color);}
        void drawWheel(View view,Canvas canvas,RectF area,float unit){
            if(model.snapshot==null){model.hide();return;}
            model.arrange(area.left,area.top,area.right,area.bottom,unit,view.getWidth(),view.getHeight());
            QuickSelectModel.Geometry g=model.geometry;if(g==null||g.sx<=0||g.sy<=0)return;
            int saved=canvas.save();
            try{
                canvas.translate(g.left-g.originX*g.sx,g.top-g.originY*g.sy);canvas.scale(g.sx,g.sy);
                for(QuickSelectModel.Quad d:model.current().artwork){
                    Bitmap bitmap=texture(d.texture);if(bitmap==null||d.w==0||d.h==0)continue;
                    tint(d.r,d.g,d.b,d.a);source.set(d.u,d.v,d.u+d.tw,d.v+d.th);target.set(0,0,Math.abs(d.w),Math.abs(d.h));
                    int quad=canvas.save();canvas.translate(d.x,d.y);canvas.scale(d.w<0?-1:1,d.h<0?-1:1);
                    canvas.drawBitmap(bitmap,source,target,paint);canvas.restoreToCount(quad);
                }
            }finally{canvas.restoreToCount(saved);}
        }
        float textWidth(String text,float scale){
            if(model.snapshot==null)return 0;float width=0;
            for(int i=0;i<text.length();i++){JSONArray glyph=model.snapshot.glyphs.optJSONArray(text.charAt(i));if(glyph!=null)width+=glyph.optInt(3)*scale;}
            return width;
        }
        void label(Canvas canvas,String text,float cx,float y,float scale,int color){
            if(model.snapshot==null)return;Bitmap font=texture(-2);if(font==null)return;
            float x=cx-textWidth(text,scale)/2;tint(Color.red(color),Color.green(color),Color.blue(color),128);
            for(int i=0;i<text.length();i++){
                char ch=text.charAt(i);JSONArray glyph=model.snapshot.glyphs.optJSONArray(ch);if(glyph==null||glyph.optInt(3)==0)continue;
                int u=glyph.optInt(0),v=glyph.optInt(1);float gy=y+glyph.optInt(2)*scale;
                if(ch!=' '){source.set(u,v,u+16,v+16);target.set(x,gy,x+16*scale,gy+16*scale);canvas.drawBitmap(font,source,target,paint);}
                x+=glyph.optInt(3)*scale;
            }
        }
        void fitted(Canvas c,String text,float cx,float y,float scale,float width,int color){
            float actual=textWidth(text,scale);label(c,text,cx,y,actual>width?scale*width/actual:scale,color);
        }
        void planet(Canvas c,float x,float y,float size,int alpha){
            if(home==null){
                try{int[] data=(int[])game("nativePlanetIcon",new Class<?>[]{String.class},"");if(data!=null&&data.length>2&&data[0]>0&&data[1]>0&&(long)data[0]*data[1]==data.length-2)home=Bitmap.createBitmap(data,2,data[0],data[0],data[1],Bitmap.Config.ARGB_8888);}catch(RuntimeException|LinkageError ignored){}
            }
            if(home==null)return;plain(Color.WHITE);paint.setAlpha(alpha);target.set(x-size/2,y-size/2,x+size/2,y+size/2);c.drawBitmap(home,null,target,paint);
        }
        void pager(Canvas c,RectF bounds,float unit,int width){
            bounds.set(width-64*unit,0,width-8*unit,39*unit);plain(GOLD);int saved=c.save();
            c.translate(width-35*unit,19.5f*unit);c.scale(model.page==model.snapshot.pages.size()-1?-unit:unit,unit);pager.reset();
            for(int i=0;i<2;i++){float left=i*14-14;pager.moveTo(left,-10);pager.lineTo(left+12,0);pager.lineTo(left,10);pager.close();}
            c.drawPath(pager,paint);c.restoreToCount(saved);
        }
    }

    private static Controller state(View view){Controller s=STATES.get(view);if(s==null){s=new Controller(NATIVE);STATES.put(view,s);}return s;}
    /** Test injection changes the transport, never the production input/geometry implementation. */
    static void attach(View view,Bridge bridge){STATES.put(view,new Controller(bridge));}
    static JSONObject inputStatus(View view){
        Controller s=state(view);QuickSelectModel.Snapshot p=s.model.snapshot;
        JSONObject out=new JSONObject();try{out.put("implementation","source").put("page",s.model.page).put("available",s.model.available()).put("submittedSequence",s.model.submittedSequence).put("error",s.error).put("presentationError",s.presentationError);
            if(p!=null)out.put("generation",p.generation).put("sequence",p.sequence).put("item",p.selectedItem).put("state",p.selectionState);
        }catch(Exception ignored){}return out;
    }
    static void draw(View view,Canvas c){
        Controller s=state(view);s.refresh();
        // UI-thread polling is owned here. An unchanged valid packet is always
        // reparsed after failure; a map-refresh exception cannot disable it.
        view.postInvalidateDelayed(50);
        syncPresentation(view,s);
        boolean menu=s.model.snapshot!=null&&s.model.snapshot.menu;
        if(menu&&CompanionMenu.draw(view,c)){s.model.hide();return;}
        if(!menu)CompanionMenu.inactive(view);
        float unit=unit(view),w=view.getWidth(),h=view.getHeight();if(unit<=0)return;
        int layout=integer(view,"companionPage"),mode=s.model.snapshot==null?0:s.model.snapshot.mode;s.layout(layout);
        float bottom=h-(menu?76:42)*unit;c.drawColor(Color.rgb(8,8,16));rect(view,"pagerBox").setEmpty();rect(view,"skillButton").setEmpty();
        if(!menu&&!s.model.available()){
            s.model.hide();
            rect(view,"homeBox").setEmpty();rect(view,"wheelBox").setEmpty();rect(view,"mapBox").setEmpty();rect(view,"mapControls").setEmpty();
            s.planet(c,w/2,h/2,96*unit,255);if(mode==1||mode==2||mode==6)s.label(c,"Skip",w/2,h-30*unit,unit,BLUE);return;
        }
        JSONObject collectibles=(JSONObject)get(view,"collectibleInfo");
        boolean progress=!menu&&layout!=1&&collectibles!=null&&collectibles.has("level");
        float titleLeft=64*unit,titleRight=w-(progress?202:64)*unit;
        String title=menu?"GADGETRON - GAME MENU":layout==1?"QUICK SELECT "+(s.model.page+1)+" / "+s.model.snapshot.pages.size():"GADGETRON";
        s.fitted(c,title,(titleLeft+titleRight)/2,12*unit,unit*1.2f,titleRight-titleLeft,GOLD);
        if(progress)paintRetained(view,"collectibleHeader",new Class<?>[]{Canvas.class},c);
        rect(view,"homeBox").set(10*unit,0,60*unit,39*unit);
        s.planet(c,35*unit,19.5f*unit,32*unit,(!menu||mode==3&&!s.model.snapshot.transition)?(menu||layout!=0?190:255):90);
        if(!menu&&layout==1)s.pager(c,rect(view,"pagerBox"),unit,view.getWidth());
        s.plain(Color.rgb(112,112,64));c.drawRect(16*unit,39*unit,w-16*unit,41*unit,s.paint);
        RectF area=rect(view,"bodyArea");area.set(16*unit,50*unit,w-16*unit,bottom-10*unit);
        if(menu){s.model.hide();paintRetained(view,"menuCanvas",new Class<?>[]{Canvas.class,RectF.class},c,area);}
        else if(bool(view,"showingSkills")){s.model.hide();paintRetained(view,"skillsCanvas",new Class<?>[]{Canvas.class,RectF.class},c,area);}
        else if(layout==2){s.model.hide();paintRetained(view,"liveMap",new Class<?>[]{Canvas.class,RectF.class},c,area);}
        else if(layout==1)drawWheel(view,s,c,area,unit);
        else{
            RectF map=rect(view,"mapArea"),ring=rect(view,"ringArea");
            if(area.width()>area.height()){map.set(area.left+area.width()*0.43f,area.top,area.right,area.bottom);ring.set(area.left,area.top,map.left-8*unit,area.bottom);}
            else{map.set(area.left,area.top+area.height()*0.43f,area.right,area.bottom);ring.set(area.left,area.top,area.right,map.top-8*unit);}
            drawWheel(view,s,c,ring,unit);paintRetained(view,"liveMap",new Class<?>[]{Canvas.class,RectF.class},c,map);
        }
        s.plain(Color.rgb(112,112,64));c.drawRect(16*unit,bottom,w-16*unit,bottom+unit,s.paint);
        if(menu){for(int i=0;i<NAV.length;i++){int row=i<6?0:1,col=i<6?i:i-6,n=row==0?6:4;s.label(c,NAV[i],(col+0.5f)*w/n,bottom+(row*36+10)*unit,unit,BLUE);}}
        else for(int i=0;i<3;i++)s.label(c,i==2&&(mode==1||mode==2||mode==6)?"Skip":GAME_NAV[i],(i+0.5f)*w/3,bottom+12*unit,unit,(i==0&&layout==1||i==1&&layout==2)?GOLD:BLUE);
    }

    static boolean touch(View view,MotionEvent event){
        Controller s=state(view);s.refresh();syncPresentation(view,s);
        if(!s.model.fresh())return true;
        if(!s.model.snapshot.menu)CompanionMenu.inactive(view);
        if(CompanionMenu.touch(view,event)){s.model.cancel();return true;}
        if(s.model.snapshot.menu){s.model.hide();return (Boolean)invoke(view,"retainedMenuTouch",new Class<?>[]{MotionEvent.class},event);}
        int action=event.getActionMasked(),index=event.getActionIndex(),id=event.getPointerId(index);float x=event.getX(index),y=event.getY(index);
        if(s.model.hasGesture()){
            if(action==MotionEvent.ACTION_CANCEL){s.model.cancel();return true;}
            if(action==MotionEvent.ACTION_POINTER_DOWN){s.model.extraPointer();return true;}
            if(action==MotionEvent.ACTION_MOVE){for(int i=0;i<event.getPointerCount();i++)s.model.move(event.getPointerId(i),event.getX(i),event.getY(i));return true;}
            if(action==MotionEvent.ACTION_UP||action==MotionEvent.ACTION_POINTER_UP){
                long before=s.model.submittedSequence;
                s.model.up(id,x,y,view.getWidth(),view.getHeight(),s.bridge::select);
                if(s.model.submittedSequence!=before&&s.model.submittedSequence>0)view.performClick();view.invalidate();return true;
            }
            return true;
        }
        if(action==MotionEvent.ACTION_CANCEL){set(view,"draggingMap",false);set(view,"skillDragging",false);return true;}
        float unit=unit(view),bottom=view.getHeight()-42*unit;int mode=s.model.snapshot.mode,layout=integer(view,"companionPage");s.layout(layout);
        if(!s.model.available()){
            if(action==MotionEvent.ACTION_DOWN&&y>=bottom&&(mode==1||mode==2||mode==6)){button(2048);view.performClick();}return true;
        }
        if(event.getPointerCount()>1&&!bool(view,"draggingMap"))return true;
        if(action==MotionEvent.ACTION_DOWN&&layout==1&&rect(view,"pagerBox").contains(x,y)){s.nextPage();view.invalidate();view.performClick();return true;}
        if(action==MotionEvent.ACTION_DOWN&&rect(view,"homeBox").contains(x,y)){showPage(view,s,0);view.performClick();return true;}
        if(action==MotionEvent.ACTION_DOWN&&rect(view,"skillButton").contains(x,y)){s.model.hide();invoke(view,"showSkills",new Class<?>[]{boolean.class},!bool(view,"showingSkills"));view.performClick();return true;}
        if(bool(view,"showingSkills")&&(bool(view,"skillDragging")||y<bottom))return (Boolean)invoke(view,"skillTouch",new Class<?>[]{MotionEvent.class},event);
        if(action==MotionEvent.ACTION_DOWN&&layout!=1&&rect(view,"mapControls").contains(x,y)){
            RectF controls=rect(view,"mapControls");int control=Math.min(2,(int)((x-controls.left)/controls.width()*3));
            if(control==1){set(view,"follow",true);set(view,"centreU",get(view,"heroU"));set(view,"centreV",get(view,"heroV"));}
            else set(view,"mapZoom",Math.max(0.65f,Math.min(4f,number(view,"mapZoom")*(control==0?0.8f:1.25f))));
            set(view,"draggingMap",false);invoke(view,"sendMap",NONE);view.performClick();return true;
        }
        if(action==MotionEvent.ACTION_DOWN&&layout!=1&&rect(view,"mapBox").contains(x,y)&&!((java.util.List<?>)get(view,"mapCommands")).isEmpty()){
            set(view,"draggingMap",true);set(view,"pinchedMap",false);set(view,"panU",get(view,"centreU"));set(view,"panV",get(view,"centreV"));set(view,"downX",x);set(view,"downY",y);
        }
        if(bool(view,"draggingMap")){
            ((ScaleGestureDetector)get(view,"pinch")).onTouchEvent(event);
            if(action==MotionEvent.ACTION_MOVE&&!bool(view,"pinchedMap")&&Math.hypot(x-number(view,"downX"),y-number(view,"downY"))>unit*4){
                set(view,"follow",false);set(view,"centreU",pan(number(view,"panU"),x-number(view,"downX"),number(view,"mapZoom"),number(view,"mapScale"),bool(view,"mapMirror")));
                set(view,"centreV",pan(number(view,"panV"),y-number(view,"downY"),number(view,"mapZoom"),number(view,"mapScale"),false));invoke(view,"sendMap",NONE);
            }
            if(action==MotionEvent.ACTION_UP||action==MotionEvent.ACTION_CANCEL)set(view,"draggingMap",false);return true;
        }
        if(action==MotionEvent.ACTION_DOWN){
            if(y>=bottom){int nav=Math.max(0,Math.min(2,(int)(x/view.getWidth()*3)));if(nav<2)showPage(view,s,nav==0?1:2);else button(2048);view.performClick();return true;}
            if(layout!=2){s.model.down(id,x,y);return true;}
        }
        return true;
    }

    private static void showPage(View view,Controller s,int page){s.model.hide();invoke(view,"showPage",new Class<?>[]{int.class},page);s.layout(page);}
    private static void syncPresentation(View view,Controller s){
        if(s.model.snapshot==null)return;QuickSelectModel.Snapshot snapshot=s.model.snapshot;
        // These are outputs for the preserved map/menu painters, never inputs
        // to wheel availability or selection. Old poll failures cannot hold us
        // in a stale pause/menu state after native gameplay has resumed.
        set(view,"menu",snapshot.menu);set(view,"mode",snapshot.mode);set(view,"transitioning",snapshot.transition);
    }
    private static void drawWheel(View view,Controller s,Canvas canvas,RectF area,float unit){
        s.drawWheel(view,canvas,area,unit);QuickSelectModel.Geometry g=s.model.geometry;
        // Compatibility for the surrounding layout and its status report only.
        // Controller.drawWheel itself can also render into a primary overlay.
        if(g!=null)rect(view,"wheelBox").set(g.left,g.top,g.right,g.bottom);
    }
    private static float unit(View view){return Math.min(view.getWidth()/640f,view.getHeight()/480f);}
    private static void button(int value){game("nativeMenuButton",new Class<?>[]{int.class},value);}
    private static float pan(float value,float delta,float zoom,float scale,boolean mirror){
        try{Class<?> type=Class.forName("org.rerac.android.MapTransform");Method m=type.getDeclaredMethod("pan",float.class,float.class,float.class,float.class,boolean.class);m.setAccessible(true);return (Float)m.invoke(null,value,delta,zoom,scale,mirror);}catch(ReflectiveOperationException e){throw new IllegalStateException(e);}
    }
    private static final HashMap<Class<?>,HashMap<String,Field>> FIELDS=new HashMap<>();
    private static final HashMap<Class<?>,HashMap<String,Method>> METHODS=new HashMap<>();
    private static Field field(View view,String name)throws ReflectiveOperationException{
        HashMap<String,Field> fields=FIELDS.get(view.getClass());if(fields==null){fields=new HashMap<>();FIELDS.put(view.getClass(),fields);}
        Field f=fields.get(name);if(f==null){f=view.getClass().getDeclaredField(name);f.setAccessible(true);fields.put(name,f);}return f;
    }
    private static Object get(View view,String name){try{return field(view,name).get(view);}catch(ReflectiveOperationException e){throw new IllegalStateException("Companion component changed: "+name,e);}}
    private static void set(View view,String name,Object value){try{field(view,name).set(view,value);}catch(ReflectiveOperationException e){throw new IllegalStateException("Companion component changed: "+name,e);}}
    private static boolean bool(View view,String name){return (Boolean)get(view,name);}
    private static int integer(View view,String name){return (Integer)get(view,name);}
    private static float number(View view,String name){return (Float)get(view,name);}
    private static RectF rect(View view,String name){return (RectF)get(view,name);}
    private static Object invoke(View view,String name,Class<?>[] signature,Object...args){
        try{
            HashMap<String,Method> methods=METHODS.get(view.getClass());if(methods==null){methods=new HashMap<>();METHODS.put(view.getClass(),methods);}
            Method method=methods.get(name);if(method==null){method=view.getClass().getDeclaredMethod(name,signature);method.setAccessible(true);methods.put(name,method);}
            return method.invoke(view,args);
        }catch(ReflectiveOperationException e){Throwable cause=e instanceof InvocationTargetException?e.getCause():e;throw new IllegalStateException("Companion component failed: "+name,cause);}
    }
    private static Object game(String name,Class<?>[] signature,Object...args){
        try{Method method=Class.forName("org.rerac.android.GameActivity").getDeclaredMethod(name,signature);method.setAccessible(true);return method.invoke(null,args);}catch(ReflectiveOperationException e){throw new IllegalStateException(e);}
    }
    private static void paintRetained(View view,String name,Class<?>[] signature,Object...args){
        // Independent map/menu textures can be unavailable during a refresh.
        // Never turn that presentation failure into a wheel input lockout.
        Controller s=state(view);
        try{invoke(view,name,signature,args);if(s.presentationError.startsWith(name+":"))s.presentationError="";}
        catch(RuntimeException failure){String detail=name+":"+failure.getClass().getSimpleName();if(!detail.equals(s.presentationError))android.util.Log.w("ReRACCompanion",name+" refresh failed",failure);s.presentationError=detail;}
    }
}
