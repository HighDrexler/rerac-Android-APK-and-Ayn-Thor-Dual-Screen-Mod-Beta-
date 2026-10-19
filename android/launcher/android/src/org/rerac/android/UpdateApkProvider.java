package org.rerac.android;

import android.content.ContentProvider;
import android.content.ContentValues;
import android.database.Cursor;
import android.database.MatrixCursor;
import android.net.Uri;
import android.os.ParcelFileDescriptor;
import android.provider.OpenableColumns;
import java.io.File;
import java.io.FileNotFoundException;

/** Read-only, single-file provider. Never exposes saves, data, directories or paths. */
public final class UpdateApkProvider extends ContentProvider {
    @Override public boolean onCreate(){return true;}
    private File file(Uri uri) throws FileNotFoundException {
        if(getContext()==null || !(getContext().getPackageName()+".updates").equals(uri.getAuthority())
                || !"/verified.apk".equals(uri.getPath()) || uri.getQuery()!=null || uri.getFragment()!=null)
            throw new FileNotFoundException("Unknown update file");
        try { File f=UpdateClient.verifiedFile(getContext());if(f.isFile())return f; }
        catch(java.io.IOException ignored){}
        throw new FileNotFoundException("No verified update is available");
    }
    @Override public ParcelFileDescriptor openFile(Uri uri,String mode)throws FileNotFoundException {
        if(!"r".equals(mode))throw new FileNotFoundException("Updates are read-only");
        return ParcelFileDescriptor.open(file(uri),ParcelFileDescriptor.MODE_READ_ONLY);
    }
    @Override public String getType(Uri uri){return "application/vnd.android.package-archive";}
    @Override public Cursor query(Uri uri,String[] projection,String selection,String[] args,String order){
        try{
            File f=file(uri);
            String[] columns=projection==null?new String[]{OpenableColumns.DISPLAY_NAME,OpenableColumns.SIZE}:projection;
            MatrixCursor result=new MatrixCursor(columns,1);Object[] row=new Object[columns.length];
            for(int i=0;i<columns.length;i++){
                if(OpenableColumns.DISPLAY_NAME.equals(columns[i]))row[i]="ReRAC-update.apk";
                else if(OpenableColumns.SIZE.equals(columns[i]))row[i]=f.length();
            }
            result.addRow(row);return result;
        }catch(FileNotFoundException e){return null;}
    }
    @Override public Uri insert(Uri u,ContentValues v){throw new UnsupportedOperationException("Read-only");}
    @Override public int update(Uri u,ContentValues v,String s,String[] a){throw new UnsupportedOperationException("Read-only");}
    @Override public int delete(Uri u,String s,String[] a){throw new UnsupportedOperationException("Read-only");}
}
