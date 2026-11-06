# Codec benchmark handoff

> **Status:** Historical benchmark handoff. Its opening status and frozen candidate details describe the pre-run stage; later results are appended below. For current implementation settings and the benchmark interpretation, see [the provisional level 22 record](../zstd-level-22-provisional.md), [the binary sample persistence decision](../../DECISIONS.md#18-binary-sample-persistence-zstandard-level-22-provisional), and the [current snapshot contract](../../SNAPSHOTS.md). The setting remains provisional; native Linux remains **not verified**.

Status as of 2026-11-05 21:10 +07:00: **stable gated candidate inputs and Git storage evidence are prepared; the timed codec matrix has not run; no level is selected.** The input/method freeze is at `benchmark-freeze.json`. All results below are provisional for the immutable trial snapshot listed below; the parent reports that the eventual branch may still change CI gating/docs and the paired release version. Rebuild and recheck the final source before treating any future codec result as final.

## Frozen candidate and captures

The immutable source snapshot at `/private/tmp/tuiscotti-trial-gates-20261105` passed `SHA256SUMS`: 672/672 entries. `SHA256SUMS` itself has SHA-256 `a6943e65c825de6067ad0ef37d75274e37cf5f9677300f3798b32c72c1112853`. The disposable benchmark tree was based on this snapshot plus test-only hooks. Its source-tree hash is `722a77bb1ebfcd963fe854f38b4c7df0693c22b610967392f7e346ec5448cae7` (674 files, 31,829,062 bytes); candidate lib-test binary SHA-256 is `e24d153dd79713f331686151252be33146f4ff088bee7f8878cb537ff8d753d1` (36,238,360 bytes). These hashes are recorded in the freeze.

The native capture producer used four checked-in macOS/aarch64 downstream reference executables from detached downstream revision `60b5fb2762d91a3549bd4ac63b1d77d534ee53d2`. Ten PTY scenarios cover Showcase, TablePro, Holla, and Jackin at 80×24, 120×40, 160×50, and 240×80. Each has state and state-plus-pixels samples. The old/new capture pairer found all ten matched, including equal logical snapshot projections, render contracts, PNG hashes, and RGBA hashes. Manifest SHA-256: `f07a3d2c643c5633b81221e1bf407eded40b847808656829f9e42c33ecec9924`; pairing SHA-256: `b3879a8d8d051e23ed68bce3aa5442f7f6001d37befd51214f5fee9605bbd08f`.

The corpus has 20 canonical JSON files totaling 27,954,874 bytes. All 20 are byte-identical to compact canonical JSON reserialization. The gated candidate MBX raw-Protobuf export passed, producing 20 files totaling 7,379,815 bytes. The candidate test reconstructs Samples from the exact snapshot/contract/image JSON fields; the raw-only validation path also calls the public pixel builder and checks exact PNG/RGBA and render-contract equality. Raw source/JSON pairing hashes are in `captures/codec-source/codec-source-records.json`.

The prior five-scenario rehearsal is retained under `captures/five-scenario-rehearsal/`. No downstream baselines were used as codec inputs, converted, approved, or written.

## Git history evidence (size only)

`history/history-plan.json` defines eight commits: a nine-pair base, sparse-cell/style/palette/metadata edits, addition of the tenth high-resolution pair, one pixel change with recomputed PNG/RGBA hashes, and a deletion. The deletion is a file operation and therefore has no codec artifact to encode. Candidate Protobuf/Zstd files for the seven changed-file stages passed round-trip and state-digest checks at all ten levels: 250 verified artifacts total. The history exporter and builder are reproducible scripts under `tools/`.

The JSON history produced identical packed results at all ten settings. It has 8 commits and 19 final checkout files. Checkout bytes are separate from packed Git storage. The table reports `checkout / object-dir / pack` bytes:

| Zstd level | Protobuf checkout | Object directory | Pack |
|---:|---:|---:|---:|
| JSON | 26,678,979 | 1,254,356 | 1,251,002 |
| 1 | 1,153,739 | 1,230,523 | 1,227,169 |
| 3 | 877,149 | 973,604 | 970,250 |
| 9 | 810,028 | 903,379 | 900,025 |
| 12 | 806,747 | 898,626 | 895,272 |
| 15 | 798,251 | 888,544 | 885,190 |
| 18 | 731,758 | 814,971 | 811,617 |
| 19 | 727,602 | 812,882 | 809,528 |
| 20 | 727,452 | 812,715 | 809,361 |
| 21 | 727,088 | 812,339 | 808,985 |
| 22 | 726,450 | 811,643 | 808,289 |

These are storage-size results only, generated against the trial candidate and the eight-commit synthetic history from fresh captures. They do not select a level. Raw records and pack hashes are under `history/results-gated/`; summary is `history/results-gated/history-matrix-summary.json`. The freeze records that summary’s SHA-256.

## Prepared timed harness

- `tools/pair-captures.py`: compare old/new capture projections and domain/pixel/render contracts.
- `tools/codec-task.py`: serial JSON/raw-Protobuf compression and byte-exact decompression checks for Zstd CLI, XZ/LZMA2, Brotli, and gzip.
- `tools/run-matrix.py`: 52 tasks per pass, five deterministic interleaved passes. It records shuffled task order before running, invokes the hash-frozen candidate test binary directly under `/usr/bin/time -l`, and checks external child RSS too. Preflight refuses a 1-minute load average above the logical CPU count, active Cargo/rustc/Nextest work, or another benchmark runner. It also verifies the candidate source/binary, all 20 JSON inputs, and all 20 raw Protobuf inputs against the freeze.
- Candidate Zstd levels: 1, 3, 9, 12, 15, 18, 19, 20, 21, 22, with 32 MiB window. Other settings: Zstd CLI same levels/window; XZ presets 0/3/6/9 with 32 MiB dictionary; Brotli qualities 1/5/9/11; gzip 1/6/9.
- Caps: 1 GiB peak RSS per measured process, 60 minutes per complete pass, 72 hours cumulative, serial execution.
- `tools/summarize-results.py` aggregates the raw five-pass output without choosing a setting.

Installed versions: MBX 1.21.1, Rust 1.99.1, Nextest 0.9.151, zstd 1.5.7, xz 5.8.4, Brotli 1.2.1, Apple gzip 487.40.2, Git 2.54.0, Python 3.15.2. Host: macOS 27.2, Apple M5 Max, arm64, 18 logical CPUs, 137,438,953,472 bytes RAM. Native evidence is only `macos-aarch64`; emulated Linux containers are not native Linux evidence.

## Latest macOS codec matrix (2026-11-05)

The five-pass interleaved matrix completed in the isolated benchmark copy using the frozen optimized test binary. This is a macOS arm64 codec result only; it is **not eligible for selecting a setting** because host load was 40.75–86.97 on 18 logical CPUs, and 219 of 520 before/after host snapshots saw active unrelated build work. No Linux x86_64 native host was available.

Inputs were 10 freshly paired state/pixel scenarios, 20 canonical JSON messages totaling 27,954,874 bytes, and matching 20 raw Protobuf messages totaling 7,379,815 bytes. Candidate source snapshot manifest SHA-256: `c159934ac6b9c878d2bdb248340aadf69144eb7774a34fce2a931e96a2bcb4de` (673/673 verified). Candidate tree SHA-256: `279db9c434436aaa2a4b36976a389a92ca7be3ec0dbf06ff9d95388ff8a8e3dd`; optimized test binary SHA-256: `217c1c02dee9cdcd45a9a3e069f2a2b8f9c49e40bdf319b155b2a5083f194091`.

All 260 measured tasks completed and verified: external codecs decompressed every file byte-for-byte; the candidate decoded each Protobuf container and matched the logical state digest. Each of five passes ran all 52 tasks serially. Pass times were 123.93, 91.37, 146.91, 110.84, and 89.14 seconds (median 110.84; range 89.14–146.91); total was 562.20 seconds. Caps were 1,073,741,824-byte process RSS, 3,600 seconds per pass, and 259,200 seconds total. All passed: largest process RSS across codecs was 226,213,888 bytes (216 MiB), every pass was below 147 seconds, and total was 9m22s. The large timing spread and host contention invalidate cross-codec wall/CPU winner claims.

### Candidate Protobuf/Zstandard levels

Each row summarizes five runs on the same 20-message corpus. Process wall and user CPU include the optimized test harness sample reconstruction, Protobuf encode, Zstandard encode/decode, and logical-digest checks. “Encode wall” is the separately measured sum of Zstandard encode calls over all 20 messages. Sizes are compressed candidate container bytes for the 20 messages. Git history figures are separate: an eight-commit, 19-final-file workload with documented edits and deletion.

| Level | Candidate output bytes | Process wall median [range] s | Process user CPU median [range] s | Encode wall median [range] s | Max RSS MiB | History checkout bytes | Git object dir bytes | Git pack bytes |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 1 | 1,187,635 | 1.254 [1.167, 4.588] | 1.070 [1.040, 1.250] | 0.168 [0.153, 0.470] | 175.5 | 1,153,739 | 1,230,523 | 1,227,169 |
| 3 | 890,854 | 1.292 [1.207, 2.703] | 1.080 [1.010, 1.240] | 0.170 [0.154, 0.307] | 178.6 | 877,149 | 973,604 | 970,250 |
| 9 | 819,288 | 1.253 [1.218, 1.894] | 1.110 [1.080, 1.150] | 0.188 [0.176, 0.312] | 179.2 | 810,028 | 903,379 | 900,025 |
| 12 | 815,850 | 1.298 [1.255, 1.495] | 1.150 [1.120, 1.170] | 0.229 [0.224, 0.278] | 196.9 | 806,747 | 898,626 | 895,272 |
| 15 | 807,201 | 1.378 [1.298, 2.163] | 1.220 [1.150, 1.310] | 0.306 [0.282, 0.514] | 215.7 | 798,251 | 888,544 | 885,190 |
| 18 | 732,760 | 1.804 [1.543, 6.624] | 1.440 [1.410, 1.740] | 0.619 [0.532, 2.649] | 211.6 | 731,758 | 814,971 | 811,617 |
| 19 | 728,596 | 2.115 [1.812, 3.548] | 1.670 [1.620, 1.810] | 0.941 [0.740, 1.528] | 209.2 | 727,602 | 812,882 | 809,528 |
| 20 | 728,434 | 1.913 [1.816, 3.623] | 1.710 [1.640, 1.830] | 0.831 [0.775, 1.529] | 207.7 | 727,452 | 812,715 | 809,361 |
| 21 | 728,070 | 2.153 [1.970, 2.591] | 1.840 [1.760, 1.920] | 1.023 [0.892, 1.271] | 210.3 | 727,088 | 812,339 | 808,985 |
| 22 | 727,524 | 2.227 [2.095, 3.337] | 2.040 [1.930, 2.250] | 1.194 [1.095, 1.782] | 206.6 | 726,450 | 811,643 | 808,289 |

JSON history baseline: 26,678,979 checkout bytes; 1,254,356 object-directory bytes; 1,251,002 pack bytes. Level 22 pack saves 35.39% against that JSON history; L22 saves 696 pack bytes (0.086%) versus L21. The 20 current initial-capture encoded outputs exactly match the prior history base/addition outputs at all ten levels, and the current raw Protobuf corpus is identical 20/20. History edit-stage sizes therefore remain useful provisional storage evidence, but were not recomputed from the final snapshot.

Level 22 is the strongest available regular Zstandard compression setting in this vendored Zstandard 1.5.7 build: `zstd-sys` documents `ZSTD_maxCLevel()` as 22 and its source defines `ZSTD_MAX_CLEVEL 22`. Among candidate levels, L22 produced the smallest measured container output (727,524 bytes), 546 bytes below L21. It also had a 1.194 s median encode-region wall time over 20 messages and 216,645,632-byte maximum process RSS. These observations meet the frozen absolute time/RSS limits by a wide margin. The L22-vs-L21 timing difference is not defensible under this host load, so no compression level is selected from this run.

### Level 22 five-run details

- Candidate process wall: median 2.227 s, range 2.095–3.337 s.
- Candidate process CPU: user median 2.040 s (1.930–2.250), system median 0.070 s (0.040–0.080).
- Candidate encode-region wall: median 1.194 s (1.095–1.782); decode-region wall median 0.258 s (0.239–0.414).
- Candidate maximum RSS across these runs: 216,645,632 bytes (206.6 MiB).
- Output: 727,524 bytes across 20 messages; all 20 decoded logical digests matched.

### Reproducible artifacts

- Frozen inputs/method: `benchmark-freeze.json`.
- Raw pass tasks, order, host observations, stdout, `/usr/bin/time -l` logs, and per-task codec records: `results/macos-release-contended-20261105/`.
- Derived CPU parsing and complete per-setting summary: `results/macos-release-contended-20261105/analyze.py` and `postrun-analysis.json` (script SHA-256 is recorded in the JSON). This parser extracts macOS `real`, `user`, and `sys` tokens from the same line in each raw time log.
- Matrix summary: `results/macos-release-contended-20261105/summary.json`.
- No baseline conversion, acceptance, or persistent converter was run.
