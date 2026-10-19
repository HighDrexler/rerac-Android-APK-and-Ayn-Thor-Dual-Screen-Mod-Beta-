package org.rerac.android;

import org.junit.Test;
import org.json.JSONObject;
import org.json.JSONArray;
import java.io.ByteArrayInputStream;
import java.nio.charset.StandardCharsets;
import java.security.*;
import java.util.Base64;
import static org.junit.Assert.*;

public class UpdatePolicyTest {
    interface Operation {void run()throws Exception;}
    static void rejects(Operation op){try{op.run();fail("Expected rejection");}catch(AssertionError e){throw e;}catch(Exception expected){}}
    static JSONObject valid()throws Exception{
        JSONObject r=new JSONObject();r.put("package_id",LauncherBuild.PACKAGE);r.put("signer_sha256",LauncherBuild.SIGNER);
        r.put("launcher_version","1.1.0-beta.2");r.put("core_version","0.3.4");r.put("version_code",18);r.put("bytes",221000000);
        r.put("min_sdk",26);r.put("abis",new JSONArray().put("arm64-v8a").put("x86_64"));
        r.put("sha256",new String(new char[64]).replace('\0','a'));r.put("apk_url","https://github.com/"+LauncherBuild.REPOSITORY+"/releases/download/launcher-v1.1.0-beta.2/ReRAC-Launcher.apk");
        r.put("notes","Launcher features; v16 engine retained.");return new JSONObject().put("schema",1).put("channel","development").put("release",r);
    }
    static byte[] bytes(JSONObject o){return o.toString().getBytes(StandardCharsets.UTF_8);}
    @Test public void validReleaseIsParsed()throws Exception {UpdatePolicy.Release r=UpdatePolicy.parsePayload(bytes(valid()),"development");assertEquals(18,r.code);assertEquals("0.3.4",r.core);assertEquals(2,r.abis.size());}
    @Test public void signedEmptyChannelIsValid()throws Exception {assertNull(UpdatePolicy.parsePayload(bytes(new JSONObject().put("schema",1).put("channel","stable").put("release",JSONObject.NULL)),"stable"));}
    @Test public void wrongChannelRejected()throws Exception{rejects(()->UpdatePolicy.parsePayload(bytes(valid()),"stable"));}
    @Test public void unknownChannelRejected(){rejects(()->UpdatePolicy.feedUrl("../main"));}
    @Test public void wrongPackageRejected()throws Exception{JSONObject o=valid();o.getJSONObject("release").put("package_id","another.app");rejects(()->UpdatePolicy.parsePayload(bytes(o),"development"));}
    @Test public void wrongSignerRejected()throws Exception{JSONObject o=valid();o.getJSONObject("release").put("signer_sha256","b");rejects(()->UpdatePolicy.parsePayload(bytes(o),"development"));}
    @Test public void missingReleaseRejected(){rejects(()->UpdatePolicy.parsePayload(bytes(new JSONObject().put("schema",1).put("channel","stable")),"stable"));}
    @Test public void futureSchemaRejected()throws Exception{JSONObject o=valid().put("schema",2);rejects(()->UpdatePolicy.parsePayload(bytes(o),"development"));}
    @Test public void fractionalVersionRejected()throws Exception{JSONObject o=valid();o.getJSONObject("release").put("version_code",17.5);rejects(()->UpdatePolicy.parsePayload(bytes(o),"development"));}
    @Test public void oversizedVersionRejected()throws Exception{JSONObject o=valid();o.getJSONObject("release").put("version_code",Long.MAX_VALUE);rejects(()->UpdatePolicy.parsePayload(bytes(o),"development"));}
    @Test public void apkSizeBoundEnforced()throws Exception{JSONObject o=valid();o.getJSONObject("release").put("bytes",UpdatePolicy.MAX_APK+1);rejects(()->UpdatePolicy.parsePayload(bytes(o),"development"));}
    @Test public void duplicateAbiRejected()throws Exception{JSONObject o=valid();o.getJSONObject("release").put("abis",new JSONArray().put("x86_64").put("x86_64"));rejects(()->UpdatePolicy.parsePayload(bytes(o),"development"));}
    @Test public void unknownAbiRejected()throws Exception{JSONObject o=valid();o.getJSONObject("release").put("abis",new JSONArray().put("mips"));rejects(()->UpdatePolicy.parsePayload(bytes(o),"development"));}
    @Test public void missingHashRejected()throws Exception{JSONObject o=valid();o.getJSONObject("release").remove("sha256");rejects(()->UpdatePolicy.parsePayload(bytes(o),"development"));}
    @Test public void malformedHashRejected()throws Exception{JSONObject o=valid();o.getJSONObject("release").put("sha256","not-a-checksum");rejects(()->UpdatePolicy.parsePayload(bytes(o),"development"));}
    @Test public void downgradeAndEqualRejected()throws Exception{UpdatePolicy.Release r=UpdatePolicy.parsePayload(bytes(valid()),"development");rejects(()->UpdatePolicy.compatible(r,18,36,new String[]{"arm64-v8a"}));rejects(()->UpdatePolicy.compatible(r,19,36,new String[]{"arm64-v8a"}));}
    @Test public void supportedUpgradeAccepted()throws Exception{UpdatePolicy.Release r=UpdatePolicy.parsePayload(bytes(valid()),"development");UpdatePolicy.compatible(r,17,26,new String[]{"x86_64"});}
    @Test public void sdkAndProcessorMismatchRejected()throws Exception{UpdatePolicy.Release r=UpdatePolicy.parsePayload(bytes(valid()),"development");rejects(()->UpdatePolicy.compatible(r,17,25,new String[]{"x86_64"}));rejects(()->UpdatePolicy.compatible(r,17,36,new String[]{"armeabi-v7a"}));}
    @Test public void unsafeUrlsRejected(){String[] urls={"http://github.com/a.apk","https://evil.example/a.apk","https://github.com.evil.example/a.apk","https://github.com@evil.example/a.apk","https://github.com:443/a.apk","https://github.com/"+LauncherBuild.REPOSITORY+"/releases/download/v1/../foo.apk","https://github.com/"+LauncherBuild.REPOSITORY+"/releases/download/v1/a.apk?extra=1","https://github.com/"+LauncherBuild.REPOSITORY+"/releases/download/v1/a.apk#fragment","https://github.com/"+LauncherBuild.REPOSITORY+"/releases/download/v1/%2e%2e.apk","https://github.com/"+LauncherBuild.REPOSITORY+"/releases/download/v1/a.zip"};for(String url:urls)rejects(()->UpdatePolicy.releaseUrl(url));}
    @Test public void approvedRedirectsAccepted()throws Exception{UpdatePolicy.redirectUrl("https://release-assets.githubusercontent.com/file?sig=ok");UpdatePolicy.redirectUrl("https://objects.githubusercontent.com/file");}
    @Test public void unsafeRedirectsRejected(){rejects(()->UpdatePolicy.redirectUrl("https://example.org/file"));rejects(()->UpdatePolicy.redirectUrl("http://objects.githubusercontent.com/file"));rejects(()->UpdatePolicy.redirectUrl("https://github.com/someone/else/releases/download/tag/a.apk"));}
    @Test public void strictUtf8AndSizeLimits(){rejects(()->UpdatePolicy.utf8(new byte[]{(byte)0xff}));rejects(()->UpdatePolicy.readBounded(new ByteArrayInputStream(new byte[101]),100));}
    @Test public void validSignatureAndTamperDetection()throws Exception{
        KeyPairGenerator generator=KeyPairGenerator.getInstance("RSA");generator.initialize(2048);KeyPair pair=generator.generateKeyPair();
        byte[] payload=bytes(valid());Signature s=Signature.getInstance("SHA256withRSA");s.initSign(pair.getPrivate());s.update(payload);
        JSONObject env=new JSONObject().put("algorithm","SHA256withRSA").put("payload",Base64.getEncoder().encodeToString(payload)).put("signature",Base64.getEncoder().encodeToString(s.sign()));
        assertEquals(18,UpdatePolicy.parseSigned(bytes(env),"development",pair.getPublic()).code);
        env.put("payload",Base64.getEncoder().encodeToString(bytes(valid().put("channel","stable"))));
        rejects(()->UpdatePolicy.parseSigned(bytes(env),"stable",pair.getPublic()));
    }
    @Test public void unsignedOrWrongAlgorithmRejected(){rejects(()->UpdatePolicy.parseSigned(bytes(new JSONObject()),"stable",null));rejects(()->UpdatePolicy.parseSigned(bytes(new JSONObject().put("algorithm","none")),"stable",null));}
}
