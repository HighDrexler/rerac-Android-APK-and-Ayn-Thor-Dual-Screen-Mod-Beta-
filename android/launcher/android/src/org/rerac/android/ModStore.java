package org.rerac.android;

import android.content.Context;
import android.content.SharedPreferences;
import android.database.Cursor;
import android.hardware.display.DisplayManager;
import android.net.Uri;
import android.provider.DocumentsContract;
import android.util.AtomicFile;
import android.view.Display;
import java.io.*;
import java.util.*;

/** Adapts the existing dual-screen setting and bounded, user-chosen mod folders. */
final class ModStore {
    static final String PREFS="launcher-mods-v1", TREE="tree_uri";
    static final class Scan { final List<ModDefinition> mods=new ArrayList<ModDefinition>();final List<String> warnings=new ArrayList<String>(); }
    private ModStore(){}
    static SharedPreferences preferences(Context c){return c.getSharedPreferences(PREFS,Context.MODE_PRIVATE);}
    static boolean secondDisplay(Context c) {
        DisplayManager dm=(DisplayManager)c.getSystemService(Context.DISPLAY_SERVICE);
        if(dm!=null)for(Display d:dm.getDisplays())if(d.getDisplayId()!=Display.DEFAULT_DISPLAY && d.getState()!=Display.STATE_OFF)return true;
        return false;
    }
    static boolean dualEnabled(Context c) {
        boolean fallback=c.getSharedPreferences("LauncherActivity",Context.MODE_PRIVATE).getBoolean("dual",secondDisplay(c));
        try(InputStream in=new AtomicFile(new File(c.getFilesDir(),"dual-screen.setting")).openRead()){return in.read()==1;}
        catch(IOException e){return fallback;}
    }
    static void setDual(Context c,boolean enabled)throws IOException {
        AtomicFile f=new AtomicFile(new File(c.getFilesDir(),"dual-screen.setting"));FileOutputStream out=null;
        try{out=f.startWrite();out.write(enabled?1:0);f.finishWrite(out);}
        catch(IOException e){f.failWrite(out);throw e;}
        if(dualEnabled(c)!=enabled)throw new IOException("The dual-screen setting could not be saved.");
    }
    static byte[] bundled(Context c)throws IOException {
        try(InputStream in=c.getAssets().open("mods/dual-screen/mod.json")){return UpdatePolicy.readBounded(in,16384);}
    }
    static Scan scan(Context c)throws Exception {
        Scan result=new Scan();String core=LauncherBuild.read(c).core;
        byte[] builtin=bundled(c);result.mods.add(ModDefinition.parse(builtin,core,true));
        File root=new File(c.getFilesDir(),"mods"),dual=new File(root,"dual-screen"),manifest=new File(dual,"mod.json");
        if(!dual.isDirectory() && !dual.mkdirs())result.warnings.add("App-managed mod storage is unavailable.");
        else if(!manifest.exists())try(FileOutputStream out=new FileOutputStream(manifest)){out.write(builtin);}
        String selected=preferences(c).getString(TREE,"");
        Set<String> ids=new HashSet<String>();ids.add("dual-screen");
        if(selected.isEmpty()) {
            File[] dirs=root.listFiles(); if(dirs!=null) {
                if(dirs.length>64)throw new IOException("The mod folder contains more than 64 entries.");
                Arrays.sort(dirs);
                for(File dir:dirs) {
                    if(!dir.isDirectory() || dir.getName().equals("dual-screen"))continue;
                    File f=new File(dir,"mod.json");if(!f.isFile())continue;
                    if(!f.getCanonicalPath().startsWith(root.getCanonicalPath()+File.separator))continue;
                    try(InputStream in=new FileInputStream(f)){add(result,ids,UpdatePolicy.readBounded(in,16384),core,dir.getName());}
                    catch(Exception e){result.warnings.add(dir.getName()+": "+message(e));}
                }
            }
        } else {
            Uri tree=Uri.parse(selected);
            try {
                List<Doc> docs=children(c,tree,DocumentsContract.getTreeDocumentId(tree));
                for(Doc dir:docs) {
                    if(!DocumentsContract.Document.MIME_TYPE_DIR.equals(dir.mime) || "dual-screen".equals(dir.name))continue;
                    for(Doc doc:children(c,tree,dir.id))if("mod.json".equals(doc.name)) {
                        try(InputStream in=c.getContentResolver().openInputStream(DocumentsContract.buildDocumentUriUsingTree(tree,doc.id))) {
                            if(in==null)throw new IOException("Could not read mod.json.");
                            add(result,ids,UpdatePolicy.readBounded(in,16384),core,dir.name);
                        }catch(Exception e){result.warnings.add(dir.name+": "+message(e));}
                    }
                }
            }catch(Exception e){result.warnings.add("Mods folder could not be read. Choose it again: "+message(e));}
        }
        return result;
    }
    private static void add(Scan scan,Set<String> ids,byte[] bytes,String core,String folder) throws Exception {
        ModDefinition mod=ModDefinition.parse(bytes,core,false);
        if(!ids.add(mod.id)) {
            for(int i=0;i<scan.mods.size();i++)if(scan.mods.get(i).id.equals(mod.id)) {
                ModDefinition old=scan.mods.get(i);
                scan.mods.set(i,new ModDefinition(old.id,old.name,old.version,old.description,old.kind,old.resolution,"Duplicate mod ID. Rename or remove the conflicting folder."));
            }
            throw new IOException("Duplicate mod ID: "+mod.id);
        }
        scan.mods.add(mod);
    }
    private static final class Doc {String id,name,mime;Doc(String a,String b,String c){id=a;name=b;mime=c;}}
    private static List<Doc> children(Context c,Uri tree,String id)throws IOException {
        List<Doc> out=new ArrayList<Doc>();
        String[] projection={DocumentsContract.Document.COLUMN_DOCUMENT_ID,DocumentsContract.Document.COLUMN_DISPLAY_NAME,DocumentsContract.Document.COLUMN_MIME_TYPE};
        try(Cursor rows=c.getContentResolver().query(DocumentsContract.buildChildDocumentsUriUsingTree(tree,id),projection,null,null,null)) {
            if(rows==null)throw new IOException("The folder provider did not respond.");
            while(rows.moveToNext()) {if(out.size()>=64)throw new IOException("A mod folder contains more than 64 entries.");out.add(new Doc(rows.getString(0),rows.getString(1),rows.getString(2)));}
        }
        return out;
    }
    static void choose(Context c,Uri tree)throws Exception {
        List<Doc> root=children(c,tree,DocumentsContract.getTreeDocumentId(tree));Uri parent=DocumentsContract.buildDocumentUriUsingTree(tree,DocumentsContract.getTreeDocumentId(tree));
        Uri dual=null;
        for(Doc d:root)if(d.name.equals("dual-screen") && d.mime.equals(DocumentsContract.Document.MIME_TYPE_DIR))dual=DocumentsContract.buildDocumentUriUsingTree(tree,d.id);
        if(dual==null)dual=DocumentsContract.createDocument(c.getContentResolver(),parent,DocumentsContract.Document.MIME_TYPE_DIR,"dual-screen");
        if(dual==null)throw new IOException("Choose a folder that allows creating mod files.");
        boolean exists=false;
        for(Doc d:children(c,tree,DocumentsContract.getDocumentId(dual)))if("mod.json".equals(d.name))exists=true;
        if(!exists) {
            Uri manifest=DocumentsContract.createDocument(c.getContentResolver(),dual,"application/json","mod.json");
            if(manifest==null)throw new IOException("Could not create the bundled mod manifest.");
            try(OutputStream out=c.getContentResolver().openOutputStream(manifest,"w")){if(out==null)throw new IOException("Could not write mod manifest.");out.write(bundled(c));}
        }
        if(!preferences(c).edit().putString(TREE,tree.toString()).commit())throw new IOException("Could not save the selected folder.");
    }
    static boolean enabled(Context c,ModDefinition m) {return m.id.equals("dual-screen")?dualEnabled(c):m.id.equals(preferences(c).getString("active_profile",""));}
    static void setEnabled(Context c,ModDefinition m,boolean value)throws IOException {
        if(value && m.problem!=null)throw new IOException(m.problem);
        if(m.id.equals("dual-screen")){setDual(c,value);return;}
        SharedPreferences p=preferences(c);File settings=new File(c.getFilesDir(),"settings.toml");
        if(value) {
            if(m.resolution==null)throw new IOException("This mod type is not supported.");
            String active=p.getString("active_profile","");
            if(!active.isEmpty() && !active.equals(m.id))throw new IOException("Turn off the active resolution profile first; both would change the same setting.");
            if(active.equals(m.id))return;
            String previous=ResolutionFile.effective(settings);
            if(!p.edit().putString("active_profile",m.id).putString("previous_resolution",previous).putString("applied_resolution",m.resolution).commit())throw new IOException("Could not save mod state.");
            try{ResolutionFile.select(settings,m.resolution);}catch(IOException e){p.edit().remove("active_profile").remove("previous_resolution").remove("applied_resolution").commit();throw e;}
        } else if(m.id.equals(p.getString("active_profile",""))) disableProfile(c);
    }
    static void disableProfile(Context c)throws IOException {
        SharedPreferences p=preferences(c);File settings=new File(c.getFilesDir(),"settings.toml");
        String applied=p.getString("applied_resolution","");
        // Do not overwrite a later user change in Port Options or launcher settings.
        if(ResolutionFile.effective(settings).equals(applied))ResolutionFile.select(settings,p.getString("previous_resolution","window"));
        if(!p.edit().remove("active_profile").remove("previous_resolution").remove("applied_resolution").commit())throw new IOException("Could not save disabled mod state.");
    }
    static String message(Exception e){String s=e.getMessage();return s==null?e.getClass().getSimpleName():s;}
}
