package org.rerac.android;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.net.URI;
import java.nio.ByteBuffer;
import java.nio.charset.CodingErrorAction;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.security.PublicKey;
import java.security.Signature;
import java.util.Base64;
import java.util.HashSet;
import java.util.Set;
import org.json.JSONArray;
import org.json.JSONObject;

/** Bounded, signed update metadata. Pure policy: no network, UI or installation. */
final class UpdatePolicy {
    static final int MAX_FEED = 131072;
    static final long MAX_APK = 1024L * 1024L * 1024L;
    static final long MAX_ARCHIVE = 100L * 1024L * 1024L;
    static final String PREFIX = "/" + LauncherBuild.REPOSITORY + "/update-channel3/builds/";

    static final class Release {
        final String channel, launcher, core, sha256, url, notes;
        final long code, bytes;
        final int minSdk;
        final Set<String> abis;
        final long downloadBytes;
        final String downloadSha256;
        final boolean archived;
        Release(String channel, String launcher, String core, String hash, String url,
                String notes, long code, long bytes, int minSdk, Set<String> abis,
                long downloadBytes, String downloadSha256, boolean archived) {
            this.channel=channel; this.launcher=launcher; this.core=core; this.sha256=hash;
            this.url=url; this.notes=notes; this.code=code; this.bytes=bytes; this.minSdk=minSdk; this.abis=abis;
            this.downloadBytes=downloadBytes; this.downloadSha256=downloadSha256; this.archived=archived;
        }
    }
    private UpdatePolicy() {}

    static String channel(String value) throws IOException {
        if (!"stable".equals(value) && !"development".equals(value)) throw new IOException("Unknown update channel.");
        return value;
    }

    static String feedUrl(String channel) throws IOException {
        return "https://raw.githubusercontent.com/"+LauncherBuild.REPOSITORY+"/update-channel3/"+channel(channel)+".json";
    }

    static Release parseSigned(byte[] raw, String channel, PublicKey publicKey) throws Exception {
        channel(channel);
        if (raw.length > MAX_FEED) throw new IOException("Update information is too large.");
        JSONObject env = new JSONObject(utf8(raw));
        if (!"SHA256withRSA".equals(env.getString("algorithm"))) throw new IOException("Unsupported update signature.");
        String encoded = text(env, "payload", 90000);
        byte[] payload = Base64.getDecoder().decode(encoded);
        if (payload.length > 65536) throw new IOException("Update information is too large.");
        byte[] sig = Base64.getDecoder().decode(text(env,"signature",2048));
        Signature verifier = Signature.getInstance("SHA256withRSA");
        verifier.initVerify(publicKey); verifier.update(payload);
        if (!verifier.verify(sig)) throw new IOException("Update signature did not match. Nothing was downloaded.");
        return parsePayload(payload,channel);
    }

    static Release parsePayload(byte[] payload, String channel) throws Exception {
        JSONObject o = new JSONObject(utf8(payload));
        if (integer(o,"schema",1,1)!=1 || !channel(channel).equals(o.getString("channel")))
            throw new IOException("The update belongs to another channel.");
        if (!o.has("release")) throw new IOException("Update information is incomplete.");
        if (o.isNull("release")) return null;
        JSONObject r = o.getJSONObject("release");
        if (!LauncherBuild.PACKAGE.equals(r.getString("package_id"))
                || !LauncherBuild.SIGNER.equals(r.getString("signer_sha256")))
            throw new IOException("The update is not for this Android launcher.");
        String launcher=text(r,"launcher_version",64), core=text(r,"core_version",64);
        long code=integer(r,"version_code",1,2100000000L), bytes=integer(r,"bytes",1024,MAX_APK);
        int sdk=(int)integer(r,"min_sdk",26,1000);
        String hash=text(r,"sha256",64);
        if (!hash.matches("[a-f0-9]{64}")) throw new IOException("Invalid update checksum.");
        String url=text(r,"apk_url",2048); releaseUrl(url);
        boolean archived=url.endsWith(".apk.zip");
        long downloadBytes=archived ? integer(r,"archive_bytes",1024,MAX_ARCHIVE) : bytes;
        String downloadHash=archived ? text(r,"archive_sha256",64) : hash;
        if(!downloadHash.matches("[a-f0-9]{64}")) throw new IOException("Invalid build archive checksum.");
        if(!archived && (r.has("archive_bytes") || r.has("archive_sha256")))
            throw new IOException("Unexpected build archive information.");
        String notes=r.has("notes") ? text(r,"notes",6000) : "Android launcher update.";
        JSONArray list=r.getJSONArray("abis");
        if(list.length()<1 || list.length()>4) throw new IOException("Invalid supported-device list.");
        Set<String> abis=new HashSet<String>();
        for(int i=0;i<list.length();i++) {
            String abi=list.getString(i);
            if (!abi.equals("arm64-v8a") && !abi.equals("x86_64")) throw new IOException("Unsupported update architecture.");
            if (!abis.add(abi)) throw new IOException("Duplicate update architecture.");
        }
        return new Release(channel,launcher,core,hash,url,notes,code,bytes,sdk,abis,downloadBytes,downloadHash,archived);
    }

    static void compatible(Release r,long installed,int sdk,String[] supported) throws IOException {
        if (r.code<=installed) throw new IOException("This version is already installed or is older. Downgrades are not installed.");
        if (sdk<r.minSdk) throw new IOException("This update needs Android API "+r.minSdk+" or newer.");
        for(String abi:supported) if(r.abis.contains(abi)) return;
        throw new IOException("This update does not support this device's processor.");
    }

    static URI releaseUrl(String value) throws IOException {
        URI u=secureUri(value);
        String p=u.getRawPath();
        if(!"raw.githubusercontent.com".equals(u.getHost()) || u.getRawQuery()!=null || p==null || !p.startsWith(PREFIX))
            throw new IOException("Updates must come from this project's update-channel3 branch.");
        String tail=p.substring(PREFIX.length());
        if(!tail.matches("[A-Za-z0-9][A-Za-z0-9_-]{0,95}/[A-Za-z0-9][A-Za-z0-9._-]{0,127}\\.apk(?:\\.zip)?"))
            throw new IOException("Invalid branch build address.");
        return u;
    }

    static URI redirectUrl(String value) throws IOException {
        return releaseUrl(value);
    }

    private static URI secureUri(String value) throws IOException {
        try {
            URI u=new URI(value);
            if(!"https".equals(u.getScheme()) || u.getHost()==null || u.getUserInfo()!=null
                    || u.getPort()!=-1 || u.getFragment()!=null) throw new IOException("Only secure update addresses are allowed.");
            return u;
        } catch(java.net.URISyntaxException e) { throw new IOException("Invalid update address.",e); }
    }

    static String text(JSONObject o,String key,int maximum) throws Exception {
        Object value=o.get(key);
        if(!(value instanceof String)) throw new IOException("Invalid field: "+key);
        String s=(String)value;
        if(s.isEmpty() || s.length()>maximum || s.indexOf('\u0000')>=0) throw new IOException("Invalid field: "+key);
        return s;
    }
    static long integer(JSONObject o,String key,long min,long max) throws Exception {
        Object value=o.get(key);
        if(!(value instanceof Integer) && !(value instanceof Long)) throw new IOException("Invalid number: "+key);
        long n=((Number)value).longValue();
        if(n<min || n>max) throw new IOException("Invalid number: "+key);
        return n;
    }
    static byte[] readBounded(InputStream in,int max) throws IOException {
        ByteArrayOutputStream out=new ByteArrayOutputStream(); byte[] b=new byte[8192]; int n;
        while((n=in.read(b))!=-1) { if(out.size()+n>max)throw new IOException("File is too large."); out.write(b,0,n); }
        return out.toByteArray();
    }
    static String utf8(byte[] bytes) throws IOException {
        return StandardCharsets.UTF_8.newDecoder().onMalformedInput(CodingErrorAction.REPORT)
                .onUnmappableCharacter(CodingErrorAction.REPORT).decode(ByteBuffer.wrap(bytes)).toString();
    }
    static String hex(byte[] bytes) {
        StringBuilder s=new StringBuilder(bytes.length*2);
        for(byte b:bytes) s.append(String.format(java.util.Locale.ROOT,"%02x",b&255));
        return s.toString();
    }
    static String hash(byte[] bytes) throws Exception { return hex(MessageDigest.getInstance("SHA-256").digest(bytes)); }
}
