#!/usr/bin/env bash
set -euo pipefail
PKG=org.rerac.android.ui035test
mkdir -p work/reports
capture(){
  adb logcat -d > work/reports/logcat.txt || true
  adb shell pidof "$PKG" > work/reports/pid.txt || true
  adb exec-out screencap -p > work/reports/launcher.png || true
  adb shell uiautomator dump /sdcard/sidecar-test.xml || true
  adb exec-out cat /sdcard/sidecar-test.xml > work/reports/ui.xml || true
}
trap capture EXIT
adb shell wm size 1280x720
adb shell wm density 160
adb logcat -c
adb install work/test/sidecar.apk
adb shell am start -W -n "$PKG/org.rerac.android.LauncherActivity" | tee work/reports/launch.txt
sleep 8
capture
python3 - <<'PY'
from pathlib import Path
import os,json
import xml.etree.ElementTree as ET
s=Path('work/reports/ui.xml').read_text()
root=ET.fromstring(s)
texts=[n.attrib.get('text','') for n in root.iter('node')]
pid=Path('work/reports/pid.txt').read_text().strip()
log=Path('work/reports/logcat.txt').read_text(errors='replace')
errors=[x for x in log.splitlines() if 'FATAL EXCEPTION' in x or 'Fatal signal' in x or 'Process: org.rerac.android.ui035test' in x]
print('App PID:',pid)
expected_launcher=os.environ.get('EXPECT_LAUNCHER','1.1.0-beta.5')
expected_core=os.environ.get('EXPECT_CORE','0.3.5')
print('Launcher footer:',[x for x in texts if 'Launcher '+expected_launcher in x])
print('Fatal log lines:',errors[-12:])
assert pid,'Process died after opening launcher'
assert any('Launcher '+expected_launcher in t and 'ReRAC '+expected_core in t for t in texts),'No launcher footer'
assert not errors,'Fatal runtime exception'
assert any('Mods' in t for t in texts) and any('Updates' in t for t in texts),'Missing Mods or Updates'
Path('work/reports/launcher-test.json').write_text(json.dumps(dict(status='passed',launcher_version=expected_launcher,core_version=expected_core,package_id='org.rerac.android.ui035test',startup_fatal_exceptions=0),indent=2)+'\n')
PY
printf 'launcher_startup=passed\n' > work/reports/result.txt
