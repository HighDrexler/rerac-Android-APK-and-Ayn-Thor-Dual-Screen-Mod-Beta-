# Game data

The project supplies no game data. Use an original USA v1.00 Ratchet & Clank disc image (SCUS-97199).

## Extract and verify

Run from the repository root:

```sh
cargo run --release -p rc-extract -- identify --iso "<disc>.iso"
cargo run --release -p rc-extract -- extract --iso "<disc>.iso" --out extracted
cargo run --release -p rc-extract -- verify --out extracted
cargo run --release -p rc-engine -- --data-dir extracted
```

The extractor verifies supported files against its committed size/SHA-1 table and prepares `cache/v1`. Rebuild the cache without rereading the disc with `cargo run --release -p rc-extract -- prepare --out extracted`. The runtime rebuilds missing or stale cache entries; `RC_CACHE=0` bypasses that cache.

`cargo xtask regen-data --iso "<disc>.iso"` safely rebuilds the development data folder: classify its existing contents, extract into staging, verify, then swap. Unknown files stop it. `--check` only lists contents. See [the task tool](../../tools/xtask/README.md) for options and safety checks.

## Paths

The runtime chooses its data folder from `--data-dir`, then `RC_DATA_DIR`, then `RC_EXTRACTED`, then `<repo>/extracted`. Tests use `RC_EXTRACTED` or that development default. A data folder contains the extractor's archive, `extract-info.json`, and generated cache. Keep personal and generated data out of Git; the repository ignores `extracted/`, `iso/`, `assets/`, `out/` and `work/`.

`rerac --version-json` reports the native source version without opening a window or reading game data. The runtime's version is independent of the Android launcher version.

## Optional exports

```sh
cargo run --release -p rc-extract -- export --out extracted --to work/exports
```

This writes PNG, WAV, glTF and JSON files for inspection. `--what` selects textures, audio, models, levels, collision or text; `--level NN` selects a level. The game does not consume these exports. Keep them under the ignored `work/` folder.
