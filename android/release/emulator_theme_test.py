#!/usr/bin/env python3
"""Exercise the signed theme APK in a disposable AOSP emulator, never a user device."""
import argparse, hashlib, json, re, time
from pathlib import Path
from emulator_smoke import adb, sh, require, root_emulator, launch, screen, tap_text, PKG


def click(out, text, prefix=False):
    for attempt in range(9):
        root = screen(out, 'navigation')
        for node in root.iter('node'):
            value = node.get('text', '')
            if (value.startswith(text) if prefix else value == text) and node.get('enabled') == 'true' and node.get('clickable') == 'true':
                box = list(map(int, re.findall(r'\d+', node.get('bounds', ''))))
                if len(box) == 4 and box[2] > box[0] and box[3] > box[1]:
                    tap_text(root, value)
                    return
        sh('input swipe 980 580 980 250 250')
        time.sleep(.5)
    raise AssertionError('Enabled visible control missing: '+text)


def texts(root):
    return [n.get('text', '') for n in root.iter('node')]


def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--previous', type=Path, required=True)
    parser.add_argument('--candidate', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    a=parser.parse_args(); out=a.output; out.mkdir(parents=True, exist_ok=True)
    checks=[]
    try:
        root_emulator()
        sh('settings put system accelerometer_rotation 0'); sh('settings put system user_rotation 1')
        sh('wm size 1280x720'); sh('wm density 160'); adb('logcat','-c')
        require('Success' in adb('install',a.previous), 'Previous beta.2 installation failed')
        launch(); screen(out,'01-previous-beta2')
        sh('printf theme-upgrade-preserved > /data/user/0/'+PKG+'/files/theme-canary')
        click(out,'Mods',True); click(out,'AYN Thor Dual-Screen Mod',True)
        require(adb('exec-out','cat','/data/user/0/'+PKG+'/files/dual-screen.setting',binary=True)==b'\1','Original dual-screen control failed')
        sh('input keyevent 4'); time.sleep(1); click(out,'Updates',True); click(out,'Development')
        checks.append('Previous beta.2 configured with Development channel, enabled dual-screen and private-data canary')
        require('Success' in adb('install','-r',a.candidate), 'Same-signer beta.2 to beta.3 upgrade failed')
        require(sh('cat /data/user/0/'+PKG+'/files/theme-canary')=='theme-upgrade-preserved','Private data lost')
        require(adb('exec-out','cat','/data/user/0/'+PKG+'/files/dual-screen.setting',binary=True)==b'\1','Mod choice lost')
        launch(); root=screen(out,'02-themed-launcher')
        require(any('Launcher 1.1.0-beta.3' in t and 'ReRAC 0.3.4' in t for t in texts(root)),'Inaccurate or missing footer')
        require(any('GADGETRON' in t for t in texts(root)),'Theme missing from actual APK')
        checks.append('Actual signed themed launcher opens; accurate separate footer; in-place update retains existing data')
        click(out,'Rendering resolution:',True); screen(out,'03-resolution-options'); sh('input keyevent 4'); time.sleep(1)
        click(out,'Mods',True); root=screen(out,'04-themed-mods')
        require(sum('AYN Thor Dual-Screen Mod' in t for t in texts(root))==1,'Bundled mod list changed')
        click(out,'AYN Thor Dual-Screen Mod',True)
        require(adb('exec-out','cat','/data/user/0/'+PKG+'/files/dual-screen.setting',binary=True)==b'\0','Off toggle does not reach native setting')
        click(out,'AYN Thor Dual-Screen Mod',True)
        require(adb('exec-out','cat','/data/user/0/'+PKG+'/files/dual-screen.setting',binary=True)==b'\1','On toggle does not reach native setting')
        checks.append('Rendering-resolution dialog and both dual-screen toggle directions work')
        sh('input keyevent 4'); time.sleep(1); click(out,'Updates',True); root=screen(out,'05-themed-updates')
        require(any(n.get('text')=='Development' and n.get('checked')=='true' for n in root.iter('node')),'Channel reset by update')
        click(out,'Check for updates'); time.sleep(5); screen(out,'06-themed-update-check')
        checks.append('Development choice preserved; themed update screen performs network check')
        sh('input keyevent 4'); time.sleep(1); click(out,'Help & controls'); screen(out,'07-expanded-help')
        checks.append('Help panel remains usable')
        launch(); sh('wm density 240'); time.sleep(3); screen(out,'08-compact-landscape')
        click(out,'Updates',True); screen(out,'09-compact-updates')
        sh('wm density 160'); time.sleep(2); launch()
        require('VERSION_DOWNGRADE' in adb('install','-r',a.previous,check=False),'Unexpected downgrade acceptance')
        checks.append('Compact landscape navigation works; Android refuses downgrade')
        logs=adb('logcat','-d')
        require(not re.search(r'FATAL EXCEPTION[^\n]*\n(?:(?!FATAL EXCEPTION).){0,600}Process: '+re.escape(PKG),logs,re.S),'Launcher crash in logcat')
        report=dict(status='passed',checks=checks,apk_sha256=hashlib.sha256(a.candidate.read_bytes()).hexdigest(),launcher_version='1.1.0-beta.3',core_version='0.3.4',android_release=sh('getprop ro.build.version.release'),abi=sh('getprop ro.product.cpu.abi'),same_signer_upgrade=True,tested_actual_theme_apk=True,tested_native_gameplay=False,tested_physical_dual_screen=False,tested_system_installer_update=False)
        (out/'theme-smoke-report.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))
    finally:
        (out/'logcat.txt').write_text(adb('logcat','-d',check=False))
        try: screen(out,'last-window')
        except Exception: pass

if __name__=='__main__':main()
