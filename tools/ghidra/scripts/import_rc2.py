#!/usr/bin/env python3
# Ratchet 2 (Going Commando, SCUS_972.68) counterpart of import_levels.py. See docs/workflows/ghidra.md.
import functools; print = functools.partial(print, flush=True)  # unbuffered progress when run in the background
"""Imports the Ratchet 2 boot ELF (work/ghidra-import/rc2/SCUS_972.68, as /SCUS_972.68) and every level ELF
(work/ghidra-import/rc2/levelNN.elf, from overlay_to_elf.py --rc2-dir) into the open Ghidra project, analyses
them and saves. Refuses to run unless the open project is `ratchet2`. Idempotent: skips programs that exist.

    import_rc2.py [NN ...]      default: all 27 levels (00..26)

Programs are named levelNN.elf under /levels, like the Ratchet 1 project. There is no name source for this game
yet (no Lombyte): names come from Version Tracking against the Ratchet 1 programs afterwards.
"""
import os, sys, time
sys.path.insert(0, os.path.dirname(__file__))
from ghidra_http import get, post

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "../../.."))  # repo root (tools/ghidra/scripts/)
IMPORT_DIR = os.path.join(ROOT, "work/ghidra-import/rc2")
BOOT = "SCUS_972.68"
PROJECT = "ratchet2"
levels = [int(a) for a in sys.argv[1:]] or list(range(27))

def files_in(folder):
    files = get("list_project_files", folder=folder)
    return files if isinstance(files, dict) else {}

def wait_for_analysis(name):
    while True:
        st = get("analysis_status", program=name)
        if isinstance(st, dict) and not st.get("analyzing", st.get("is_analyzing", False)):
            return
        time.sleep(5)

def import_one(name, path, folder):
    t = time.time()
    res = post("import_file", file_path=path, project_folder=folder, auto_analyze=True)  # no language: the ELF loader runs and the EE plugin picks r5900 from e_flags
    if not (isinstance(res, dict) and res.get("success")):
        print(f"{name}: import failed: {str(res)[:300]}")
        return False
    wait_for_analysis(name)
    post("save_program", program=name)
    post("close_program", name=name, save=True)
    print(f"{name}: imported, analysed, saved and closed in {time.time() - t:.0f}s")
    return True

root = files_in("/")
if root.get("project_name") != PROJECT:
    sys.exit(f"the open Ghidra project is {root.get('project_name')!r}, not {PROJECT!r}: open ~/Ghidra/{PROJECT}.gpr first")

if BOOT not in {f["name"] for f in root.get("files", [])}:
    import_one(BOOT, os.path.join(IMPORT_DIR, BOOT), "/")
else:
    print(f"{BOOT}: already in project")

have = {f["name"] for f in files_in("/levels").get("files", [])}
for lv in levels:
    name = f"level{lv:02d}.elf"
    if name in have:
        print(f"{name}: already in project, skipping")
        continue
    import_one(name, os.path.join(IMPORT_DIR, name), "/levels")
print("done")
