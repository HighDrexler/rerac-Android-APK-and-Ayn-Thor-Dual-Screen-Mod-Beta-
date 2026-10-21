package org.rerac.android;

import org.json.JSONArray;
import org.json.JSONObject;
import org.junit.Test;
import static org.junit.Assert.*;

public class QuickSelectModelTest {
    private static final int[][] CENTRES={{125,134},{179,156},{202,208},{179,260},{125,282},{71,260},{48,208},{71,156}};
    static JSONObject packet()throws Exception{
        JSONArray pages=new JSONArray();
        for(int p=0;p<3;p++){
            JSONArray slots=new JSONArray();
            for(int i=0;i<8;i++)slots.put(new JSONObject().put("id",9+p*8+i).put("x",CENTRES[i][0]).put("y",CENTRES[i][1]).put("enabled",true));
            JSONArray draws=new JSONArray().put(new JSONArray("[1,20,108,105,100,0,0,32,32,127,127,127,128]"));
            pages.put(new JSONObject().put("slots",slots).put("draws",draws));
        }
        return new JSONObject().put("generation",7).put("enabled",true).put("box",new JSONArray("[20,108,210,200]")).put("glyphs",new JSONArray()).put("pages",pages)
            .put("selection",new JSONObject().put("sequence",0).put("item",0).put("state","idle"));
    }
    private static QuickSelectModel create(JSONObject packet)throws Exception{
        QuickSelectModel model=new QuickSelectModel();model.accept(QuickSelectModel.Snapshot.parse(packet.toString()));arrange(model);return model;
    }
    private static void arrange(QuickSelectModel model){model.arrange(20,62.5f,780,535,1.25f,800,600);}
    private static float x(QuickSelectModel m,int slot){return m.geometry.screenX(CENTRES[slot][0]);}
    private static float y(QuickSelectModel m,int slot){return m.geometry.screenY(CENTRES[slot][1]);}
    static final class Sink implements QuickSelectModel.Submit {
        int calls,item;long generation;
        public long select(long gen,int selected){calls++;generation=gen;item=selected;return 41+calls;}
    }
    @Test public void everyPageSubmitsItemIdentityOnceOnRelease()throws Exception{
        QuickSelectModel m=create(packet());Sink sink=new Sink();
        for(int page=0;page<3;page++){
            if(page>0){m.nextPage();arrange(m);}
            for(int i=0;i<8;i++){
                int before=sink.calls;float x=x(m,i),y=y(m,i);
                assertTrue(m.down(9,x,y));assertEquals(before,sink.calls);
                assertTrue(m.up(9,x,y,800,600,sink));assertEquals(before+1,sink.calls);
                assertEquals(9+page*8+i,sink.item);assertEquals(7,sink.generation);
                assertFalse(m.up(9,x,y,800,600,sink));assertEquals(before+1,sink.calls);
            }
        }
    }
    @Test public void primaryAspectChangesCannotChangeSecondaryTransformOrSelection()throws Exception{
        QuickSelectModel m=create(packet());float original=x(m,0),originalY=y(m,0);Sink sink=new Sink();
        for(String primary:new String[]{"4:3","16:9","9:16","4:3"}){
            JSONObject next=packet().put("primaryDisplay",primary);m.accept(QuickSelectModel.Snapshot.parse(next.toString()));arrange(m);
            assertEquals(original,x(m,0),0);assertEquals(originalY,y(m,0),0);
            m.down(4,x(m,0),y(m,0));m.up(4,x(m,0),y(m,0),800,600,sink);
            assertEquals(9,sink.item);
        }
        assertEquals(4,sink.calls);
    }
    @Test public void originalWheelPixelAspectAndBothSourceOriginsAreInverted()throws Exception{
        QuickSelectModel m=create(packet());
        assertEquals(13f/12f,m.geometry.sx/m.geometry.sy,0.000001f);
        assertEquals(400f,x(m,0),0.0001f);
        for(int i=0;i<8;i++)assertEquals(9+i,m.geometry.hit(m.current(),x(m,i),y(m,i)).item);
        assertNull(m.geometry.hit(m.current(),400,298.75f));
        assertNull(m.geometry.hit(m.current(),-1,300));
    }
    @Test public void hybridAndPortraitLayoutsUseTheSameVisibleHitTransform()throws Exception{
        QuickSelectModel m=create(packet());Sink sink=new Sink();
        float[][] areas={{20,62.5f,336.8f,535,1.25f,800,600},{16,50,624,320,1,640,960},{30,75,930,660,1.5f,960,720}};
        for(float[] a:areas){
            m.arrange(a[0],a[1],a[2],a[3],a[4],(int)a[5],(int)a[6]);
            for(int i=0;i<8;i++){
                float x=x(m,i),y=y(m,i);assertTrue(m.down(0,x,y));m.up(0,x,y,(int)a[5],(int)a[6],sink);assertEquals(9+i,sink.item);
            }
        }
        assertEquals(24,sink.calls);
    }
    @Test public void unrelatedCompanionFieldsNeverDisableTheNewController()throws Exception{
        JSONObject nativePacket=packet();QuickSelectModel m=create(nativePacket);Sink sink=new Sink();
        nativePacket.put("legacyEnabled",false).put("lastWheel","poisoned").put("map",JSONObject.NULL).put("draws",new JSONArray());
        m.accept(QuickSelectModel.Snapshot.parse(nativePacket.toString()));arrange(m);
        assertTrue(m.available());m.down(0,x(m,2),y(m,2));m.up(0,x(m,2),y(m,2),800,600,sink);assertEquals(11,sink.item);
    }
    @Test public void failedPacketRetriesIdenticalBytesWithoutPoisoningCache()throws Exception{
        String bytes=packet().toString();QuickSelectModel m=create(packet());Sink sink=new Sink();
        m.down(0,x(m,0),y(m,0));m.failed();assertFalse(m.available());assertFalse(m.up(0,x(m,0),y(m,0),800,600,sink));
        m.accept(QuickSelectModel.Snapshot.parse(bytes));arrange(m);assertTrue(m.available());
        m.down(0,x(m,0),y(m,0));m.up(0,x(m,0),y(m,0),800,600,sink);assertEquals(1,sink.calls);
    }
    @Test public void cancellationDraggingAndSecondFingerCannotEquip()throws Exception{
        QuickSelectModel m=create(packet());Sink sink=new Sink();float x=x(m,1),y=y(m,1);
        m.down(1,x,y);m.cancel();assertFalse(m.up(1,x,y,800,600,sink));
        m.down(1,x,y);m.move(1,0,0);m.move(1,x,y);m.up(1,x,y,800,600,sink);
        m.down(1,x,y);m.extraPointer();m.up(2,x,y,800,600,sink);m.up(1,x,y,800,600,sink);
        assertEquals(0,sink.calls);
    }
    @Test public void generationChangesAndDisplayResizesCancelAnOldGesture()throws Exception{
        QuickSelectModel m=create(packet());Sink sink=new Sink();float x=x(m,1),y=y(m,1);
        m.down(1,x,y);m.accept(QuickSelectModel.Snapshot.parse(packet().put("generation",8).toString()));assertFalse(m.up(1,x,y,800,600,sink));
        arrange(m);m.down(1,x,y);m.up(1,x,y,1600,900,sink);assertEquals(0,sink.calls);
        m.down(1,x,y);m.arrange(20,40,900,500,1,960,540);assertFalse(m.up(1,x,y,960,540,sink));
    }
    @Test public void pagingAndUnavailableItemsCannotReuseAPressedItem()throws Exception{
        QuickSelectModel m=create(packet());Sink sink=new Sink();float x=x(m,1),y=y(m,1);
        m.down(1,x,y);m.nextPage();arrange(m);assertFalse(m.up(1,x,y,800,600,sink));
        m.down(1,x(m,1),y(m,1));JSONObject next=packet();next.getJSONArray("pages").getJSONObject(1).getJSONArray("slots").getJSONObject(1).put("enabled",false);
        m.accept(QuickSelectModel.Snapshot.parse(next.toString()));m.up(1,x(m,1),y(m,1),800,600,sink);assertEquals(0,sink.calls);
    }
    @Test public void sameGenerationRefreshDuringTouchDoesNotDropTheRequest()throws Exception{
        QuickSelectModel m=create(packet());Sink sink=new Sink();float x=x(m,3),y=y(m,3);
        m.down(5,x,y);
        for(int i=0;i<5;i++){m.accept(QuickSelectModel.Snapshot.parse(packet().toString()));arrange(m);}
        m.up(5,x,y,800,600,sink);assertEquals(1,sink.calls);assertEquals(12,sink.item);assertEquals(42,m.submittedSequence);
    }
    @Test public void nativeReceiptsReflectActualLifecycleWithoutOptimisticEquipment()throws Exception{
        QuickSelectModel m=create(packet());
        for(String state:new String[]{"queued","pending","equipped","rejected"}){
            JSONObject p=packet();p.getJSONObject("selection").put("sequence",24).put("item",10).put("state",state);m.accept(QuickSelectModel.Snapshot.parse(p.toString()));
            assertEquals(24,m.snapshot.sequence);assertEquals(10,m.snapshot.selectedItem);assertEquals(state,m.snapshot.selectionState);
        }
    }
}
