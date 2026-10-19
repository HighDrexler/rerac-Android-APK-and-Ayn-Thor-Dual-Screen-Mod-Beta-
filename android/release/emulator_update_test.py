#!/usr/bin/env python3
"""Exercise the shipped updater over public HTTPS in a disposable emulator.
The emulator alone is pre-granted permission to request installs. The user-facing
Android package installation confirmation is still clicked normally.
"""
import argparse,json,re,time,xml.etree.ElementTree as ET
from pathlib import Path
from emulator_smoke import adb,sh,require,root_emulator,launch,screen,tap_text,PKG

def dump():
    for attempt in range(6):
        text=adb('shell','uiautomator dump /sdcard/update-test.xml',check=False)
        if 'dumped' in text.lower():
            return ET.fromstring(sh('cat /sdcard/update-test.xml'))
        time.sleep(1)
    raise RuntimeError('Cannot inspect updater UI')

def visible(root,text,prefix=False,enabled=True):
    nodes=[]
    for node in root.iter('node'):
        t=node.get('text','')
        if not (t.startswith(text) if prefix else t==text): continue
        if enabled and node.get('enabled')!='true': continue
        coords=list(map(int,re.findall(r'\d+',node.get('bounds',''))))
        if len(coords)==4 and coords[2]>coords[0] and coords[3]>coords[1]:nodes.append(node)
    return nodes

def click(text,prefix=False):
    for attempt in range(10):
        root=dump()
        if visible(root,text,prefix):
            tap_text(root,text,prefix);return
        sh('input swipe 1060 570 1060 230 300');time.sleep(1)
    raise AssertionError('Control missing, disabled or not visible: '+text)

def await_text(fragment,timeout=120):
    end=time.monotonic()+timeout
    while time.monotonic()<end:
        root=dump()
        if any(fragment in n.get('text','') for n in root.iter('node')): return root
        time.sleep(2)
    raise AssertionError('Updater never displayed: '+fragment)

def code():
    result=sh('dumpsys package '+PKG)
    match=re.search(r'versionCode=(\d+)',result)
    return int(match.group(1)) if match else -1

def main():
    p=argparse.ArgumentParser();p.add_argument('--apk',type=Path,required=True);p.add_argument('--output',type=Path,required=True);args=p.parse_args()
    out=args.output;out.mkdir(parents=True,exist_ok=True)
    try:
        root_emulator();sh('settings put system accelerometer_rotation 0');sh('settings put system user_rotation 1');sh('wm size 1280x720');sh('wm density 160')
        require('Success' in adb('install',args.apk),'Initial beta.1 installation failed');require(code()==17,'Wrong initial APK')
        launch();sh("printf network-update-kept-data > /data/user/0/"+PKG+"/files/network-update-canary");sh("printf '\\001' > /data/user/0/"+PKG+"/files/dual-screen.setting")
        # Equivalent to the user allowing this source in Android Settings, only
        # in this throwaway emulator. It does not silently approve installation.
        sh('appops set '+PKG+' REQUEST_INSTALL_PACKAGES allow')
        click('Updates',True);click('Development');click('Check for updates')
        await_text('A signed update is available.');screen(out,'01-real-feed-update-available')
        click('Download update');time.sleep(2);screen(out,'02-real-apk-downloading')
        end=time.monotonic()+360
        while time.monotonic()<end:
            root=dump()
            if visible(root,'Install downloaded update'):break
            sh('input swipe 1060 570 1060 230 300');time.sleep(2)
        else:raise AssertionError('APK download/verification did not enable installation')
        screen(out,'03-real-apk-verified');click('Install downloaded update')
        end=time.monotonic()+60
        while time.monotonic()<end:
            root=dump();installer_nodes=[n for n in root.iter('node') if 'packageinstaller' in n.get('package','')]
            if installer_nodes:
                screen(out,'04-android-install-confirmation')
                buttons=[n for n in installer_nodes if n.get('text','').lower() in ('update','install') and n.get('enabled')=='true']
                if buttons:
                    tap_text(root,buttons[0].get('text'));break
            time.sleep(1)
        else:raise AssertionError('Android system installer did not offer confirmation')
        end=time.monotonic()+90
        while time.monotonic()<end and code()!=18:time.sleep(2)
        require(code()==18,'System installer did not update to beta.2');launch();screen(out,'05-updated-launcher-beta2')
        require(sh('cat /data/user/0/'+PKG+'/files/network-update-canary')=='network-update-kept-data','Network update lost app data')
        require(adb('exec-out','cat','/data/user/0/'+PKG+'/files/dual-screen.setting',binary=True)==b'\1','Network update reset dual-screen setting')
        click('Updates',True);root=dump();require(any(n.get('text')=='Development' and n.get('checked')=='true' for n in root.iter('node')),'Network update reset channel')
        click('Check for updates');await_text('You already have this version');screen(out,'06-no-repeat-update')
        report=dict(status='passed',android_release=sh('getprop ro.build.version.release'),abi=sh('getprop ro.product.cpu.abi'),initial_version_code=17,final_version_code=18,live_https_feed_verified=True,live_https_apk_download_verified=True,actual_android_installer_confirmed=True,existing_data_preserved=True,mod_and_channel_preserved=True,permission_setup='REQUEST_INSTALL_PACKAGES pre-granted only in disposable emulator',tested_native_gameplay=False,tested_physical_dual_screen=False)
        (out/'network-update-report.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))
    finally:
        (out/'logcat.txt').write_text(adb('logcat','-d',check=False))
        try:screen(out,'last-window')
        except Exception:pass
if __name__=='__main__':main()
