# Testing

Run from the repository root. Rust and Cargo must be on PATH. Use `cargo xtask test-*` so tests share the dev Bevy build; never run `cargo test` or `cargo nextest` by hand with a different feature set.

## Commands

| Task | Command |
|---|---|
| Unit tests | `cargo xtask test-quick` |
| Unit tests for one crate, with nextest | `cargo xtask test-quick rc-game` |
| Unit tests and affected integration suites | `cargo xtask test-job hero ui` |
| One integration binary | `cargo xtask test-job --test guards` |
| One module | `cargo xtask test-job --test ui --filter pause_pages_novalis::` |
| One exact test | `cargo xtask test-job --test hero --filter hero_novalis::novalis_hero_digest --exact` |
| Complete suite and shared-code digest check | `cargo xtask test-full` |
| Type check all sources and tests | `cargo check-all` |
| Lints | `cargo clippy-all` |

Test commands set `RC_AUDIO=0`. They use cargo-nextest when installed, otherwise Cargo's test runner with the same `--workspace --features rc-engine/dev` flags. `--cargo-test` selects that fallback. `--ignored` runs ignored surveys; `--nocapture` shows their output. Every failed step contributes to a failing exit status.

## Areas

The test runner's `tools/xtask/src/test.rs` defines these areas and aliases:

| Area | Integration binaries |
|---|---|
| `hero` | `hero` |
| `weapons` | `weapons` |
| `classes` (`creatures`, `mobys`) | `classes` |
| `world` (`levels`, `collision`, `water`) | `world` |
| `ui` (`menus`, `hud`, `map`, `save`, `vendor`) | `ui` |
| `audio` | Audio modules in `ui` and the Ryno sound test in `weapons` |
| `formats`, `data`, `extract` | Their corresponding integration binaries |
| `video` (`movies`) | `movies` |
| `engine` (`render`, `input`) | Unit tests only |
| `particles` (`fx`) | Particle-consumer tests in `classes` |
| `repo` (`guards`, `layout`) | `guards` |
| `shared` | All-level smoke tests and the hero digest comparison |

## Game data and baselines

Tests use the committed fixtures and extracted data at `RC_EXTRACTED`, or `<repo>/extracted`. Data-dependent tests skip when that data is absent. Supply data extracted from a supported disc for a complete gameplay check; a skipped test does not verify that path. See [game-data.md](game-data.md).

Tests must not read personal disc images, savestates, recordings, home-directory files or generated session output. Repository guards enforce that boundary. Keep distilled regression fixtures committed alongside the tests that consume them.

`test-job shared` and `test-full` compare the NO_IDLE hero digest against `work/test-results/hero_digest_no_idle.txt`. Create that local baseline before editing with `cargo xtask digest-baseline`; replace it only after intentionally changing gameplay and reviewing the difference. Test runs never rewrite it. A filtered or `--ignored` job skips the digest.

`test-full` finishes with an optional `cargo xtask sweep` to trim old build output. Missing cargo-sweep or a refused sweep warns without hiding test failures. See [the task tool](../../tools/xtask/README.md).
