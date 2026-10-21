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
            pages.put(new JSONObject().put("slots",slots).put("draws",new JSONArray().put(new JSONArray("[1,20,108,105,100,0,0,32,32,127,127,127,128]"))));
        }
        return new JSONObject().put("generation",7).put("enabled",true).put("box",new JSONArray("[20,108,210,200]")).put("pages",pages);
    }
    private static QuickSelectModel create(JSONObject p)throws Exception{QuickSelectModel m=new QuickSelectModel();m.accept(QuickSelectModel.Snapshot.parse(p.toString()));arrange(m);return m;}
    private static void arrange(QuickSelectModel m){m.arrange(20,62.5f,780,535,1.25f,800,600);}
    private static float x(QuickSelectModel m,int slot){return m.geometry.screenX(CENTRES[slot][0]);}
    private static float y(QuickSelectModel m,int slot){return m.geometry.screenY(CENTRES[slot][1]);}
    static final class Sink implements QuickSelectModel.Submit{int calls,item;long generation;public long select(long gen,int selected){calls++;generation=gen;item=selected;return 41+calls;}}
    private static boolean press(QuickSelectModel m,int slot,Sink sink){return m.down(9,x(m,slot),y(m,slot),800,600,sink);}
    @Test public void allPagesActivateOnContactWithoutWaitingForRelease()throws Exception{
        QuickSelectModel m=create(packet());Sink sink=new Sink();
        for(int page=0;page<3;page++){
            if(page>0){m.nextPage();arrange(m);}
            for(int slot=0;slot<8;slot++){
                int before=sink.calls;assertTrue(press(m,slot,sink));assertEquals(before+1,sink.calls);
                assertEquals(9+page*8+slot,sink.item);assertEquals(7,sink.generation);
                m.up(9);m.up(9);assertEquals(before+1,sink.calls);
            }
        }
    }
    @Test public void cancelAndLostReleaseCannotEraseOrDuplicateAnIssuedPress()throws Exception{
        QuickSelectModel m=create(packet());Sink sink=new Sink();
        press(m,0,sink);assertEquals(1,sink.calls);m.cancel();m.up(9);assertEquals(1,sink.calls);
        press(m,1,sink);m.cancel();press(m,2,sink);assertEquals(3,sink.calls);assertEquals(11,sink.item);
    }
    @Test public void duplicateDownAndAdditionalPointerCannotRepeatEquipment()throws Exception{
        QuickSelectModel m=create(packet());Sink sink=new Sink();press(m,0,sink);
        m.down(9,x(m,1),y(m,1),800,600,sink);m.down(10,x(m,2),y(m,2),800,600,sink);
        m.up(10);assertTrue(m.hasGesture());assertEquals(1,sink.calls);m.up(9);assertFalse(m.hasGesture());
    }
    @Test public void newContactRecoversWhenThePreviousReleaseNeverArrived()throws Exception{
        QuickSelectModel m=create(packet());Sink sink=new Sink();m.begin(100);press(m,0,sink);
        m.begin(200);press(m,1,sink);assertEquals(2,sink.calls);assertEquals(10,sink.item);
        m.begin(200);press(m,1,sink);assertEquals(2,sink.calls);m.up(9);
    }
    @Test public void emptyDisabledAndOutsideSlotsNeverSubmit()throws Exception{
        JSONObject p=packet();JSONArray slots=p.getJSONArray("pages").getJSONObject(0).getJSONArray("slots");
        slots.getJSONObject(0).put("enabled",false);slots.getJSONObject(1).put("id",0);
        QuickSelectModel m=create(p);Sink sink=new Sink();assertFalse(press(m,0,sink));assertFalse(press(m,1,sink));
        assertFalse(m.down(0,-1,-1,800,600,sink));assertFalse(m.down(0,400,298.75f,800,600,sink));assertEquals(0,sink.calls);
    }
    @Test public void missingOrResizedGeometryNeverUsesAnOldHitArea()throws Exception{
        QuickSelectModel m=create(packet());Sink sink=new Sink();
        assertFalse(m.down(0,x(m,0),y(m,0),1600,900,sink));m.hide();assertFalse(m.down(0,400,100,800,600,sink));assertEquals(0,sink.calls);
    }
    @Test public void generationAndPageChangesRequireFreshLayout()throws Exception{
        QuickSelectModel m=create(packet());Sink sink=new Sink();float x=x(m,0),y=y(m,0);
        m.accept(QuickSelectModel.Snapshot.parse(packet().put("generation",8).toString()));assertFalse(m.down(0,x,y,800,600,sink));
        arrange(m);press(m,0,sink);assertEquals(8,sink.generation);m.nextPage();assertFalse(m.down(0,x,y,800,600,sink));
        arrange(m);press(m,0,sink);assertEquals(17,sink.item);
    }
    @Test public void refreshBetweenPressAndReleaseDoesNotDropEquipment()throws Exception{
        QuickSelectModel m=create(packet());Sink sink=new Sink();press(m,3,sink);
        for(int i=0;i<5;i++){m.accept(QuickSelectModel.Snapshot.parse(packet().toString()));arrange(m);}
        m.up(9);assertEquals(1,sink.calls);assertEquals(12,sink.item);assertEquals(42,m.submittedSequence);
    }
    @Test public void parseFailureDisablesNewPressAndIdenticalPacketRecovers()throws Exception{
        QuickSelectModel m=create(packet());Sink sink=new Sink();m.failed();assertFalse(press(m,0,sink));
        m.accept(QuickSelectModel.Snapshot.parse(packet().toString()));arrange(m);assertTrue(press(m,0,sink));assertEquals(1,sink.calls);
    }
    @Test public void bothSourceOriginsAndPixelAspectAreInverted()throws Exception{
        QuickSelectModel m=create(packet());assertEquals(13f/12f,m.geometry.sx/m.geometry.sy,0.000001f);
        assertEquals(400f,x(m,0),0.0001f);for(int i=0;i<8;i++)assertEquals(9+i,m.geometry.hit(m.current(),x(m,i),y(m,i)).item);
    }
    @Test public void hybridPortraitAndPrimaryAspectUseVisibleWheelCoordinates()throws Exception{
        QuickSelectModel m=create(packet());Sink sink=new Sink();float[][] areas={{20,62.5f,336.8f,535,1.25f,800,600},{16,50,624,320,1,640,960},{30,75,930,660,1.5f,960,720}};
        for(float[] a:areas)for(String primary:new String[]{"4:3","16:9"}){
            m.accept(QuickSelectModel.Snapshot.parse(packet().put("primaryAspect",primary).toString()));m.arrange(a[0],a[1],a[2],a[3],a[4],(int)a[5],(int)a[6]);
            for(int i=0;i<8;i++){assertTrue(m.down(0,x(m,i),y(m,i),(int)a[5],(int)a[6],sink));assertEquals(9+i,sink.item);m.up(0);}
        }
        assertEquals(48,sink.calls);
    }
    @Test public void nativeRejectionDoesNotPretendEquipmentSucceeded()throws Exception{
        QuickSelectModel m=create(packet());assertTrue(m.down(0,x(m,0),y(m,0),800,600,(g,i)->0));assertEquals(0,m.submittedSequence);
    }
}
