# Repository guard tests

Run `cargo xtask test-job --test guards` for these checks alone, or `cargo xtask test-job repo` alongside unit tests. They are also part of `cargo xtask test-full`. The tool has no external dependencies and ships no runtime files.

The guards verify:

1. Product crates in `crates/` never depend on development tools.
2. Product sources and tests contain no personal paths or session output; tests do not read disc images or home-directory files.
3. The top level contains only source, tools, documentation, build configuration, the README and license, plus ignored development output.
4. Test instructions use `cargo xtask test-*` and keep the shared Bevy feature set.

See [testing](../../docs/workflows/testing.md) for commands and data requirements. The tests read the repository without changing files.
