"""Replace companion wheel entry points with source, preserving the surrounding artwork."""
import hashlib,subprocess,tempfile,urllib.request
from pathlib import Path

ROOT=Path(__file__).resolve().parents[2]
JARS={
    'org/smali/dexlib2/2.5.2/dexlib2-2.5.2.jar':'5a5c8982d8bd7d6e3bb1a0713049e3c78b719ec32b20f6b619885cec30a0dd61',
    'com/google/guava/guava/27.1-android/guava-27.1-android.jar':'686404f2d1d4d221911f96bd627ff60dac2226a5dfa6fb8ba517073eb97ec0ef',
    'com/google/code/findbugs/jsr305/3.0.2/jsr305-3.0.2.jar':'766ad2a0783f2687962c8ad74ceecc38a28b9f72a2d085ee438b7813e928d0c7',
}
BASE='d13bbd9c4365ea0a229d5fa47c8356fbe575af61bee155c7d9620b4aa9e4c110'
def hook_companion(data):
    if hashlib.sha256(data).hexdigest()!=BASE:raise ValueError('Unrecognized retained companion DEX')
    with tempfile.TemporaryDirectory(prefix='rerac-companion-') as td:
        work=Path(td)
        for artifact,digest in JARS.items():
            with urllib.request.urlopen('https://repo.maven.apache.org/maven2/'+artifact,timeout=60) as response:body=response.read(8*1024*1024)
            if hashlib.sha256(body).hexdigest()!=digest:raise ValueError('Companion tool checksum mismatch')
            (work/Path(artifact).name).write_bytes(body)
        subprocess.run(['javac','--release','8','-cp',str(work/'*'),'-d',str(work),str(ROOT/'android/release/CompanionDexHooks.java')],check=True)
        (work/'input.dex').write_bytes(data)
        subprocess.run(['java','-cp',str(work)+':'+str(work/'*'),'CompanionDexHooks',str(work/'input.dex'),str(work/'output.dex')],check=True)
        result=(work/'output.dex').read_bytes()
        if b'Lorg/rerac/android/CompanionQuickSelect;' not in result:raise ValueError('Missing source-built companion controller')
        if b'retainedMenuTouch' not in result:raise ValueError('Missing complex pause-menu routing')
        return result
