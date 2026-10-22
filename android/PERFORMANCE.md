# Launcher 1.1 performance review

The companion now reuses its parsed quick-select state when the native snapshot is unchanged. Previously every draw and touch parsed JSON, allocated page/slot/quad lists and rebuilt the glyph array, even between native publications. New packets still refresh immediately. A failure clears the cache; replaying the same valid packet recovers input. Generation changes continue to clear textures and page state.

A desktop JVM microbenchmark using the three-page test packet (1,239 characters), after 2,000 warm-up iterations, measured 10,000 repeated reads at 196.98 ms without caching and 0.223 ms with caching. This isolates redundant parsing, not total companion or engine time. It is not an Android FPS, battery or memory benchmark. A separate desktop thread-allocation measurement for those 10,000 reads recorded 408,793,480 bytes allocated by repeated parsing and zero bytes in the unchanged-packet cache path. JNI string retrieval and the Android rendering loop are outside that measurement. The cache retains one packet and parsed model, not a history.

The regression test verifies object reuse over 1,000 reads, changed packets, failed parsing and identical-packet recovery.

The review retained the existing native publication throttle (90 ms during gameplay, 40 ms in menus), lazy icon decoding, generation-bound Java texture cache and native dynamic-texture pruning. Live-map composition is already gated by map visibility and the secondary display. Snapshot conversion does not advance gameplay RNG. No additional per-frame service, polling thread, telemetry or library was added.

Both ARM64 and x86-64 remain source-built with stripped native symbols and 16 KiB library alignment. The APK's game data, extraction libraries and other DEX files remain byte-identical to the prior 1.1. Android minimum version and update compatibility checks remain unchanged. Existing rendering-resolution controls remain available for devices with different GPU budgets; this fix does not change image-quality or simulation defaults.

Validation covers native input/menu behavior, all 30 real-game skill descriptions, companion routing/layout tests and the actual installed updater. Android fixture tests cover multiple display shapes; they do not establish sustained performance on physical Thor, low-end phones or every GPU. Device frame-time and thermal profiling are still needed before claiming a measured game-performance gain across hardware.
