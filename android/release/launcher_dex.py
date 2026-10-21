"""Compile the production launcher from editable Java source."""
import re,subprocess,tempfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
OLD_PACKAGE='org.rerac.android.uipreview'
DEV_PACKAGE='org.rerac.android.ui035test'
OLD_CERT='25c5eebb05a013bf3fb84706f9ad87904b8d4d8a99ba7857d62e665d2b94e24a'
DEV_BRANCH='update-channel3'
EXPECTED_SIGNER='57188113b5796632a7d4db37622abf7c4d2fb3d9b0bccdc9830b01560ee0f81e'
def source_built_dex(signer,zipalign):
    """Compile Java sources with proper DEX string_ids ordering, never byte-patch DEX.

    Prior sidecar corrupted the DEX sorted string pool, preventing the provider
    from loading and causing a crash before the launcher could open.
    """
    assert re.fullmatch(r'[0-9a-f]{64}',signer)
    source=ROOT/'android/launcher/android/src'
    android_jar=zipalign.parents[2]/'platforms/android-36/android.jar'
    d8=zipalign.parent/'d8'
    if not d8.is_file():d8=zipalign.parent/'d8.bat'
    assert android_jar.is_file() and d8.is_file(), 'Android SDK 36 build tools required'
    with tempfile.TemporaryDirectory(prefix='rerac-recompile-') as td:
        base=Path(td);sources=base/'sources';classes=base/'classes';dexdir=base/'dex'
        classes.mkdir();dexdir.mkdir();paths=[]
        for original in sorted(source.rglob('*.java')):
            relative=original.relative_to(source)
            target=sources/relative;target.parent.mkdir(parents=True,exist_ok=True)
            text=original.read_text()
            def single(a,b):
                nonlocal text
                assert text.count(a)==1, f'Unexpected constant in {relative}: {a}'
                text=text.replace(a,b)
            if original.name=='LauncherBuild.java':
                single(OLD_PACKAGE,DEV_PACKAGE)
                single(OLD_CERT,signer)
            if original.name=='UpdatePolicy.java':
                assert '"/'+DEV_BRANCH+'/"' in text, 'Wrong branch feed constant'
            target.write_text(text);paths.append(str(target))
        assert len(paths)>=10, 'Missing Android launcher sources'
        subprocess.run(['javac','-encoding','UTF-8','--release','8','-classpath',str(android_jar),'-d',str(classes),*paths],check=True)
        jar=base/'launcher.jar'
        subprocess.run(['jar','cf',str(jar),'-C',str(classes),'.'],check=True)
        subprocess.run([str(d8),'--release','--min-api','26','--lib',str(android_jar),'--output',str(dexdir),str(jar)],check=True)
        found=sorted(dexdir.glob('*.dex'))
        assert [x.name for x in found]==['classes.dex'],'Unexpected compiled multidex layout'
        data=found[0].read_bytes()
        assert b'UpdateApkProvider' in data and signer.encode() in data and DEV_PACKAGE.encode() in data
        assert DEV_BRANCH.encode() in data and OLD_CERT.encode() not in data
        return data
