#!/usr/bin/env python3
"""Real native-core checks in a disposable rooted AOSP emulator; no game data."""
import argparse, hashlib, json, re, subprocess, time, zipfile
from pathlib import Path
from emulator_smoke import adb, sh, require, root_emulator, launch, screen, PKG
from emulator_theme_test import click, texts

def main():
    p=argparse.ArgumentParser(description=__doc__)
    for name in ('previous','candidate','probe','output'):p.add_argument('--'+name,type=Path,required=True)
    a=p.parse_args();out=a.output;out.mkdir(parents=True,exist_ok=True)
    with zipfile.ZipFile(a.candidate) as z:build=json.loads(z.read('assets/rerac-build.json'))
    require(build.get('native_source_rebuild') is True,'Candidate still retains the v16 engine')
    checks=[];private='/data/user/0/'+PKG+'/files/'
    try:
        root_emulator();sh('settings put system accelerometer_rotation 0');sh('settings put system user_rotation 1')
        sh('wm size 1280x720');sh('wm density 160');adb('logcat','-c')
        require('Success' in adb('install',a.previous),'Previous released APK did not install')
        launch();screen(out,'01-previous-launcher')
        sh('mkdir -p '+private+'memcard '+private+'gamedata')
        for path,value in [('native-upgrade-canary','private-data-retained'),('memcard/native-test-canary','save-folder-retained'),('gamedata/native-test-canary','import-folder-retained')]:
            sh('printf '+value+' > '+private+path)
        click(out,'Mods',True)
        # Explicitly retain the user's original v16 setting representation.
        if adb('exec-out','cat',private+'dual-screen.setting',binary=True)!=b'\1':click(out,'AYN Thor Dual-Screen Mod',True)
        sh('input keyevent 4');time.sleep(1);click(out,'Updates',True);click(out,'Development')
        require('Success' in adb('install','-r',a.candidate),'Same-certificate in-place native update failed')
        for path,value in [('native-upgrade-canary','private-data-retained'),('memcard/native-test-canary','save-folder-retained'),('gamedata/native-test-canary','import-folder-retained')]:
            require(sh('cat '+private+path)==value,'Update lost '+path)
        require(adb('exec-out','cat',private+'dual-screen.setting',binary=True)==b'\1','Update reset the dual-screen choice')
        checks.append('Same-signer upgrade preserves private test data in actual save/import paths and the existing mod choice')
        launch();r=screen(out,'02-native-core-launcher')
        require(any('Launcher '+build['launcher_version'] in t and 'ReRAC '+build['core_version'] in t for t in texts(r)),'Launcher footer does not describe the actual native build')
        require(any('GADGETRON' in t for t in texts(r)),'Existing launcher presentation was lost')
        click(out,'Mods',True);r=screen(out,'03-native-core-mods')
        require(sum('AYN Thor Dual-Screen Mod' in t for t in texts(r))==1,'Bundled mod count changed')
        click(out,'AYN Thor Dual-Screen Mod',True);require(adb('exec-out','cat',private+'dual-screen.setting',binary=True)==b'\0','Mod off did not reach original setting')
        click(out,'AYN Thor Dual-Screen Mod',True);require(adb('exec-out','cat',private+'dual-screen.setting',binary=True)==b'\1','Mod on did not reach original setting')
        sh('input keyevent 4');time.sleep(1);click(out,'Updates',True);r=screen(out,'04-native-core-updates')
        require(any(n.get('text')=='Development' and n.get('checked')=='true' for n in r.iter('node')),'Development preference did not persist')
        checks.append('Themed launcher, dual-version footer, bundled mod control and update-channel preference work after the core update')
        require('Success' in adb('install',a.probe),'Same-signer instrumentation did not install')
        adb('shell','am','force-stop',PKG)
        # Default am instrument output prints only the stream when one is supplied.
        # Raw mode retains INSTRUMENTATION_CODE; never infer success from prose.
        cmd=['adb','shell','am','instrument','-w','-r','-e','core',build['core_version'],'-e','source',build['native_source_sha'],'org.rerac.tests.nativeprobe/org.rerac.tests.NativeRuntimeInstrumentation']
        run=subprocess.run(cmd,capture_output=True,text=True,timeout=300)
        (out/'native-instrumentation-output.txt').write_text(run.stdout+run.stderr)
        result_codes=re.findall(r'^INSTRUMENTATION_CODE:\s*(-?\d+)\s*$',run.stdout,re.MULTILINE)
        require(run.returncode==0 and result_codes==['-1'],'Native instrumentation failed: '+(run.stdout+run.stderr)[-4000:])
        native=json.loads(sh('cat '+private+'native-instrumentation.json'))
        require(native['status']=='passed' and native['original_jni_methods']==19 and native['pause_resume'] is True,'Native execution checks did not pass')
        require(native['core']['core_version']==build['core_version'] and native['core']['source_sha']==build['native_source_sha'],'Executed native result differs from packaged source')
        (out/'native-instrumentation.json').write_text(json.dumps(native,indent=2)+'\n')
        (out/'native-render.png').write_bytes(adb('exec-out','cat',private+'native-probe.png',binary=True))
        (out/'native-probe-report.json').write_text(sh('cat '+private+'native-probe.report.json'))
        checks.append('Actual native binary reports the correct source/core, resolves and executes all 19 original JNI signatures')
        checks.append('Actual GameActivity initializes the GPU, loads all packaged shader files, captures native graphics and survives background/resume')
        path=sh('pm path '+PKG).splitlines()[0].removeprefix('package:')
        installed=sh('sha256sum '+path).split()[0]
        expected=hashlib.sha256(a.candidate.read_bytes()).hexdigest()
        require(installed==expected,'Installed APK differs from the supplied signed bytes')
        require('VERSION_DOWNGRADE' in adb('install','-r',a.previous,check=False),'Android accepted a native update downgrade')
        checks.append('Installed APK hash matches the signed candidate; Android rejects downgrade')
        report=dict(schema=1,status='passed',checks=checks,apk_sha256=expected,launcher_version=build['launcher_version'],version_code=build['version_code'],core_version=build['core_version'],native_source_sha=build['native_source_sha'],upstream_sha=build['upstream_sha'],android_release=sh('getprop ro.build.version.release'),abi=sh('getprop ro.product.cpu.abi'),native_runtime_tested=True,tested_native_gameplay=False,tested_real_saved_game=False,tested_physical_dual_screen=False,tested_system_installer_update=False)
        (out/'native-smoke-report.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))
    finally:
        (out/'logcat.txt').write_text(adb('logcat','-d',check=False))
        for name in ('native-instrumentation.json','native-probe.report.json'):
            try:
                value=sh('cat '+private+name)
                json.loads(value);(out/name).write_text(value)
            except Exception:pass
        try:screen(out,'last-window')
        except Exception:pass
if __name__=='__main__':main()
