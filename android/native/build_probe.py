#!/usr/bin/env python3
"""Build unsigned native runtime instrumentation; private signing is offline."""
from pathlib import Path
import argparse, copy, subprocess, tempfile, zipfile
ROOT=Path(__file__).resolve().parents[2]
MANIFEST='''<manifest xmlns:android="http://schemas.android.com/apk/res/android" package="org.rerac.tests.nativeprobe" android:versionCode="1" android:versionName="1.0"><uses-sdk android:minSdkVersion="26" android:targetSdkVersion="36"/><application android:label="ReRAC native runtime test" android:debuggable="false"/><instrumentation android:name="org.rerac.tests.NativeRuntimeInstrumentation" android:targetPackage="org.rerac.android.uipreview" android:functionalTest="true"/></manifest>'''
def build(tools:Path,output:Path):
    def run(args):subprocess.run([str(a) for a in args],check=True)
    output.parent.mkdir(parents=True,exist_ok=True)
    if output.exists():raise ValueError('Refusing to overwrite a probe APK')
    bt=tools/'build-tools'
    with tempfile.TemporaryDirectory(prefix='native-probe-') as td:
        w=Path(td);(w/'classes').mkdir();(w/'dex').mkdir();(w/'AndroidManifest.xml').write_text(MANIFEST)
        run(['javac','-encoding','UTF-8','--release','8','-classpath',tools/'android.jar','-d',w/'classes',ROOT/'android/native/tests/NativeRuntimeInstrumentation.java'])
        run(['jar','cf',w/'probe.jar','-C',w/'classes','.'])
        run([bt/'d8','--release','--min-api','26','--lib',tools/'android.jar','--output',w/'dex',w/'probe.jar'])
        run([bt/'aapt2','link','-o',w/'resources.apk','-I',tools/'android.jar','--manifest',w/'AndroidManifest.xml'])
        with zipfile.ZipFile(w/'resources.apk') as old,zipfile.ZipFile(w/'unaligned.apk','w') as new:
            for entry in old.infolist():
                info=copy.copy(entry);info.date_time=(2026,10,19,0,0,0);info.extra=b'';new.writestr(info,old.read(entry))
            entry=zipfile.ZipInfo('classes.dex',(2026,10,19,0,0,0));entry.compress_type=zipfile.ZIP_DEFLATED;new.writestr(entry,(w/'dex/classes.dex').read_bytes())
        run([bt/'zipalign','-P','16','-f','4',w/'unaligned.apk',output])
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--tools',type=Path,required=True);p.add_argument('--output',type=Path,required=True);a=p.parse_args();build(a.tools,a.output)
