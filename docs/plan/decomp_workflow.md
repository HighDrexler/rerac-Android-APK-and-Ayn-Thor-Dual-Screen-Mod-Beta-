# Decompilation workflow

The step-by-step workflow (set-up from scratch, naming, export, backups) is `docs/workflows/ghidra.md`; the scripts
are described in `tools/ghidra/README.md`. This page keeps the conventions and the history of the naming runs.

The Ghidra project lives outside the repo: `~/ratchet1.gpr` + `~/ratchet1.rep` (Ghidra GUI project `ratchet1`; its
target home is `~/PS2/ratchet1/ghidra/`, which the user moves it to). The GhidraMCP plugin exposes an HTTP API on
`127.0.0.1:8089` while the GUI is open; the `ghidra` MCP server registered for Claude bridges to the same API.
Scripts in `tools/ghidra/scripts/` talk to it directly (moved from `decomp/scripts/` on 2026-09-27; not re-run since,
paths updated).

| Script | Purpose |
|---|---|
| `ghidra_http.py` | Minimal GET/POST client for the plugin API |
| `import_levels.py [NN ...]` | Imports `extracted/boot/SCUS_971.99` (if missing) and `work/ghidra-import/levelNN.elf` into `/levels`, analyses, applies Lombyte's boot-match names (renamed from `import_overlays.py`) |
| `apply_boot_names.py` | Renames boot ELF functions from `tools/ghidra/names/boot_functions.csv` (derived from Lombyte's splat config and semantic header) and records the source path as a plate comment |
| `overlay_diff.py` | Clusters functions across the boot ELF and the 19 overlays by a relocation-tolerant hash; writes `tools/ghidra/names/clusters.tsv` and `overlay_names.csv` |
| `apply_overlay_names.py` | Applies `tools/ghidra/names/overlay_names.csv` to the level programs |
| `apply_doc_names.py` | Applies the names our docs give (`tools/ghidra/names/doc_names.csv`), below |
| `export_decomp.py <program>` | Dumps every function's decompiled C to `work/decomp/<program>/` with an `index.tsv`, for grepping without the GUI |
| `export_overlays.py` | Level01 in full, every other level only its non-shared functions, to `work/decomp/levelNN.elf/` |

Conventions:

* Programs: `SCUS_971.99` is the boot ELF. Overlays are `levelNN.elf`; their
  addresses overlap the boot ELF's game-code window, so they are never merged.
* Names: boot ELF function names come from Lombyte's per-function source file
  names (snake_case, e.g. `check_state_range`), with Lombyte's PascalCase
  semantic name kept in the plate comment when one exists. Names we derive
  ourselves go in `tools/ghidra/names/*.csv` so they can be re-applied to a fresh
  project.
* Types: struct definitions we recover are written to `src/game/**/*.h` as the
  source of truth and pushed into Ghidra, not the other way round, so the port
  and the database never disagree.
* Re-export after any batch of renames; `work/decomp/` is git-ignored because
  it contains the game's code.

Ground truth beyond Ghidra: `~/Globals/Lombyte` (matching decomp of this exact
ELF; `src/` has 1844 reconstructed C files as of 2026-10-20 — pull it: it moves fast) and `~/Globals/rac1-decomp` (PAL
v2.00, addresses differ).

## Rust toolchain note

Rust is installed through Homebrew's rustup. The shims live in
`/opt/homebrew/opt/rustup/bin`; put that directory on `PATH` (the `~/.cargo/bin`
proxies are not created by this install method):

```
export PATH="/opt/homebrew/opt/rustup/bin:$PATH"
cargo xtask test-job formats # unit + golden tests (golden tests need extracted/)
cargo run -p rc-engine       # Bevy app
```

## Names from our own docs (`apply_doc_names.py`)

Agents reverse-engineering subsystems name functions and globals in
`docs/plan/*.md` and `docs/formats/*.md`. Those identifications are collected in
`tools/ghidra/names/doc_names.csv` (`program, address, name, kind, source_doc,
confidence, note`). There is one row per (program, address, kind). When docs
disagree, the row keeps the preferred name, and the note lists the others as
`alt X (doc; confidence)`. A name the docs call a misnomer loses to the
alternative, and the note records `Lombyte name believed wrong: X`.

```
python3 tools/ghidra/scripts/apply_doc_names.py --dry-run            # plan + log, touches nothing
python3 tools/ghidra/scripts/apply_doc_names.py --apply --export     # rename/comment, save, re-export touched functions
python3 tools/ghidra/scripts/apply_doc_names.py --apply --programs level01   # limit to some programs
python3 tools/ghidra/scripts/apply_doc_names.py --sync-export        # re-export any export entry whose name is stale
```

Conventions:

* **program.** `boot` means `/SCUS_971.99`. `levelNN` means `/levels/levelNN.elf`.
  The project also holds stale raw imports under `/overlays/` with the same file
  names, so always address a program by its project path. A plain `level01.elf`
  resolves by name to whichever program is found first, and `close_program`
  with a bare name closes both. Addresses below the overlay base `0x15ef00` are
  boot memory shared by every level, so they are `boot` rows even when a
  level01 doc cites them. Lowercase `fun_XXXXXXXX` in the docs is a boot
  address, not a level01 one.
* **Names.** The `name` column follows the program's majority style: boot names
  are snake_case like Lombyte's file names (the doc's PascalCase original is in
  the note as `doc name X`), and overlay names are the doc's PascalCase. Globals
  use `g_snake_case`. Confidence is `verified` (checked against
  disassembly/decompile), `inferred`, or `suggested`. Suggested names include
  names the harvest coined from a doc's description; their note starts with
  `coined:`.
* **Lombyte names win.** Only default names (`FUN_`, `fun_`, `func_`, `thunk_`,
  `LAB_`, `DAT_`/`PTR_` for data) are replaced. An existing name that differs
  gets a plate line `doc alias: <name> (<doc>)` instead. Plate lines are
  appended, never replace an existing plate, and are never duplicated, so
  re-runs are no-ops.
* **Cluster propagation.** A named function passes its name to every member of
  its hash cluster in `clusters.tsv`, in the other 19 programs, under the same
  rules. Clusters that have two members in one program are skipped as too
  generic. Data is never propagated.
* **Labels.** Data gets labels through `create_label`. `rename_symbol` runs the
  plugin's Hungarian-prefix gate, which has no `strict_mode` override. A doc
  function at an address where Ghidra has no function (code reached only
  through pointer tables, such as particle updates) gets a label, not a new
  function.
* **Logs.** `work/decomp/doc_names_apply.tsv` holds one line per action with
  the old name, so a batch can be undone. It is git-ignored.

First run (2026-09-26):

* **Input.** 1631 harvested identifications were merged into 1357 rows (801
  functions, 556 data). 166 addresses had competing names.
* **Functions renamed from doc rows:** 80 boot, 479 level01.
* **Functions renamed by cluster propagation:**

  | Program | Renamed | | Program | Renamed |
  |---|---|---|---|---|
  | boot | 20 | | level09 | 349 |
  | level00 | 340 | | level10 | 393 |
  | level01 | 31 | | level11 | 372 |
  | level02 | 362 | | level12 | 413 |
  | level03 | 374 | | level13 | 361 |
  | level04 | 356 | | level14 | 365 |
  | level05 | 395 | | level15 | 353 |
  | level06 | 347 | | level16 | 374 |
  | level07 | 357 | | level17 | 345 |
  | level08 | 368 | | level18 | 350 |

  That is 6625 in total.
* **Alias plate comments** (the function was already named): 45 from doc rows
  and 410 from propagation.
* **Labels:** 270 boot globals, 274 level01 globals, and 23 level01 code labels.
* **Rejected rows:** 6 doc addresses that are inside a function rather than at
  its start, and 2 level01 addresses that are not mapped in the program. The
  apply log lists both.
