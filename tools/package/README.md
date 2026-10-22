# Native packaging

`cargo xtask package` builds and packages the native runtime and extractor locally. `--no-build` packages binaries already in `target/release`. It requires Bash and the platform's archive tool.

Inputs are `rerac`, `rerac-extract`, the shaders under `crates/rc-engine/assets`, and the workspace version. Output is the ignored `dist/rerac-<version>-<os>-<arch>/` folder and ZIP.

The package allow-list permits only the two binaries, shader files, `rerac-manifest.json` and `README.txt`. Game data and development tools are excluded. This command creates local packages only; it does not publish to GitHub.
