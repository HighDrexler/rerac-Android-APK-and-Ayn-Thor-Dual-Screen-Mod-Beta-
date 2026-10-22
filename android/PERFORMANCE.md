# Launcher 1.1 — performance pass, build 39

The companion's normal world/map display is now independent of permission to switch weapons. Ledge hangs, platforming and ambient interaction ownership keep the map visible; the original hand/body/interaction guards still reject prohibited equipment changes. Vendor mode keeps the map too. Actual menus, freeze dialogs, cinematics and level loading retain their separate presentation.

The native map projection now compares the exact fog mask, source-file identity, level, Map-o-Matic set and palette before composing a new image. When those inputs are unchanged, it reuses the picture and Android texture key. Hero position/yaw, camera pan/zoom, mirror, objectives and markers still update. Fog-only changes use the original bitmap compositor and palette decoder while retaining the three unchanged globe layers. File, palette or Map-o-Matic changes run the complete original composition path. Level/language generation changes reset the cache.

The comparison uses bytes, not a lossy hash. A retained 32 KiB mask is reused in place. Companion image keys have a separate namespace from the primary pause map to prevent stale-texture collisions. Quick-select pages now share one isolated ring/global copy per publication, rather than cloning one per page. Unchanged selection receipts no longer allocate duplicate JSON objects each simulation tick. These changes do not write game state, alter RNG, relax equipment guards, change simulation timing or change graphics-quality defaults.

## Measurements

These are Windows x86-64 desktop **native companion microbenchmarks**, using Rust 1.99.1, a Cargo release build and the owner's extracted game data. They are not Android FPS or resident-memory measurements. Shipping Android retains its existing native compiler profile. The tests exclude the primary renderer, GPU, JNI, Android Bitmap allocation, thermals and battery use.

Each run measured 600 paired map updates per workload on Novalis, Blarg Station and Oltanis. Old/new order alternated on every update. Two full runs were recorded. The wheel test measured 3,000 three-page publications per variant per run and reversed variant order on the second run. Startup, asset loading and warm-up are excluded; allocation-meter instrumentation is enabled during measurement.

| Workload | Previous median time | New median time | Temporary allocation bytes per refresh, previous → new |
|---|---:|---:|---:|
| Move within already revealed map | 809–859 µs | 2.6–3.0 µs | ~2,593,000 → ~34,600 (**98.7% less**) |
| Fog changes once every 20 updates | 825–1,000 µs | 2.7–3.7 µs | ~2,593,000 → ~152,600 (**94.1% less**) |
| Fog changes on every update | 828–868 µs | 794–834 µs | ~2,593,000 → ~2,394,000 (**7.7% less**) |
| Three quick-select pages | 7.0–8.4 µs | 6.1–7.9 µs | 12,974 → 12,298; allocation count **32 → 20** |

Time columns show the range of medians across the measured levels/runs, not individual frame bounds. The continuously changing fog case averages about 3.7% lower median CPU time across those measurements; small timing gains are host-dependent. In the unchanged-fog case the new path rebuilt **0** map textures after warm-up versus **600** in the previous path. At one fog change per 20 updates it rebuilt **30**, preserving every change.

Allocation bytes are cumulative requested temporary heap allocations, not live/resident RAM. The cache retains one mask, source reference and current picture; it does not retain a history. Existing native dynamic-image pruning and Android texture pruning remain active. No service, polling thread, telemetry or dependency was added.

Full per-run medians, p95s, allocation counts and rebuild counts are in [performance-build39.json](validation/performance-build39.json).

## Equivalence and regression checks

Before timing, each run compared the original and optimized projection on **all 19 maps, both picture sets, 64 transitions each: 2,432 comparisons**. The checks compare exact fog pixels, globe-layer pixels, native draw geometry, camera transforms, markers and mission state, including moving/yawing heroes, movement group 15, mirror, the level-6 alternative transform and changed mission flags. No game data is committed with the benchmark.

Native CI covers fog/palette/file/Map-o-Matic invalidation, camera movement with stable texture identity, pixel channel/alpha conversion, original wheel presentation/ammo, page-copy equivalence and unchanged equipment guards. Android instrumentation covers the normal Home and dedicated Map layouts while weapon changes are blocked, actual-loading/cinematic gating, and the existing input/pause/skill regressions. The signed build also undergoes the live installed-updater test.

Reproduce with supported extracted data available through `RC_EXTRACTED`:

```text
cargo run --release --locked -p rc-game --example companion_benchmark
```

Run again with `PERF_REVERSE=1` to reverse wheel variant order; map timing remains interleaved. This example is a development tool and is not an APK component.

Build 38's unchanged-packet Java parsing cache remains in place, including identical-packet recovery after failure. Existing 90 ms gameplay / 40 ms menu publication intervals, lazy icon decoding, rendering-resolution controls, both Android ABIs and 16 KiB native-library alignment remain intact.

Physical-device frame-time, thermal and battery profiling is still required before claiming a whole-game performance gain on Thor or other GPUs. No physical-device FPS claim is made here.
