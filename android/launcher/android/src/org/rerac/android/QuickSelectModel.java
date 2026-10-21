package org.rerac.android;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import org.json.JSONArray;
import org.json.JSONException;
import org.json.JSONObject;

/** Independent wheel state. No primary-display dimensions or retained UI fields enter this model. */
final class QuickSelectModel {
    static final float PIXEL_ASPECT = 13f / 12f;
    static final float HIT_RADIUS = 24f;

    static final class Slot {
        final int item;
        final float x, y;
        final boolean enabled;
        Slot(int item,float x,float y,boolean enabled){this.item=item;this.x=x;this.y=y;this.enabled=enabled;}
    }
    static final class Quad {
        final int texture,u,v,tw,th,r,g,b,a;
        final float x,y,w,h;
        Quad(JSONArray d)throws JSONException{
            if(d.length()!=13)throw new JSONException("Invalid wheel artwork");
            texture=d.getInt(0);x=number(d,1);y=number(d,2);w=number(d,3);h=number(d,4);
            u=d.getInt(5);v=d.getInt(6);tw=d.getInt(7);th=d.getInt(8);
            r=d.getInt(9);g=d.getInt(10);b=d.getInt(11);a=d.getInt(12);
            if(tw<0||th<0)throw new JSONException("Invalid wheel texture bounds");
        }
    }
    static final class Page {
        final List<Slot> slots;
        final List<Quad> artwork;
        Page(List<Slot> slots,List<Quad> artwork){this.slots=Collections.unmodifiableList(slots);this.artwork=Collections.unmodifiableList(artwork);}
    }
    static final class Snapshot {
        final long generation,sequence;
        final int selectedItem;
        final String selectionState;
        final boolean enabled,menu,transition;
        final int mode;
        final float x,y,width,height;
        final List<Page> pages;
        final JSONArray glyphs;
        Snapshot(JSONObject root)throws JSONException{
            generation=root.getLong("generation");enabled=root.getBoolean("enabled");
            menu=root.optBoolean("menu");mode=root.optInt("mode");transition=root.optBoolean("transition");
            JSONArray box=root.getJSONArray("box");
            if(box.length()!=4)throw new JSONException("Invalid wheel bounds");
            x=number(box,0);y=number(box,1);width=number(box,2);height=number(box,3);
            if(generation<0||width<=0||height<=0||width>4096||height>4096)throw new JSONException("Invalid wheel coordinate system");
            JSONArray src=root.getJSONArray("pages");
            if(src.length()<1||src.length()>16)throw new JSONException("Invalid wheel pages");
            ArrayList<Page> parsed=new ArrayList<>();
            for(int p=0;p<src.length();p++){
                JSONObject page=src.getJSONObject(p);JSONArray slots=page.getJSONArray("slots"),draws=page.getJSONArray("draws");
                if(slots.length()>8||draws.length()>2048)throw new JSONException("Oversized wheel page");
                ArrayList<Slot> entries=new ArrayList<>();ArrayList<Quad> artwork=new ArrayList<>();
                for(int i=0;i<slots.length();i++){
                    JSONObject s=slots.getJSONObject(i);int item=s.getInt("id");
                    if(item<0)throw new JSONException("Invalid wheel item");
                    entries.add(new Slot(item,finite(s.getDouble("x")),finite(s.getDouble("y")),s.getBoolean("enabled")&&item!=0));
                }
                for(int i=0;i<draws.length();i++)artwork.add(new Quad(draws.getJSONArray(i)));
                parsed.add(new Page(entries,artwork));
            }
            pages=Collections.unmodifiableList(parsed);JSONArray font=root.optJSONArray("glyphs");glyphs=font==null?new JSONArray():font;
            JSONObject receipt=root.optJSONObject("selection");sequence=receipt==null?0:receipt.optLong("sequence");
            selectedItem=receipt==null?0:receipt.optInt("item");selectionState=receipt==null?"idle":receipt.optString("state","idle");
        }
        static Snapshot parse(String packet)throws JSONException{return new Snapshot(new JSONObject(packet));}
    }
    private static float number(JSONArray a,int n)throws JSONException{return finite(a.getDouble(n));}
    private static float finite(double value)throws JSONException{
        float f=(float)value;if(!Float.isFinite(f))throw new JSONException("Non-finite wheel coordinate");return f;
    }

    /** The exact original companion wheel transform, shared by artwork and hit testing. */
    static final class Geometry {
        final float left,top,right,bottom,sx,sy,originX,originY;
        Geometry(Snapshot s,float areaLeft,float areaTop,float areaRight,float areaBottom,float unit){
            float scale=Math.min((areaRight-areaLeft-unit*8f)/((s.width+14f)*PIXEL_ASPECT),(areaBottom-areaTop)/(s.height+14f));
            sx=Math.max(0,scale)*PIXEL_ASPECT;sy=Math.max(0,scale);originX=s.x;originY=s.y;
            left=(areaLeft+areaRight-s.width*sx)/2f;top=(areaTop+areaBottom-s.height*sy)/2f;
            right=left+s.width*sx;bottom=top+s.height*sy;
        }
        boolean contains(float x,float y){return sx>0&&sy>0&&x>=left&&x<right&&y>=top&&y<bottom;}
        float screenX(float x){return left+(x-originX)*sx;}
        float screenY(float y){return top+(y-originY)*sy;}
        Slot hit(Page page,float x,float y){
            if(!contains(x,y))return null;
            float gx=originX+(x-left)/sx,gy=originY+(y-top)/sy,best=HIT_RADIUS*HIT_RADIUS;Slot hit=null;
            for(Slot slot:page.slots){
                if(!slot.enabled)continue;float dx=gx-slot.x,dy=gy-slot.y,distance=dx*dx+dy*dy;
                if(distance<best){best=distance;hit=slot;}
            }
            return hit;
        }
    }

    interface Submit { long select(long generation,int item); }
    Snapshot snapshot;
    Geometry geometry;
    int page;
    long submittedSequence;
    private boolean fresh,tracking;
    private int pointer=-1;
    private long contactTime=-1;
    private int width,height;

    void accept(Snapshot next){
        if(snapshot==null||snapshot.generation!=next.generation){page=0;cancel();geometry=null;}
        snapshot=next;fresh=true;
        if(page>=next.pages.size()){page=0;cancel();geometry=null;}
    }
    void failed(){fresh=false;cancel();}
    boolean fresh(){return fresh&&snapshot!=null;}
    boolean available(){return fresh&&snapshot!=null&&snapshot.enabled;}
    Page current(){return snapshot.pages.get(page);}
    void nextPage(){if(snapshot!=null){page=(page+1)%snapshot.pages.size();cancel();geometry=null;}}
    void showPage(int next){next=snapshot==null?0:Math.max(0,Math.min(next,snapshot.pages.size()-1));if(next!=page){page=next;cancel();geometry=null;}}
    void arrange(float left,float top,float right,float bottom,float unit,int viewWidth,int viewHeight){
        if(width!=viewWidth||height!=viewHeight){cancel();width=viewWidth;height=viewHeight;}
        geometry=snapshot==null?null:new Geometry(snapshot,left,top,right,bottom,unit);
    }
    void hide(){geometry=null;cancel();}
    boolean hasGesture(){return tracking;}
    /** A new Android DOWN starts a new stream even if an earlier UP was lost. */
    void begin(long time){if(contactTime!=time){cancel();contactTime=time;}}
    int submittedItem;
    /** Equipment buttons activate on contact, exactly once for the touch stream. */
    boolean down(int id,float x,float y,int viewWidth,int viewHeight,Submit submit){
        if(tracking)return true;
        if(!available()||geometry==null||width!=viewWidth||height!=viewHeight)return false;
        Slot slot=geometry.hit(current(),x,y);
        if(slot==null)return false;
        tracking=true;pointer=id;submittedItem=slot.item;
        submittedSequence=submit.select(snapshot.generation,slot.item);
        return true;
    }
    /** A release finishes ownership; it never sends equipment a second time. */
    void up(int id){if(tracking&&pointer==id)cancel();}
    void cancel(){tracking=false;pointer=-1;}
}
