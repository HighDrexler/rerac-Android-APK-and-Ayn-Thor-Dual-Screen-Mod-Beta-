package org.rerac.android;
import org.junit.Test;
import org.json.*;
import static org.junit.Assert.*;

public class ModDefinitionTest {
    static JSONObject profile()throws Exception{return new JSONObject().put("schema",1).put("id","low-resolution").put("name","Low resolution").put("version","1.0").put("kind","resolution-profile").put("launcher_api",1).put("core_versions",new JSONArray().put("0.3.4")).put("settings",new JSONObject().put("resolution","720p"));}
    @Test public void supportedProfile()throws Exception{ModDefinition m=ModDefinition.parse(UpdatePolicyTest.bytes(profile()),"0.3.4",false);assertNull(m.problem);assertEquals("720p",m.resolution);}
    @Test public void incompatibleCoreNotEnabled()throws Exception{assertNotNull(ModDefinition.parse(UpdatePolicyTest.bytes(profile()),"0.3.5",false).problem);}
    @Test public void unknownKindDisabled()throws Exception{assertNotNull(ModDefinition.parse(UpdatePolicyTest.bytes(profile().put("kind","native-library")),"0.3.4",false).problem);}
    @Test public void futureApiDisabled()throws Exception{assertNotNull(ModDefinition.parse(UpdatePolicyTest.bytes(profile().put("launcher_api",2)),"0.3.4",false).problem);}
    @Test public void unsafeIdentifiersRejected()throws Exception{for(String id:new String[]{"../escape","..","a/b","A","dual-screen","a..b","\\evil"}){JSONObject o=profile().put("id",id);UpdatePolicyTest.rejects(()->ModDefinition.parse(UpdatePolicyTest.bytes(o),"0.3.4",false));}}
    @Test public void invalidResolutionRejected()throws Exception{JSONObject o=profile();o.getJSONObject("settings").put("resolution","99999p");UpdatePolicyTest.rejects(()->ModDefinition.parse(UpdatePolicyTest.bytes(o),"0.3.4",false));}
    @Test public void oversizedManifestRejected(){UpdatePolicyTest.rejects(()->ModDefinition.parse(new byte[16385],"0.3.4",false));}
}
