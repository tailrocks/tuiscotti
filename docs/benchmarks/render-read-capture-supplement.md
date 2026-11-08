# Full-corpus render-read and selected capture supplement

This record adds two native macOS arm64 feasibility observations to the
provisional level-22 benchmark. It does not change the selection status or
satisfy the complete requirements in
[the original benchmark specification](../COMPRESSED-BASELINES.md).

## Full committed TCB PB corpus: render-read

One serial CLI `render --out` pass attempted all 2,117 committed `.tui.pb`
paths from TCB commit
`534e0ff5af75b840d93038dd02ece0839cf4f71b`. The source checkout was read
only. The CLI binary was built from Tuiscotti PR head
`52cbf592c950eef5afa89816ff071bcc0d2842ce`, SHA-256
`4c39bc547a0444e5b41d3ac9f5ddb2aaddcb7b9dca30913d5771208b2a15712c`.
This is not a build from current main `8d2bd88`.

- 2,055 renders succeeded; 62 returned errors.
- Eight deliberately truncated coverage fixtures returned
  `INVALID_SAMPLE`.
- Fifty-four valid samples returned `MISSING_GLYPH` from the pinned font's
  fail-closed renderer: 45 U+26B7, six U+2387, and one each U+1F469, U+29C9,
  and U+26BF. These errors span 51 protected historical-root samples and
  three selected functional-root samples. They are not malformed protobufs
  and were not silently skipped.
- The 2,055 successful PNGs total 381,590,165 bytes. Every output existed and
  its SHA-256 matched its per-sample record. Input manifests before and after
  the run match.

A post-run source-binding check verified the TCB checkout was clean at the
stated commit. For all 2,117 PB paths, the Git blob object ID and working-tree
SHA-256 matched the commit tree, the pre/post manifests, and the raw render
record; recorded file sizes also matched. The verifier and per-path results
are included in the evidence archive.
- Observed pass wall time was 260.15 s; the maximum observed child wall time
  was 0.75 s and maximum observed child RSS was 137,478,144 bytes.

The runner checked each child RSS only after child exit and checked the
pass deadline between launches. A per-child timeout was configured. Those
checks do not enforce a hard memory cap or a hard overall-pass deadline.
The filesystem cache was warm and the host had unrelated concurrent load, so
these are feasibility observations, not performance rankings.

The 1,868 PB files in `pre_refactor/` are protected historical-root samples;
some have live test consumers. They are not described as inactive archives.
The separate 179 selected current functional roots comprise 121 functional,
57 Jackin, and one buttons sample. The remaining 70 repository PBs include 62
valid samples and the eight invalid coverage fixtures.

## Selected functional capture timing and sampled RSS

A separate warm single run used the ordinary functional tests
`holla::holla_help_overlay`, `showcase::showcase_page_diff`, and
`tablepro::tablepro_resize_and_too_small_recovers` against the same TCB
commit. All three tests passed and five normal visual-gate actions passed.
The app's ordinary frozen checks remained enabled. The only TCB source change
was disposable instrumentation in `tests/functional/support.rs`; its patch
and original/instrumented source hashes are in the evidence archive.

The timer wraps only `Session::capture_snapshot(ActiveScreen)` at ordinary
`screen()` callsites. Across four app sessions it recorded 225 calls: pooled
median 128.041 µs, p95 137.792 µs, and maximum 329.833 µs. The p95 index is
`floor(0.95 * (n - 1))`. Four app-drop final-screen calls are recorded
separately and excluded from those statistics. These are single-run
observations, not a stable latency distribution. They exclude waits, child
lifecycle, frozen comparisons, and post-return hashing/logging; there is no
per-call CPU or memory measurement.

A saved Python sampler polled `ps` every 100 ms while the selected tests ran.
It recorded 12 Holla samples (maximum observed 9,488 KiB), 12 Showcase
samples (12,576 KiB), and 56 TablePro samples (6,944 KiB across the two
TablePro app sessions). The raw timestamp, PID, full command, and RSS values
are preserved. Sampling can miss brief processes and peaks; these are
observed per-app samples, not guaranteed maxima.

The post-run SHA-256 check covered 1,990 baseline PB files and matched the
pre-run manifest. The run used isolated MBX cache and target roots with
automatic collection disabled. Session metadata records the Tuiscotti
comparator dependency revision as `92ef2d9ee082b35f68e066c5129fedbf07675a80`;
the measurement does not establish source equivalence to current main.

## Evidence and remaining requirements

The compact archive
[`tuiscotti-arm64-render-capture-20261108.tar.gz`](../../benchmark-evidence/tuiscotti-arm64-render-capture-20261108.tar.gz)
(SHA-256
`32e5baf7101baaa13b8c882edc7415f372742421c7564231ad09e53b63a31812`)
contains per-PB render results and input manifests, a Git-blob binding check, render runner and command logs,
capture sampler and raw samples, per-call timing records, selected-test
actions, binary identities, source patch/hashes, tool versions, environment,
and baseline manifests. It excludes binaries, rendered PNG files, and
checkpoint frame payloads. The archived scripts retain paths from the
measurement host; replay requires the referenced TCB source checkout and CLI
binary.

This supplement does not provide full-corpus paired JSON/PB measurements,
per-stratum paired codec runs, five repetitions, encode/decode CPU or RSS,
full-corpus capture resource cost, native Linux results, or a repeated
same-logical-update Git history sequence. It does not establish the chosen
Zstandard level as final.
