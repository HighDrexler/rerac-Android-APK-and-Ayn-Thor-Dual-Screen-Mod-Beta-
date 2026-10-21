package org.rerac.android;
import java.io.*;
import java.nio.file.*;
import java.util.zip.*;
import org.junit.Test;
import static org.junit.Assert.*;

public class UpdateArchiveTest {
    private static File archive(Path root,String name,byte[] bytes,boolean extra)throws Exception {
        File f=root.resolve("input.zip").toFile();
        try(ZipOutputStream zip=new ZipOutputStream(new FileOutputStream(f))) {
            zip.putNextEntry(new ZipEntry(name));zip.write(bytes);zip.closeEntry();
            if(extra){zip.putNextEntry(new ZipEntry("extra"));zip.write(1);zip.closeEntry();}
        }return f;
    }
    private static void rejected(String name,boolean extra,long size,String hash,boolean cancel)throws Exception {
        Path root=Files.createTempDirectory("archive-test");File target=root.resolve("out.apk").toFile();
        try {
            File zip=archive(root,name,new byte[4096],extra);
            try{UpdateArchive.extract(zip,target,size,hash,()->{if(cancel)throw new IOException("Cancelled");});fail("Accepted invalid archive");}
            catch(IOException expected){}
            assertFalse(target.exists());
        }finally{for(File f:root.toFile().listFiles())f.delete();root.toFile().delete();}
    }
    @Test public void validArchiveExtractsExactApk()throws Exception {
        Path root=Files.createTempDirectory("archive-test");byte[] bytes=new byte[4096];bytes[20]=7;
        try{File target=root.resolve("out.apk").toFile();UpdateArchive.extract(archive(root,"ReRAC.apk",bytes,false),target,bytes.length,UpdatePolicy.hash(bytes),()->{});assertArrayEquals(bytes,Files.readAllBytes(target.toPath()));}
        finally{for(File f:root.toFile().listFiles())f.delete();root.toFile().delete();}
    }
    @Test public void wrongHashRejected()throws Exception{rejected("ReRAC.apk",false,4096,UpdatePolicy.hash(new byte[8]),false);}
    @Test public void extraEntryRejected()throws Exception{rejected("ReRAC.apk",true,4096,UpdatePolicy.hash(new byte[4096]),false);}
    @Test public void archiveTraversalRejected()throws Exception{rejected("../ReRAC.apk",false,4096,UpdatePolicy.hash(new byte[4096]),false);}
    @Test public void wrongSizeRejected()throws Exception{rejected("ReRAC.apk",false,1024,UpdatePolicy.hash(new byte[4096]),false);}
    @Test public void cancellationDeletesPartialApk()throws Exception{rejected("ReRAC.apk",false,4096,UpdatePolicy.hash(new byte[4096]),true);}
}
