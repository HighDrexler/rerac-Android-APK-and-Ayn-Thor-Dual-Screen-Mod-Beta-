package org.rerac.android;

import android.app.AlertDialog;
import android.content.ClipData;
import android.content.ClipboardManager;
import android.content.Context;
import android.graphics.*;
import android.text.Layout;
import android.text.StaticLayout;
import android.text.TextPaint;
import android.text.TextUtils;
import android.view.MotionEvent;
import android.view.View;
import android.view.WindowManager;
import java.lang.reflect.Method;
import java.lang.reflect.Field;
import java.util.ArrayList;
import java.util.WeakHashMap;
import org.json.*;

/** Touch-first presentation of the game's real pause choices. No game actions are recreated here. */
final class CompanionMenu {
    private static final int BG=Color.rgb(8,15,25), CARD=Color.rgb(18,31,46), BORDER=Color.rgb(45,67,85);
    private static final int TEXT=Color.rgb(234,243,249), MUTED=Color.rgb(151,177,193), GOLD=Color.rgb(242,209,81), CYAN=Color.rgb(82,203,236);
    private static final WeakHashMap<View,State> STATES=new WeakHashMap<>();
    private static final class Hit {
        final RectF bounds;final JSONObject row;final int button;
        Hit(RectF b,JSONObject r,int key){bounds=b;row=r;button=key;}
    }
    private static final class State {
        final Paint paint=new Paint(Paint.ANTI_ALIAS_FLAG);final TextPaint text=new TextPaint(Paint.ANTI_ALIAS_FLAG);
        final ArrayList<Hit> hits=new ArrayList<>();long page=-1;float offset,downY;Hit down;boolean scrolling,active;Bitmap logo;String version;AlertDialog statusDialog;
    }
    private static State state(View v){State s=STATES.get(v);if(s==null){s=new State();STATES.put(v,s);}return s;}
    private static Object call(String name,Class<?>[] types,Object... args)throws Exception{
        Class<?> game=Class.forName("org.rerac.android.GameActivity");Method m=game.getDeclaredMethod(name,types);m.setAccessible(true);return m.invoke(null,args);
    }
    private static JSONObject snapshot()throws Exception{return new JSONObject((String)call("nativeCompanion",new Class<?>[0]));}
    private static String clean(String value){return "Paradox! This message does not exist".equals(value)?"":value;}
    private static void button(int value)throws Exception{call("nativeMenuButton",new Class<?>[]{int.class},value);}
    private static Object field(View view,String name)throws Exception{Field f=view.getClass().getDeclaredField(name);f.setAccessible(true);return f.get(view);}
    /** A failed retained poll must retry both packets, even when their text is unchanged. */
    static void pollFailed(View view,boolean ignored){
        try{
            // The retained poller writes its cache keys before all parsing and
            // texture loading have finished. Keeping those keys on failure
            // skips the next valid packet, permanently disabling quick select.
            for(String name:new String[]{"lastWheel","lastCompanion"}){
                Field f=view.getClass().getDeclaredField(name);f.setAccessible(true);f.set(view,"");
            }
            Field enabled=view.getClass().getDeclaredField("enabled");enabled.setAccessible(true);enabled.setBoolean(view,false);
            view.invalidate();
        }catch(ReflectiveOperationException error){throw new IllegalStateException("Companion poll recovery contract changed",error);}
    }
    private static JSONArray ids(JSONArray slots){JSONArray ids=new JSONArray();if(slots!=null)for(int i=0;i<slots.length();i++){JSONObject slot=slots.optJSONObject(i);if(slot!=null)ids.put(slot.optInt("id"));}return ids;}
    private static String inputReport(View view)throws Exception{
        JSONObject report=new JSONObject(NativeCore.companionInputStatus()),wheel=new JSONObject((String)call("nativeWheel",new Class<?>[0]));
        JSONArray pages=wheel.optJSONArray("pages"),items=new JSONArray();if(pages!=null)for(int i=0;i<pages.length();i++){JSONObject page=pages.optJSONObject(i);if(page!=null)items.put(ids(page.optJSONArray("slots")));}
        report.put("nativeSource",new JSONObject(NativeCore.buildInfo()).optString("source_sha")).put("nativeWheelGeneration",wheel.optLong("generation",-1)).put("nativeWheelItems",items);
        report.put("view",new JSONObject().put("generation",field(view,"generation")).put("enabled",field(view,"enabled")).put("page",field(view,"companionPage")).put("wheelPage",field(view,"wheelPage")).put("items",ids((JSONArray)field(view,"slots"))).put("width",view.getWidth()).put("height",view.getHeight()).put("display",view.getDisplay()==null?-1:view.getDisplay().getDisplayId()));
        return LauncherBuild.read(view.getContext()).footer()+"\n"+report.toString(2);
    }
    private static boolean status(View view,MotionEvent event,State state){
        if(event.getActionMasked()!=MotionEvent.ACTION_UP||event.getEventTime()-event.getDownTime()<750)return false;
        try{
            boolean home;
            if(state.active){float scale=Math.min(view.getWidth()/800f,view.getHeight()/600f);float x=(event.getX()-(view.getWidth()-800*scale)/2)/scale,y=(event.getY()-(view.getHeight()-600*scale)/2)/scale;home=new RectF(24,22,80,78).contains(x,y);}
            else{Field field=view.getClass().getDeclaredField("homeBox");field.setAccessible(true);home=((RectF)field.get(view)).contains(event.getX(),event.getY());}
            if(!home)return false;
            if(state.statusDialog!=null&&state.statusDialog.isShowing())return true;
            if(view.getWindowToken()==null)return false;
            String report=inputReport(view);
            AlertDialog dialog=new AlertDialog.Builder(view.getContext()).setTitle("Companion input status").setMessage(report).setPositiveButton("Close",null).setNeutralButton("Copy",(d,w)->{ClipboardManager clipboard=(ClipboardManager)view.getContext().getSystemService(Context.CLIPBOARD_SERVICE);if(clipboard!=null)clipboard.setPrimaryClip(ClipData.newPlainText("ReRAC companion input",report));}).create();
            // Presentation owns a different window context from the Activity.
            // A child dialog must use its actual parent view's window token.
            dialog.getWindow().setType(WindowManager.LayoutParams.TYPE_APPLICATION_ATTACHED_DIALOG);
            WindowManager.LayoutParams attrs=dialog.getWindow().getAttributes();attrs.token=view.getWindowToken();dialog.getWindow().setAttributes(attrs);
            dialog.setOnDismissListener(d->{if(state.statusDialog==dialog)state.statusDialog=null;});
            dialog.show();state.statusDialog=dialog;
            return true;
        }catch(Exception|LinkageError error){return false;}
    }
    private static void text(Canvas c,State s,String value,float x,float y,float size,int color,float width){
        s.text.setColor(color);s.text.setTextSize(size);s.text.setTypeface(Typeface.create("sans-serif-medium",Typeface.NORMAL));
        c.drawText(TextUtils.ellipsize(value,s.text,width,TextUtils.TruncateAt.END).toString(),x,y,s.text);
    }
    private static void card(Canvas c,State s,RectF r,boolean selected){
        s.paint.setStyle(Paint.Style.FILL);s.paint.setColor(selected?Color.rgb(28,52,67):CARD);c.drawRoundRect(r,12,12,s.paint);
        s.paint.setStyle(Paint.Style.STROKE);s.paint.setStrokeWidth(selected?2:1);s.paint.setColor(selected?CYAN:BORDER);c.drawRoundRect(r,12,12,s.paint);s.paint.setStyle(Paint.Style.FILL);
        if(selected){s.paint.setColor(GOLD);c.drawRoundRect(new RectF(r.left,r.top+13,r.left+4,r.bottom-13),2,2,s.paint);}
    }
    static boolean draw(View view,Canvas canvas){
        try{return render(view,canvas,snapshot());}catch(Exception error){state(view).active=false;return false;}
    }
    static boolean render(View view,Canvas canvas,JSONObject snap){
        State s=state(view);s.active=false;int saved=-1;
        try{
            JSONObject menu=snap.optJSONObject("pause");
            if(menu==null||!snap.optBoolean("menu")||!menu.optBoolean("supported"))return false;
            JSONArray choices=menu.optJSONArray("rows");
            if(choices==null||choices.length()==0)return false;
            // Native exports the choices the game's live page actually exposes,
            // including locked-menu visibility. Panel counts are not menu state.
            for(int i=0;i<choices.length();i++)if(clean(choices.getJSONObject(i).optString("label")).isEmpty())return false;
            s.active=true;long page=snap.optLong("page");if(s.page!=page){s.offset=0;s.page=page;s.down=null;}
            float scale=Math.min(view.getWidth()/800f,view.getHeight()/600f);if(scale<=0)return true;
            saved=canvas.save();canvas.drawColor(BG);canvas.translate((view.getWidth()-800*scale)/2,(view.getHeight()-600*scale)/2);canvas.scale(scale,scale);
            s.hits.clear();
            if(s.logo==null){try{int[] p=(int[])call("nativePlanetIcon",new Class<?>[]{String.class},"");if(p!=null&&p.length==p[0]*p[1]+2)s.logo=Bitmap.createBitmap(p,2,p[0],p[0],p[1],Bitmap.Config.ARGB_8888);}catch(Exception ignored){}}
            if(s.logo!=null)canvas.drawBitmap(s.logo,null,new RectF(24,22,80,78),s.paint);
            if(s.version==null)s.version=LauncherBuild.read(view.getContext()).launcher;
            text(canvas,s,"Q FORCE  /  COMPANION",100,40,15,MUTED,430);
            text(canvas,s,s.version,595,40,14,MUTED,180);
            String title=clean(menu.optString("title","Paused"));text(canvas,s,title.isEmpty()?"Game menu":title,100,77,30,TEXT,650);
            s.paint.setColor(BORDER);canvas.drawRect(24,99,776,100,s.paint);
            JSONArray rows=choices;boolean grid=menu.optBoolean("root");int count=rows.length(),cols=grid?2:1;
            JSONArray descriptions=menu.optJSONArray("descriptions");StringBuilder description=new StringBuilder();
            if(!grid&&descriptions!=null)for(int i=0;i<descriptions.length();i++){String line=clean(descriptions.optString(i));if(line.isEmpty())continue;if(description.length()>0)description.append("\n\n");description.append(line);}
            s.text.setTextSize(20);s.text.setColor(MUTED);s.text.setTypeface(Typeface.create("sans-serif",Typeface.NORMAL));
            StaticLayout paragraph=description.length()==0?null:StaticLayout.Builder.obtain(description,0,description.length(),s.text,320).setAlignment(Layout.Alignment.ALIGN_NORMAL).setIncludePad(false).setLineSpacing(6,1).build();
            float top=122,bottom=500,rowH=grid?68:66,gap=10,width=paragraph!=null?366:(752-gap*(cols-1))/cols;
            int lines=(count+cols-1)/cols;float content=Math.max(lines*(rowH+gap)-gap,paragraph==null?0:paragraph.getHeight()+36);float max=Math.max(0,content-(bottom-top));s.offset=Math.min(max,Math.max(0,s.offset));
            int clip=canvas.save();canvas.clipRect(24,top,776,bottom);
            for(int i=0;i<count;i++){
                JSONObject row=rows.getJSONObject(i);float x=24+(i%cols)*(width+gap),y=top+(i/cols)*(rowH+gap)-s.offset;
                RectF r=new RectF(x,y,x+width,y+rowH);if(r.bottom<top||r.top>bottom)continue;
                boolean enabled=row.optBoolean("enabled"),selected=row.optBoolean("selected");card(canvas,s,r,selected);
                String detail=clean(row.optString("detail"));text(canvas,s,row.optString("label"),x+20,y+(detail.isEmpty()?42:28),22,enabled?TEXT:MUTED,width-58);
                if(!detail.isEmpty())text(canvas,s,detail,x+20,y+51,15,MUTED,width-58);
                text(canvas,s,enabled?"›":"—",x+width-30,y+42,26,enabled?CYAN:MUTED,24);
                RectF hit=new RectF(r);hit.intersect(24,top,776,bottom);if(enabled&&!menu.optBoolean("busy"))s.hits.add(new Hit(hit,row,0));
            }
            if(paragraph!=null){
                RectF panel=new RectF(410,top-s.offset,776,top-s.offset+Math.max(bottom-top,paragraph.getHeight()+36));card(canvas,s,panel,false);
                int paraSave=canvas.save();canvas.translate(430,top+18-s.offset);s.text.setTextSize(20);s.text.setColor(MUTED);s.text.setTypeface(Typeface.create("sans-serif",Typeface.NORMAL));paragraph.draw(canvas);canvas.restoreToCount(paraSave);
            }
            canvas.restoreToCount(clip);
            if(max>0){s.paint.setColor(CYAN);float bar=Math.max(25,(bottom-top)*(bottom-top)/(max+bottom-top));float y=top+(bottom-top-bar)*s.offset/max;canvas.drawRoundRect(new RectF(786,y,790,y+bar),2,2,s.paint);}
            if(menu.optBoolean("busy"))text(canvas,s,"Opening…",28,524,16,MUTED,500);
            else text(canvas,s,grid?"Tap a menu to open it":"Tap a choice • Swipe to scroll",28,524,16,MUTED,720);
            if(!grid){RectF back=new RectF(24,546,238,590);card(canvas,s,back,false);text(canvas,s,"‹  Back",44,575,20,TEXT,175);s.hits.add(new Hit(back,null,16));}
            if(menu.optBoolean("canResume")&&!menu.optBoolean("busy")){
                RectF resume=new RectF(grid?24:254,546,776,590);card(canvas,s,resume,true);text(canvas,s,"Resume game",resume.left+22,575,20,GOLD,resume.width()-44);s.hits.add(new Hit(resume,null,2048));
            }
            view.postInvalidateDelayed(50);return true;
        }catch(Exception error){s.active=false;return false;}
        finally{if(saved>=0)canvas.restoreToCount(saved);}
    }
    static boolean touch(View view,MotionEvent event){
        State s=state(view);if(status(view,event,s)){s.down=null;return true;}if(!s.active)return false;
        float scale=Math.min(view.getWidth()/800f,view.getHeight()/600f);if(scale<=0)return true;
        float x=(event.getX()-(view.getWidth()-800*scale)/2)/scale,y=(event.getY()-(view.getHeight()-600*scale)/2)/scale;
        int action=event.getActionMasked();
        if(action==MotionEvent.ACTION_DOWN){s.down=null;s.scrolling=false;s.downY=y;for(Hit h:s.hits)if(h.bounds.contains(x,y)){s.down=h;break;}return true;}
        if(action==MotionEvent.ACTION_MOVE){float dy=y-s.downY;if(s.scrolling||Math.abs(dy)>10){s.scrolling=true;s.down=null;s.offset=Math.max(0,s.offset-dy);s.downY=y;view.invalidate();}return true;}
        if(action==MotionEvent.ACTION_UP){
            Hit hit=s.down;s.down=null;if(hit==null||s.scrolling||!hit.bounds.contains(x,y))return true;
            try{
                JSONObject snap=snapshot(),menu=snap.optJSONObject("pause");
                if(snap.optLong("page")!=s.page||menu==null||menu.optBoolean("busy"))return true;
                if(hit.row==null)button(hit.button);
                else if(!NativeCore.menuChoice(snap.getLong("generation"),s.page,hit.row.getLong("widget"),hit.row.getInt("index")))return true;
                view.performClick();
            }catch(Exception ignored){}return true;
        }
        if(action==MotionEvent.ACTION_CANCEL){s.down=null;s.scrolling=false;}
        return true;
    }
}
