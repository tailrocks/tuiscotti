# Benchmark planning, not yet approved

No compression selection measurements have run. The final design must freeze the corpus, resource budgets, and exact commands before measurement.

Historical source is terminal-components-baseline commit `60b5fb2762d91a3549bd4ac63b1d77d534ee53d2`. Its 1,986 tracked `.tui.json` paths include eight small coverage-fixture sentinels; enumerate the remaining captures and verify hashes and sizes before calling them representative. Preserve existing checkouts. Use disposable worktrees and separate output directories.

The historical samples inspected so far contain only state gates. Fresh captures are needed for both gates, representative platform contracts, PNG-heavy samples, and realistic changes. Build separate old and new research capture programs against the same deterministic application inputs. Each uses its own version's public capture and encoding APIs. Neither reads the other's storage format. Pair by capture manifest and compare a complete in-memory domain projection and exact PNG hash. Record unmatched pairs.

Measure canonical JSON, compressed JSON, raw Protobuf, and compressed Protobuf, plus Zstandard, XZ/LZMA2, Brotli, and gzip settings. Test Zstandard levels 1, 3, 9, 12, 15, 18, 19, 20, 21, and 22 with a 32 MiB window ceiling. Interleave at least five passes, consume and verify every output, and measure per-file sizes, wall and CPU time, peak RSS, and end-to-end capture, check, accept, and render-read costs. Avoid build or test contention. Record environment, cache conditions, failures, source hashes, versions, commands, run order, and dispersion.

Proposed limits requiring review before measurements: one process at a time, 1 GiB peak RSS per process, 60 minutes per complete corpus pass, and 72 hours cumulative selection time. A pass exceeding any limit fails. Choose the highest measured Zstandard level meeting all limits and correctness requirements, and report the smallest result for all codecs separately. These limits are proposals, not evidence of achievable runtime or memory.

Create isolated Git object databases from identical real capture histories. Include sparse cell, style, palette, metadata, pixel, addition, and deletion changes. Measure fresh and migrated histories with identical documented repack settings. Report checkout size separately from Git storage and do not infer history savings from checkout savings. Record raw per-run data and publish aggregation commands.

Open: freeze representative capture inputs and resource budgets, implement the producer and measurement tools, finish the complete corpus inventory, establish runnable commands, capture all required strata on the upgraded implementation, and independently review the selection.

## Contended-run amendment (2026-11-05)

The original freeze marked contended runs ineligible for timing-based selection. The user requested the shortest timeline and no native Linux host is available. Five interleaved release-mode passes were therefore run on the real macOS capture corpus under sustained unrelated host load, with the original flag and raw observations preserved. These runs may support a provisional choice based only on the stated absolute per-process memory and per-pass time caps when every run passes with ample margin; they do not support a relative-speed ranking or final cross-platform claim. The final encoder, end-to-end costs, broader workloads, and native Linux determinism remain separate gates. This amendment is recorded after the runs and must be disclosed with any use of their results.
