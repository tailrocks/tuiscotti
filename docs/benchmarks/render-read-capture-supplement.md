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

A later warm run used the ordinary functional tests
`holla::holla_help_overlay`, `showcase::showcase_page_diff`, and
`tablepro::tablepro_resize_and_too_small_recovers` against the same TCB
commit. Nextest run `b6242c4c-48ec-4eee-a5a8-cab13fed8181` exited 0: all
three tests passed, 2,021 were skipped, and five normal visual-gate actions
passed. The ordinary frozen checks remained enabled.

This rerun binds the measurement to the exact test harness and source state.
The fresh clone was clean before applying the instrumentation. Retained
pre/post Git status and full binary diff show that `tests/functional/support.rs`
was the only changed file and that its diff was unchanged across the run. The
`functional` test executable SHA-256 was
`45993aac4351f686d8eb224c010ca290a66722f890139ac228f3cb9d0d5f24c2` before
and after. Each spawned app executable's path and SHA-256 are recorded in its
session metadata. This was an instrumented disposable checkout at commit
`534e0ff`; it is not a clean-build binary from current main.

The timer wraps only `Session::capture_snapshot(ActiveScreen)` at ordinary
`screen()` callsites. Across four app sessions it recorded 211 calls: pooled
median 275.208 µs, p95 531.625 µs, and maximum 11,586.083 µs. The p95 index is
`floor(0.95 * (n - 1))`. These are single-run observations, not a stable
latency distribution; the maximum may reflect scheduling or instrumentation
noise. They exclude waits, child lifecycle, frozen comparisons, and
post-return hashing/logging. There is no per-call CPU or memory measurement.

A saved Python sampler polled `ps` every 100 ms while the selected tests ran.
Observed maximum samples were Holla 9,840 KiB, Showcase 12,752 KiB, and
TablePro 6,928 KiB. The archive retains each timestamp, PID, command and RSS
row. Sampling can miss brief processes and peaks; these are observations, not
guaranteed maxima.

The post-run SHA-256 check covered 1,990 baseline PB files and matched the
pre-run manifest. The run used the clone's trusted Mise configuration with
`mise exec -- mbx nextest`, an isolated MBX cache and target root, and
`MBX_GC_AUTO=0` plus `MBX_SESSION_GC_AUTO=0`; the exact invocation,
environment and MBX settings output are archived. Session metadata records
the Tuiscotti comparator dependency revision as
`92ef2d9ee082b35f68e066c5129fedbf07675a80`; the measurement does not establish
source equivalence to current main. An earlier exploratory capture run is
also preserved separately in the archive but is not used for these figures.

## Evidence and remaining requirements

The compact archive
[`tuiscotti-arm64-render-capture-20261108.tar.gz`](../../benchmark-evidence/tuiscotti-arm64-render-capture-20261108.tar.gz)
(SHA-256
`3185dff7465cccc242d01e91fd71ec9b6e8a5d115a6a434276c3d5c9b620f724`)
contains per-PB render results and input manifests, a Git-blob binding check, render runner and command logs,
both capture runs, including the source-bound rerun's full Git status/diff, functional test executable SHA-256 before/after,
per-session app identities, raw samples, per-call timing, actions, settings,
and baseline manifests. It excludes binaries and rendered PNG files, and omits 40 checkpoint frame JSON/text payloads (19,820,379 uncompressed bytes). The archived scripts retain paths from the
measurement host; replay requires the referenced TCB source checkout and CLI
binary.

This supplement does not provide full-corpus paired JSON/PB measurements,
per-stratum paired codec runs, five repetitions, encode/decode CPU or RSS,
full-corpus capture resource cost, native Linux results, or a repeated
same-logical-update Git history sequence. It does not establish the chosen
Zstandard level as final.
