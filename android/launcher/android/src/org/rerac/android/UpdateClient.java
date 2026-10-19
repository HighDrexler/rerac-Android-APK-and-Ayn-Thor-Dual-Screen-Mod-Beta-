package org.rerac.android;

import android.content.Context;
import android.content.pm.PackageInfo;
import android.content.pm.PackageManager;
import android.os.Build;
import java.io.*;
import java.net.HttpURLConnection;
import java.net.URL;
import java.nio.file.Files;
import java.nio.file.StandardCopyOption;
import java.security.MessageDigest;
import java.security.cert.CertificateFactory;
import java.security.cert.X509Certificate;
import java.util.zip.ZipEntry;
import java.util.zip.ZipFile;
import org.json.JSONObject;

/** Runs only on the launcher's worker thread. Never modifies installed game files. */
final class UpdateClient {
    static final class Task {
        volatile boolean cancelled;
        volatile HttpURLConnection connection;
        void cancel() { cancelled=true; HttpURLConnection c=connection; if(c!=null)c.disconnect(); }
        void check() throws IOException { if(cancelled || Thread.currentThread().isInterrupted()) throw new IOException("Cancelled."); }
    }
    interface Progress { void update(long current,long total); }
    private UpdateClient() {}
    static File directory(Context context) throws IOException {
        File d=new File(context.getCacheDir(),"launcher-updates");
        if(!d.isDirectory() && !d.mkdirs()) throw new IOException("Update storage is unavailable.");
        return d;
    }
    static File verifiedFile(Context context) throws IOException { return new File(directory(context),"verified.apk"); }

    static X509Certificate installedCertificate(Context context) throws Exception {
        int flags=Build.VERSION.SDK_INT>=28 ? PackageManager.GET_SIGNING_CERTIFICATES : PackageManager.GET_SIGNATURES;
        PackageInfo p=context.getPackageManager().getPackageInfo(context.getPackageName(),flags);
        return certificate(p);
    }
    private static X509Certificate certificate(PackageInfo p) throws Exception {
        android.content.pm.Signature[] signatures;
        if(Build.VERSION.SDK_INT>=28) signatures=p.signingInfo==null ? null : p.signingInfo.getApkContentsSigners();
        else signatures=p.signatures;
        if(signatures==null || signatures.length!=1) throw new IOException("The APK signing identity is unavailable.");
        byte[] der=signatures[0].toByteArray();
        if(!LauncherBuild.SIGNER.equals(UpdatePolicy.hash(der))) throw new IOException("The APK has a different signing certificate.");
        return (X509Certificate)CertificateFactory.getInstance("X.509").generateCertificate(new ByteArrayInputStream(der));
    }

    static UpdatePolicy.Release check(Context context,String channel,Task task) throws Exception {
        task.check();
        X509Certificate cert=installedCertificate(context);
        HttpURLConnection c=open(new URL(UpdatePolicy.feedUrl(channel)),task);
        try {
            int status=c.getResponseCode();
            if(status==404) throw new IOException("This update channel has not been published yet. Your installed game still works.");
            if(status!=200) throw new IOException("Update server returned HTTP "+status+". Try again later.");
            if(c.getContentLengthLong()>UpdatePolicy.MAX_FEED) throw new IOException("Update information is too large.");
            byte[] data;
            try(InputStream in=c.getInputStream()) { data=UpdatePolicy.readBounded(in,UpdatePolicy.MAX_FEED); }
            task.check();
            return UpdatePolicy.parseSigned(data,channel,cert.getPublicKey());
        } finally { c.disconnect(); task.connection=null; }
    }

    private static HttpURLConnection open(URL url,Task task) throws Exception {
        task.check();
        HttpURLConnection c=(HttpURLConnection)url.openConnection();
        c.setInstanceFollowRedirects(false); c.setConnectTimeout(15000); c.setReadTimeout(15000);
        c.setRequestProperty("User-Agent","ReRAC-Android-Launcher/1.1");
        c.setRequestProperty("Accept-Encoding","identity"); c.setUseCaches(false);
        task.connection=c; task.check();
        return c;
    }

    private static HttpURLConnection downloadConnection(String url,Task task) throws Exception {
        UpdatePolicy.releaseUrl(url);
        for(int hop=0;hop<=5;hop++) {
            HttpURLConnection c=open(new URL(url),task);
            int status;
            try { status=c.getResponseCode(); } catch(Exception e) { c.disconnect(); throw e; }
            if(status==200) return c;
            if(status==301 || status==302 || status==303 || status==307 || status==308) {
                String location=c.getHeaderField("Location"); c.disconnect();
                if(location==null) throw new IOException("The download redirect is incomplete.");
                url=new URL(new URL(url),location).toExternalForm(); UpdatePolicy.redirectUrl(url); continue;
            }
            c.disconnect(); throw new IOException("Download server returned HTTP "+status+".");
        }
        throw new IOException("Too many download redirects.");
    }

    static synchronized File download(Context context,UpdatePolicy.Release release,Task task,Progress progress) throws Exception {
        LauncherBuild installed=LauncherBuild.read(context);
        UpdatePolicy.compatible(release,installed.code,Build.VERSION.SDK_INT,Build.SUPPORTED_ABIS);
        File dir=directory(context), part=new File(dir,"download.part"), candidate=verifiedFile(context);
        if(dir.getUsableSpace()<release.bytes*2+64L*1024*1024) throw new IOException("Not enough free space for download and installation.");
        if(part.exists() && !part.delete()) throw new IOException("Could not clear an interrupted download.");
        HttpURLConnection c=null;
        try {
            c=downloadConnection(release.url,task);
            long length=c.getContentLengthLong();
            if(length!=-1 && length!=release.bytes) throw new IOException("Download size does not match the signed release.");
            MessageDigest digest=MessageDigest.getInstance("SHA-256"); long count=0,last=0;
            try(InputStream in=c.getInputStream(); FileOutputStream out=new FileOutputStream(part)) {
                byte[] b=new byte[65536]; int n;
                while((n=in.read(b))!=-1) {
                    task.check(); count+=n;
                    if(count>release.bytes) throw new IOException("Download exceeded its signed size.");
                    out.write(b,0,n); digest.update(b,0,n);
                    long now=System.nanoTime();
                    if(now-last>=150000000L) { progress.update(count,release.bytes);last=now; }
                }
                out.getFD().sync();
            }
            task.check();
            if(count!=release.bytes || !UpdatePolicy.hex(digest.digest()).equals(release.sha256))
                throw new IOException("Download verification failed. The installed app was not changed.");
            verifyPackage(context,part,release); task.check();
            Files.move(part.toPath(),candidate.toPath(),StandardCopyOption.ATOMIC_MOVE,StandardCopyOption.REPLACE_EXISTING);
            progress.update(count,release.bytes);
            return candidate;
        } finally {
            if(c!=null)c.disconnect(); task.connection=null;
            if(part.exists())part.delete();
        }
    }

    static void verifyReady(Context c,File file,UpdatePolicy.Release release,Task task) throws Exception {
        if(!file.isFile() || file.length()!=release.bytes) throw new IOException("The downloaded APK is unavailable. Download it again.");
        MessageDigest md=MessageDigest.getInstance("SHA-256");
        try(InputStream in=new FileInputStream(file)) { byte[] b=new byte[65536];int n;while((n=in.read(b))!=-1){task.check();md.update(b,0,n);} }
        if(!UpdatePolicy.hex(md.digest()).equals(release.sha256))throw new IOException("The downloaded APK changed. Download it again.");
        verifyPackage(c,file,release);task.check();
    }

    static void verifyPackage(Context c,File file,UpdatePolicy.Release release) throws Exception {
        UpdatePolicy.compatible(release,LauncherBuild.read(c).code,Build.VERSION.SDK_INT,Build.SUPPORTED_ABIS);
        int flags=Build.VERSION.SDK_INT>=28 ? PackageManager.GET_SIGNING_CERTIFICATES : PackageManager.GET_SIGNATURES;
        PackageInfo p=c.getPackageManager().getPackageArchiveInfo(file.getAbsolutePath(),flags);
        if(p==null || !LauncherBuild.PACKAGE.equals(p.packageName))throw new IOException("This APK is not a ReRAC Android update.");
        long code=Build.VERSION.SDK_INT>=28 ? p.getLongVersionCode() : p.versionCode;
        if(code!=release.code || !release.launcher.equals(p.versionName) || p.applicationInfo==null
                || p.applicationInfo.minSdkVersion!=release.minSdk)throw new IOException("APK version does not match the signed update information.");
        certificate(p); installedCertificate(c);
        try(ZipFile zip=new ZipFile(file)) {
            ZipEntry metadata=zip.getEntry("assets/"+LauncherBuild.ASSET);
            if(metadata==null || metadata.getSize()>16384)throw new IOException("The APK is missing build metadata.");
            JSONObject o;
            try(InputStream in=zip.getInputStream(metadata)){o=new JSONObject(UpdatePolicy.utf8(UpdatePolicy.readBounded(in,16384)));}
            if(!release.core.equals(o.getString("core_version")) || !release.launcher.equals(o.getString("launcher_version"))
                    || release.code!=o.getLong("version_code") || !LauncherBuild.PACKAGE.equals(o.getString("package_id")))
                throw new IOException("The bundled engine version does not match this release.");
            for(String abi:release.abis)if(zip.getEntry("lib/"+abi+"/librc_engine.so")==null)
                throw new IOException("The APK is missing a promised native engine.");
        }
    }
}
