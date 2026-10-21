#!/usr/bin/env python3
"""Build a separate instrumentation APK against the disposable Development app."""
import argparse,copy,subprocess,tempfile,zipfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
def build(sdk,output):
    bt=sdk/'build-tools/36.0.0';android=sdk/'platforms/android-36/android.jar'
    def run(*args):subprocess.run([str(a) for a in args],check=True)
    with tempfile.TemporaryDirectory() as td:
        w=Path(td);(w/'classes').mkdir();(w/'dex').mkdir()
        (w/'AndroidManifest.xml').write_text('<manifest xmlns:android="http://schemas.android.com/apk/res/android" package="org.rerac.tests.companion" android:versionCode="1"><uses-sdk android:minSdkVersion="26" android:targetSdkVersion="36"/><application android:label="Companion UI regression"/><instrumentation android:name="org.rerac.tests.CompanionUiInstrumentation" android:targetPackage="org.rerac.android.ui035test" android:functionalTest="true"/></manifest>')
        run('javac','--release','8','-classpath',android,'-d',w/'classes',ROOT/'android/release/CompanionUiInstrumentation.java')
        run('jar','cf',w/'test.jar','-C',w/'classes','.')
        run(bt/'d8','--release','--min-api','26','--lib',android,'--output',w/'dex',w/'test.jar')
        run(bt/'aapt2','link','-o',w/'resources.apk','-I',android,'--manifest',w/'AndroidManifest.xml')
        with zipfile.ZipFile(w/'resources.apk') as old,zipfile.ZipFile(w/'test.apk','w') as new:
            for e in old.infolist():new.writestr(copy.copy(e),old.read(e))
            new.writestr('classes.dex',(w/'dex/classes.dex').read_bytes())
        run(bt/'zipalign','-P','16','-f','4',w/'test.apk',output)
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--sdk',type=Path,required=True);p.add_argument('--output',type=Path,required=True);a=p.parse_args();build(a.sdk,a.output)
