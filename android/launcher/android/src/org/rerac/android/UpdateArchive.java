package org.rerac.android;

import java.io.*;
import java.security.MessageDigest;
import java.util.zip.ZipEntry;
import java.util.zip.ZipFile;

/** Extract only the one signed, size-bounded APK. Never resolve archive paths. */
final class UpdateArchive {
    interface Check { void run() throws IOException; }
    private UpdateArchive() {}
    static void extract(File archive, File target, long expectedBytes, String hash, Check check) throws Exception {
        boolean complete=false;
        try(ZipFile zip=new ZipFile(archive)) {
            if(zip.size()!=1) throw new IOException("The build archive must contain one APK.");
            ZipEntry entry=zip.entries().nextElement();
            if(!"ReRAC.apk".equals(entry.getName()) || entry.isDirectory() || entry.getSize()!=expectedBytes)
                throw new IOException("The build archive does not match the signed update.");
            MessageDigest digest=MessageDigest.getInstance("SHA-256"); long count=0;
            try(InputStream in=zip.getInputStream(entry);FileOutputStream out=new FileOutputStream(target)) {
                byte[] buffer=new byte[65536];int n;
                while((n=in.read(buffer))!=-1) {
                    check.run();count+=n;
                    if(count>expectedBytes) throw new IOException("The unpacked APK is too large.");
                    out.write(buffer,0,n);digest.update(buffer,0,n);
                }
                out.getFD().sync();
            }
            check.run();
            if(count!=expectedBytes || !UpdatePolicy.hex(digest.digest()).equals(hash))
                throw new IOException("The unpacked APK failed verification.");
            complete=true;
        } finally {if(!complete && target.exists())target.delete();}
    }
}
