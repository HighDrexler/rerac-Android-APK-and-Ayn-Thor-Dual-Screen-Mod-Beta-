package org.rerac.android;

import java.io.IOException;
import org.json.JSONArray;
import org.json.JSONObject;

/** Versioned configuration-mod contract. No downloaded native code or scripts. */
final class ModDefinition {
    final String id,name,version,description,kind,resolution,problem;
    ModDefinition(String id,String name,String version,String description,String kind,String resolution,String problem) {
        this.id=id;this.name=name;this.version=version;this.description=description;
        this.kind=kind;this.resolution=resolution;this.problem=problem;
    }
    static ModDefinition parse(byte[] bytes,String core,boolean bundled) throws Exception {
        if(bytes.length>16384)throw new IOException("Mod manifest is too large.");
        JSONObject o=new JSONObject(UpdatePolicy.utf8(bytes));
        UpdatePolicy.integer(o,"schema",1,1);
        String id=UpdatePolicy.text(o,"id",64);
        if(!id.matches("[a-z0-9][a-z0-9.-]{1,63}") || id.contains(".."))throw new IOException("Invalid mod ID.");
        if("dual-screen".equals(id)!=bundled)throw new IOException("The bundled dual-screen ID is reserved.");
        String name=UpdatePolicy.text(o,"name",100),version=UpdatePolicy.text(o,"version",48),kind=UpdatePolicy.text(o,"kind",48);
        String description=o.has("description")?UpdatePolicy.text(o,"description",1500):"User-installed mod.";
        String problem=null,resolution=null;
        long api=UpdatePolicy.integer(o,"launcher_api",1,1000);
        if(api!=1)problem="Needs a different launcher mod API.";
        JSONArray cores=o.getJSONArray("core_versions");
        if(cores.length()<1 || cores.length()>32)throw new IOException("Invalid engine compatibility list.");
        boolean matches=false;
        for(int i=0;i<cores.length();i++)if(core.equals(cores.getString(i)))matches=true;
        if(!matches)problem="Not marked compatible with ReRAC "+core+".";
        if(bundled){if(!"built-in".equals(kind))throw new IOException("Invalid bundled mod.");}
        else if("resolution-profile".equals(kind)) {
            resolution=UpdatePolicy.text(o.getJSONObject("settings"),"resolution",16);
            if(!resolution.equals(ResolutionFile.normalize(resolution)))throw new IOException("Unsupported mod resolution.");
        } else problem="This mod type is not supported. Native-code and script mods cannot be loaded.";
        return new ModDefinition(id,name,version,description,kind,resolution,problem);
    }
}
