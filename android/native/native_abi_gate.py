#!/usr/bin/env python3
"""Fail closed if an Android APK loses v16's native entry points.

An ABI pass alone does NOT prove the game runs. No source rewriting or signing.
"""
import argparse
import hashlib
import json
import re
import subprocess
import tempfile
import zipfile
from pathlib import Path

ABIS = {"arm64-v8a": "AArch64", "x86_64": "Advanced Micro Devices X86-64"}
NATIVES = (
    "nativeCompanion nativeDisplayRate nativeDualEnabled nativeIcon nativeInput "
    "nativeMapView nativeMapVisible nativeMenuButton nativeMenuGesture nativeMenuMode "
    "nativeMenuTouch nativePause nativePlanetIcon nativeSecondary nativeSelect nativeSetDual "
    "nativeTouchInput nativeTouchMode nativeWheel"
).split()
EXPORTS = {"android_main", "ANativeActivity_onCreate"} | {
    "Java_org_rerac_android_GameActivity_" + name for name in NATIVES
}
SOURCE_COMPANION_EXPORTS = {
    "Java_org_rerac_android_NativeCore_" + name
    for name in ("buildInfo", "menuChoice", "companionInputStatus", "quickSelect", "quickSelectSnapshot")
}

def inspect(elf, abi, source_companion=False):
    hdr = subprocess.check_output(["readelf", "-h", str(elf)], text=True)
    sym = subprocess.check_output(["readelf", "--dyn-syms", "--wide", str(elf)], text=True)
    found = set()
    for line in sym.splitlines():
        parts = line.split()
        if len(parts) >= 8 and parts[3] == "FUNC" and parts[6] != "UND":
            found.add(parts[7].split("@")[0])
    m = re.search(r"^\s*Machine:\s*(.*)$", hdr, re.M)
    machine = m.group(1).strip() if m else ""
    required = EXPORTS | (SOURCE_COMPANION_EXPORTS if source_companion else set())
    missing = sorted(required - found)
    ok = ABIS[abi] in machine and not missing
    return {"abi": abi, "machine": machine, "machine_ok": ABIS[abi] in machine,
            "missing_exports": missing, "required_exports": len(required),
            "bytes": elf.stat().st_size, "sha256": hashlib.sha256(elf.read_bytes()).hexdigest(),
            "ok": ok}

def check(apk, source_companion=False):
    results = []
    with zipfile.ZipFile(apk) as z, tempfile.TemporaryDirectory() as tmp:
        for abi in ABIS:
            path = f"lib/{abi}/librc_engine.so"
            if path not in z.namelist():
                results.append({"abi": abi, "ok": False, "error": "Missing " + path})
                continue
            target = Path(tmp) / (abi + ".so")
            with z.open(path) as src, target.open("wb") as dst:
                import shutil
                shutil.copyfileobj(src, dst)
            results.append(inspect(target, abi, source_companion))
    return {"schema": 1, "candidate": apk.name, "checks": results,
            "ok": all(x["ok"] for x in results)}

def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("apk", type=Path)
    ap.add_argument("--baseline", type=Path)
    ap.add_argument("--report", type=Path)
    ap.add_argument("--require-source-companion", action="store_true",
                    help="Require the rebuilt companion controller's JNI entry points")
    args = ap.parse_args()
    try:
        result = check(args.apk, args.require_source_companion)
        if args.baseline:
            base = json.loads(args.baseline.read_text())
            assert base["ok"] and {x["abi"] for x in base["checks"]} == set(ABIS)
            assert all(x["missing_exports"] == [] for x in base["checks"])
    except (OSError, ValueError, AssertionError, zipfile.BadZipFile, subprocess.CalledProcessError) as e:
        ap.exit(2, "native ABI gate failed: " + str(e) + "\n")
    report = json.dumps(result, indent=2, sort_keys=True) + "\n"
    print(report)
    if args.report:
        args.report.parent.mkdir(parents=True, exist_ok=True)
        args.report.write_text(report)
    return 0 if result["ok"] else 1

if __name__ == "__main__":
    raise SystemExit(main())
