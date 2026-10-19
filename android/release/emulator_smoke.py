#!/usr/bin/env python3
"""Launcher-only smoke checks in a disposable rooted AOSP emulator. No game data."""
import argparse, json, re, subprocess, time, xml.etree.ElementTree as ET
from pathlib import Path
PKG='org.rerac.android.uipreview'
def adb(*args, check=True, binary=False):
    p=subprocess.run(['adb',*map(str,args)],capture_output=True,text=not binary,timeout=180)
    if check and p.returncode: raise RuntimeError(str(args)+': '+str(p.stdout)+str(p.stderr))
    return p.stdout if binary else (p.stdout+p.stderr).strip()
def sh(cmd): return adb('shell',cmd)
def require(ok, msg):
    if not ok: raise AssertionError(msg)
def root_emulator():
    # adbd restarts when switching to root. A closed transport is expected here,
    # but the eventual uid must be verified; never skip private-data assertions.
    for attempt in range(5):
        adb('root',check=False)
        adb('wait-for-device')
        if adb('shell','id','-u',check=False).strip()=='0':
            return
        time.sleep(2)
    raise RuntimeError('Disposable AOSP emulator did not reconnect as root')
def screen(out, name):
    (out/(name+'.png')).write_bytes(adb('exec-out','screencap','-p',binary=True))
    for _ in range(6):
        result=adb('shell','uiautomator dump /sdcard/launcher-test.xml',check=False)
        if 'dumped' in result.lower():
            xml=sh('cat /sdcard/launcher-test.xml'); (out/(name+'.xml')).write_text(xml);return ET.fromstring(xml)
        time.sleep(1)
    raise RuntimeError('Cannot inspect current Android window')
def tap_text(root, text, prefix=False):
    candidates=[n for n in root.iter('node') if (n.get('text','').startswith(text) if prefix else n.get('text')==text)]
    require(bool(candidates),'Missing UI control: '+text)
    clickable=[n for n in candidates if n.get('clickable')=='true'];n=(clickable or candidates)[0]
    coords=list(map(int,re.findall(r'\d+',n.attrib['bounds']))); require(len(coords)==4,'Bad control bounds')
    sh('input tap %d %d'%((coords[0]+coords[2])//2,(coords[1]+coords[3])//2));time.sleep(1)
def launch():
    adb('shell','am','force-stop',PKG); adb('shell','am','start','-W','-n',PKG+'/org.rerac.android.LauncherActivity');time.sleep(2)
def check_canary():
    require(sh('cat /data/user/0/'+PKG+'/files/launcher-upgrade-canary')=='keep-existing-data','Upgrade discarded app data')
def main():
    p=argparse.ArgumentParser();p.add_argument('--baseline',type=Path,required=True);p.add_argument('--signed',type=Path,required=True);p.add_argument('--output',type=Path,required=True);a=p.parse_args()
    a.output.mkdir(parents=True,exist_ok=True);checks=[]
    root_emulator();sh('settings put system accelerometer_rotation 0');sh('settings put system user_rotation 1');sh('wm size 1280x720');sh('wm density 160')
    adb('logcat','-c');require('Success' in adb('install',a.baseline),'Baseline install failed');launch();screen(a.output,'01-baseline')
    sh("printf keep-existing-data > /data/user/0/"+PKG+"/files/launcher-upgrade-canary")
    sh("printf '\\000' > /data/user/0/"+PKG+"/files/dual-screen.setting")
    apk1=a.signed/'ReRAC-Launcher-1.1.0-beta.1.apk';apk2=a.signed/'ReRAC-Launcher-1.1.0-beta.2.apk'
    require('Success' in adb('install','-r',apk1),'Same-signer v16 to beta.1 update failed');check_canary();checks.append('v16 to beta.1 in-place installation; existing private data retained')
    launch();root=screen(a.output,'02-launcher-beta1');texts=[n.get('text','') for n in root.iter('node')]
    require(any('Launcher 1.1.0-beta.1' in t and 'ReRAC 0.3.4' in t for t in texts),'Wrong or missing version footer');checks.append('Actual launcher opens with accurate separate versions')
    tap_text(root,'Mods',True);root=screen(a.output,'03-mods-off');tap_text(root,'AYN Thor Dual-Screen Mod',True)
    require(adb('exec-out','cat','/data/user/0/'+PKG+'/files/dual-screen.setting',binary=True)==b'\1','Mod switch did not update original native setting');screen(a.output,'04-mods-on');checks.append('Dual-screen switch writes original setting; sole bundled mod visible')
    sh('input keyevent 4');time.sleep(1);root=screen(a.output,'05-launcher-mod-on');tap_text(root,'Updates',True)
    root=screen(a.output,'06-updates-stable');require(any(n.get('text')=='Stable' and n.get('checked')=='true' for n in root.iter('node')),'Stable was not default')
    tap_text(root,'Development');screen(a.output,'07-updates-development');checks.append('Updates page and opt-in Development channel usable')
    require('Success' in adb('install','-r',apk2),'beta.1 to beta.2 install failed');check_canary();launch();root=screen(a.output,'08-launcher-beta2')
    require(any('Launcher 1.1.0-beta.2' in n.get('text','') for n in root.iter('node')),'Beta.2 version not shown')
    require(adb('exec-out','cat','/data/user/0/'+PKG+'/files/dual-screen.setting',binary=True)==b'\1','Upgrade reset mod toggle')
    tap_text(root,'Updates',True);root=screen(a.output,'09-updates-preserved');require(any(n.get('text')=='Development' and n.get('checked')=='true' for n in root.iter('node')),'Upgrade reset update channel');checks.append('Second in-place upgrade preserves mod and channel choices')
    result=adb('install','-r',a.baseline,check=False);require('VERSION_DOWNGRADE' in result,'Unexpected downgrade acceptance');checks.append('Android rejects downgrade to v16')
    logs=adb('logcat','-d');(a.output/'logcat.txt').write_text(logs)
    require(not re.search(r'FATAL EXCEPTION[^\n]*\n(?:(?!FATAL EXCEPTION).){0,600}Process: '+re.escape(PKG),logs,re.S),'Launcher crashed during smoke test')
    report=dict(status='passed',checks=checks,android_release=sh('getprop ro.build.version.release'),abi=sh('getprop ro.product.cpu.abi'),device=sh('getprop ro.product.model'),tested_native_gameplay=False,tested_physical_dual_screen=False,tested_in_launcher_network_install=False)
    (a.output/'smoke-report.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))
if __name__=='__main__': main()
