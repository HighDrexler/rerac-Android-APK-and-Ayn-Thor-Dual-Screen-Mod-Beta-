#!/usr/bin/env python3
"""Build the editable launcher layer over the immutable, checksum-pinned v16 APK.

This is NOT a native ReRAC 0.3.5 build. All four native libraries and original
classes.dex..classes5.dex stay identical. Signing is a separate private step.
"""
from __future__ import annotations
import argparse
import copy
import hashlib
import json
from pathlib import Path
import shutil
import struct
import subprocess
import tempfile
import zipfile
from apk_format import axml_chunks, string_pool, make_string_pool, manifest_values, native_alignment, file_sha256

BASE_SHA = 'cb13360fc1796e2c1e9ef00029eacbb853bd8e966837140878bd65df55f3b477'
PACKAGE = 'org.rerac.android.uipreview'
CORE = '0.3.4'
SIGNER = '25c5eebb05a013bf3fb84706f9ad87904b8d4d8a99ba7857d62e665d2b94e24a'
ANDROID = 'http://schemas.android.com/apk/res/android'
UINT_MAX = 0xffffffff
RES_IDS = {'name':0x01010003,'exported':0x01010010,'launchMode':0x0101001d,
           'screenOrientation':0x0101001e,'configChanges':0x0101001f,
           'authorities':0x01010018,'grantUriPermissions':0x0101001b}


def patch_manifest(data:bytes, version:str, code:int)->bytes:
    if code<=16 or code>2100000000 or not version.startswith('1.1.'):
        raise ValueError('Use a Launcher 1.1 version and versionCode above the released v16 APK')
    chunks=axml_chunks(data)
    pi=next(i for i,c in enumerate(chunks) if struct.unpack_from('<H',c)[0]==1)
    ri=next(i for i,c in enumerate(chunks) if struct.unpack_from('<H',c)[0]==0x180)
    strings,flags=string_pool(chunks[pi]); original_strings=list(strings)
    ids=list(struct.unpack_from('<'+'I'*((len(chunks[ri])-8)//4),chunks[ri],8))
    def index(s):
        if s not in strings:strings.append(s)
        return strings.index(s)
    ns=index(ANDROID)
    def attr(key,value):
        key_index=index(key)
        while len(ids)<=key_index:ids.append(0)
        if ids[key_index] not in (0,RES_IDS[key]):raise ValueError('Unexpected resource ID')
        ids[key_index]=RES_IDS[key]
        if isinstance(value,bool):raw,typ,num=UINT_MAX,0x12,int(value)
        elif isinstance(value,int):raw,typ,num=UINT_MAX,0x10,value
        else:raw= index(value);typ,num=3,raw
        return struct.pack('<IIIHBBI',ns,key_index,raw,8,0,typ,num)
    def node(name,attrs):
        name_index=index(name)
        entries=b''.join(attr(k,v) for k,v in sorted(attrs.items(),key=lambda kv:RES_IDS[kv[0]]))
        start=struct.pack('<HHIII',0x102,16,36+len(entries),1,UINT_MAX)
        start+=struct.pack('<IIHHHHHH',UINT_MAX,name_index,20,20,len(attrs),0,0,0)+entries
        end=struct.pack('<HHIIIII',0x103,16,24,1,UINT_MAX,UINT_MAX,name_index)
        return [bytearray(start),bytearray(end)]
    additions=[]
    for permission in ['android.permission.INTERNET','android.permission.REQUEST_INSTALL_PACKAGES']:
        additions+=node('uses-permission',{'name':permission})
    activities=node('activity',{'name':'org.rerac.android.LauncherToolsActivity','exported':False,
                    'launchMode':2,'screenOrientation':6,'configChanges':0x4a4})
    provider=node('provider',{'name':'org.rerac.android.UpdateApkProvider','authorities':PACKAGE+'.updates',
                             'exported':False,'grantUriPermissions':True})
    found=set();out=[]
    for chunk in chunks:
        kind=struct.unpack_from('<H',chunk)[0]
        if kind==0x102:
            element=strings[struct.unpack_from('<I',chunk,20)[0]]
            start,size,count=struct.unpack_from('<HHH',chunk,24)
            for i in range(count):
                offset=16+start+i*size
                key=strings[struct.unpack_from('<I',chunk,offset+4)[0]]
                typ=chunk[offset+15];value=struct.unpack_from('<I',chunk,offset+16)[0]
                if element=='manifest' and key=='versionName':
                    if typ!=3 or strings[value]!='0.3.4-android-mobile16':raise ValueError('Wrong baseline version')
                    strings[value]=version;found.add(key)
                elif element=='manifest' and key=='versionCode':
                    if value!=16:raise ValueError('Wrong baseline version code')
                    struct.pack_into('<I',chunk,offset+16,code);found.add(key)
                elif element=='application' and key=='debuggable':
                    if typ!=0x12:raise ValueError('Unexpected debug flag')
                    struct.pack_into('<I',chunk,offset+16,0);found.add(key)
            if element=='application':out.extend(additions)
        if kind==0x103 and strings[struct.unpack_from('<I',chunk,20)[0]]=='application':out.extend(activities+provider)
        out.append(chunk)
    if found!={'versionName','versionCode','debuggable'}:raise ValueError('Missing patch points')
    # Replace the original chunk objects after building the output sequence.
    out[out.index(chunks[pi])]=bytearray(make_string_pool(strings,flags))
    out[out.index(chunks[ri])]=bytearray(struct.pack('<HHI',0x180,8,8+4*len(ids))+struct.pack('<'+'I'*len(ids),*ids))
    result=struct.pack('<HHI',3,8,8+sum(map(len,out)))+b''.join(out)
    before=manifest_values(data);after=manifest_values(result)
    new_types={'uses-permission','provider'}
    restored=[]
    for item in after:
        if item['element'] in new_types:continue
        if item['element']=='activity' and item['attrs'].get('name')=='org.rerac.android.LauncherToolsActivity':continue
        item=copy.deepcopy(item)
        if item['element']=='manifest':item['attrs'].update(versionCode=16,versionName='0.3.4-android-mobile16')
        if item['element']=='application':item['attrs']['debuggable']=True
        restored.append(item)
    if restored!=before:raise ValueError('An unrelated manifest component changed')
    return result


def run(args):
    subprocess.run([str(a) for a in args],check=True)


def build(baseline:Path, tools:Path, output:Path, version:str, code:int)->dict:
    root=Path(__file__).resolve().parents[2]
    if file_sha256(baseline)!=BASE_SHA:raise ValueError('Baseline APK is not the published v16 release')
    if not (tools/'android.jar').is_file():raise ValueError('tools/android.jar is missing')
    bt=tools/'build-tools'
    output.parent.mkdir(parents=True,exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='rerac-launcher-') as work:
        work=Path(work);classes=work/'classes';dex=work/'dex';classes.mkdir();dex.mkdir()
        sources=sorted((root/'android/src').rglob('*.java'))
        run(['javac','-encoding','UTF-8','--release','8','-classpath',tools/'android.jar','-d',classes,*sources])
        jar=work/'runtime.jar';run(['jar','cf',jar,'-C',classes,'.'])
        run([bt/'d8','--release','--min-api','26','--lib',tools/'android.jar','--output',dex,jar])
        if sorted(p.name for p in dex.iterdir())!=['classes.dex']:raise ValueError('Unexpected multidex output')
        raw=work/'unsigned-unaligned.apk'
        metadata={'schema':1,'package_id':PACKAGE,'version_code':code,'launcher_version':version,
                  'core_version':CORE,'core_provenance':'v16 native engine retained; upstream 0.3.5 is not in this APK',
                  'baseline_apk_sha256':BASE_SHA,'native_source_rebuild':False,'mod_api':1}
        with zipfile.ZipFile(baseline) as old,zipfile.ZipFile(raw,'w') as new:
            native={name:hashlib.sha256(old.read(name)).hexdigest() for name in old.namelist() if name.startswith('lib/') and name.endswith('.so')}
            metadata['native_sha256']=native
            for entry in old.infolist():
                if entry.filename.startswith('META-INF/') and entry.filename.upper().endswith(('.SF','.RSA','.DSA','.EC','MANIFEST.MF')):continue
                contents=old.read(entry)
                if entry.filename=='AndroidManifest.xml':contents=patch_manifest(contents,version,code)
                elif entry.filename=='classes6.dex':contents=(dex/'classes.dex').read_bytes()
                cloned=copy.copy(entry);cloned.extra=b'';cloned.date_time=(2026,10,19,0,0,0)
                new.writestr(cloned,contents)
            assets={'assets/rerac-build.json':(json.dumps(metadata,sort_keys=True,separators=(',',':'))+'\n').encode()}
            for f in sorted((root/'android/assets').rglob('*')):
                if f.is_file():assets['assets/'+f.relative_to(root/'android/assets').as_posix()]=f.read_bytes()
            for name,contents in assets.items():
                zi=zipfile.ZipInfo(name,(2026,10,19,0,0,0));zi.compress_type=zipfile.ZIP_DEFLATED;new.writestr(zi,contents)
        run([bt/'zipalign','-P','16','-f','4',raw,output])
    preserved=[]
    with zipfile.ZipFile(baseline) as old,zipfile.ZipFile(output) as new:
        for name in old.namelist():
            if name not in ('AndroidManifest.xml','classes6.dex') and not name.startswith('META-INF/'):
                if old.read(name)!=new.read(name):raise ValueError('Original payload changed: '+name)
                preserved.append(name)
    report={'launcher_version':version,'version_code':code,'core_version':CORE,'unsigned_sha256':file_sha256(output),
            'bytes':output.stat().st_size,'preserved_entries':len(preserved),'modified_original_entries':['AndroidManifest.xml','classes6.dex'],
            'native_libraries':native_alignment(output),'native_0_3_5_included':False,'device_gameplay_tested':False}
    output.with_suffix('.build-report.json').write_text(json.dumps(report,indent=2)+'\n')
    return report

if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--baseline',type=Path,required=True);p.add_argument('--tools',type=Path,required=True)
    p.add_argument('--output',type=Path,required=True);p.add_argument('--version',default='1.1.0-beta.1');p.add_argument('--code',type=int,default=17)
    a=p.parse_args();print(json.dumps(build(a.baseline,a.tools,a.output,a.version,a.code),indent=2))
