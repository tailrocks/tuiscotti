# Provisional Zstandard level 22 selection

Status: **provisional; the full compression goal is not verified.** This records the user's requested working setting from measured absolute limits. It is not release approval, native Linux evidence, or a claim that level 22 is faster than other codecs.

## Encoder parameters

`Sample::to_bytes()` now selects level 22. The container encoder continues to set `WindowLog(25)` (32 MiB ceiling), `ContentSizeFlag(true)`, `ChecksumFlag(true)`, `DictIdFlag(false)`, and `NbWorkers(0)`, and pledges the exact source size. The lockfile pins `zstd-safe 8.2.0` and vendored `zstd-sys 2.1.2+zstd.1.5.7`; that vendored source defines `ZSTD_MAX_CLEVEL 22` and documents 22 as the maximum regular compression level. The public writer test asserts the dependency maximum is 22, compares public output with the explicit level 22 encoder, and decodes it back to the same state digest.

Level 22 compressed the benchmark's 20 candidate containers to 727,524 bytes, 546 bytes below level 21. Five release-mode runs reported median process wall time 2.227 s (range 2.095–3.337 s), median encode-region wall time 1.194 s (range 1.095–1.782 s), and maximum candidate-process RSS 216,645,632 bytes (206.6 MiB). All 20 decoded samples matched their logical state digests. The full five-pass matrix took 562.20 s; the slowest complete pass was 146.91 s. Against the absolute limits, these are comfortably below 1,073,741,824 bytes per process, 3,600 s per pass, and 259,200 s cumulative.

## Corpus and comparisons

The frozen corpus contains ten paired captures from Showcase, TablePro, Holla, and Jackin at 80×24, 120×40, 160×50, and 240×80, for state and state-plus-pixels gates. Old/new capture projections, contracts, PNG hashes, and RGBA hashes matched. The 20 canonical JSON files total 27,954,874 bytes; their corresponding raw Protobuf messages total 7,379,815 bytes. The runs verified all codec outputs by decompression and byte comparison, or by decoding candidate containers and comparing state digests.

The external codec rows below all compress the same 7,379,815 bytes of raw Protobuf. Their outputs omit Tuiscotti's per-sample container framing, so compare those sizes as raw codec results, separately from the 727,524-byte candidate-container result.

| Codec and setting | Output bytes | Median encode wall | Maximum process RSS | Result |
|---|---:|---:|---:|---|
| Zstandard CLI 1.5.7, level 3 | 886,009 | 44.5 ms | 11.0 MiB | All five verified |
| Zstandard CLI 1.5.7, level 22 | 726,461 | 1.453 s | 55.4 MiB | All five verified |
| XZ 5.8.4, LZMA2 preset 6 | 718,560 | 545.6 ms | 33.7 MiB | All five verified |
| Brotli 1.2.1, quality 11 | 719,979 | 4.150 s | 60.0 MiB | All five verified |
| Apple gzip 487.40.2, level 9 | 944,707 | 139.5 ms | 2.5 MiB | All five verified |

The comparison tools also tested XZ presets 0, 3, 6, and 9 with a 32 MiB dictionary ceiling; Brotli qualities 1, 5, 9, and 11; gzip levels 1, 6, and 9; and Zstandard CLI levels 1, 3, 9, 12, 15, 18, 19, 20, 21, and 22. Host contention makes all relative timing comparisons unsuitable for ranking. The output sizes are measurements of these frozen inputs only.

## Measurement limits and caveats

The benchmark plan originally made contended runs ineligible for timing-based selection. The user requested the shortest timeline and no native Linux host was available, so five interleaved release-mode passes were run with `--allow-contended`. A post-run amendment permits a provisional decision only when every measured process/pass is safely within the absolute caps above. This amendment was recorded after the runs. It does not permit a relative speed ranking.

The host was macOS 27.2, Apple M5 Max, arm64, 18 logical CPUs, with Rust 1.99.1, MBX 1.21.1, Nextest 0.9.151, zstd 1.5.7, XZ 5.8.4, Brotli 1.2.1, Apple gzip 487.40.2, Git 2.54.0, and Python 3.15.2. Across 520 before/after host observations, 219 observed unrelated build activity; one-minute load ranged from 40.75 to 86.97 on 18 logical CPUs. The machine was not idle. Timing results are therefore not speed evidence.

Measurements were made against a frozen earlier trial source and optimized test binary, not this final trial tree: source snapshot SHA-256 `c159934ac6b9c878d2bdb248340aadf69144eb7774a34fce2a931e96a2bcb4de` (673 verified manifest entries); benchmark candidate tree SHA-256 `279db9c434436aaa2a4b36976a389a92ca7be3ec0dbf06ff9d95388ff8a8e3dd`; benchmark binary SHA-256 `217c1c02dee9cdcd45a9a3e069f2a2b8f9c49e40bdf319b155b2a5083f194091`. The final-source follow-up below repeats the public writer measurement, but not the complete codec matrix. Native Linux behavior and supported-platform evidence are still required; the user confirmed there is no native Linux host available in this environment.

## Evidence and reproduction

For relocatable replay instructions and the final-source public-writer follow-up, see [Reproducing the compression evidence](REPRODUCING.md). The replay archive and final-source records are local untracked workspace evidence under `.audit/benchmark-evidence/`, not shipped repository files.

Compact evidence is preserved in [`evidence/`](evidence/): the freeze, run and aggregate summaries, post-run analysis, producer manifests, and pairing report. The aggregate summaries and manifest do not include the generated capture bytes. Executable Python/Rust research harnesses remain in the external bundle; the repository policy does not permit copying those first-party executable sources into these documentation directories. Their hashes are recorded in the freeze manifest. SHA-256 references:

- [`benchmark-freeze.json`](evidence/benchmark-freeze.json): `1dbb886b45686d1a4d7d5d88b55351f45b3a79ff3b573ead409c94ee49bf3d5e`
- [`HANDOFF.md`](evidence/HANDOFF.md): `7528c234048e0802367c5d85d26e5b8ac52a8ea12a9954dd9b8985721aa55482`
- [`run-summary.json`](evidence/results/macos-release-contended-20261105/run-summary.json): `f3b136e448bac198e9a33819e9ee6c00fb4550c19b3eb30b4902766ade7e34ee`
- [`summary.json`](evidence/results/macos-release-contended-20261105/summary.json): `07a2a4b9f6c891290e3cf118dba7c8907d47834cbf7ac16acc565d541e5aaa17`
- [`postrun-analysis.json`](evidence/results/macos-release-contended-20261105/postrun-analysis.json): `bbb91b000d7ee4d279bd601ec47092d93b2fb7d5b678724a250b675b770fa37f`
- external runner `tools/run-matrix.py`: `70aeca6efd4953f428cd12eda1806d646cc54defaf8e270fb8b2f077f0468e9b`
- external aggregator `tools/summarize-results.py`: `edbf7ef6bb57d6462b5d89d55444758d7a7762836bb78541c33f14b329b26563`
- external analyzer `results/macos-release-contended-20261105/analyze.py`: `042140e475c93f2a90dfc21ca98fed4a3e5ffe9125100aaa184e728036928233`

The handoff file's opening status predates the appended final matrix results and is stale; the dated “Latest macOS codec matrix” section and the preserved summaries are the relevant evidence. The run summary itself says `timing_selection_eligible: false` and “No codec winner may be selected”; this record uses only the later amendment's absolute-cap allowance for a provisional implementation setting.

The exact matrix and aggregation commands, run in the frozen benchmark bundle, were:

```sh
cd /private/tmp/tuiscotti-benchmark-owned
python3 tools/run-matrix.py --passes 5 --allow-contended --out results/macos-release-contended-20261105
python3 tools/summarize-results.py results/macos-release-contended-20261105
python3 results/macos-release-contended-20261105/analyze.py
```

The preserved producer manifests are under `evidence/captures/`; the
original producer programs remain in the external bundle. They list the
ten scenarios and were used with the downstream application/reference executables from
revision `60b5fb2762d91a3549bd4ac63b1d77d534ee53d2`. The old Tuiscotti
producer dependency is pinned at `d8d54155947f6303a41dfd6079b53c0abc16f29a`;
the new producer was built against benchmark source revision
`d31900993b533ad4758848c261b5f4c5b2eb1cee`. The paired corpus manifest
SHA-256 is `f07a3d2c643c5633b81221e1bf407eded40b847808656829f9e42c33ecec9924`;
the old/new pairing result SHA-256 is
`b3879a8d8d051e23ed68bce3aa5442f7f6001d37befd51214f5fee9605bbd08f`.

Full independent reproduction is **not yet achieved from this repository**.
The executable research harness, generated captures (about 225 MiB), raw
per-task results (about 402 MiB), reference application executables, and exact
frozen candidate checkout/test hook are not checked in. Per-file corpus hashes
and required layout are in the preserved freeze manifest; tool hashes are
there too. Re-running requires restoring those frozen inputs under the paths
expected by `run-matrix.py`; rerunning capture additionally requires rebuilding
the pinned downstream application binaries and the old/new producers. Do not
interpret the copied summaries as a self-contained corpus.

To verify the current writer locally after the level change:

```sh
cargo nextest run -p tuiscotti --lib --locked --offline public_sample_encoder_uses_level_22_and_decodes
cargo nextest run --workspace --locked --offline
```

## Exact-head public-writer follow-up

An independently reviewed release-mode run used a disposable clean worktree
at exact reviewed source commit `9e8eab42e072936824186bbaa17919181410a6e9`,
with only a test-only harness overlay. It exercised the public
`Sample::to_bytes()` / `Sample::from_bytes()` path for 20 real captures (10
state and 10 state-and-pixels), including reconstruction, validation,
pixel re-rendering, round-trip equality, digest and PNG/RGBA checks, and output
writes. Five independent processes each completed all 20 captures; all 100
outputs matched the preserved level-22 outputs byte for byte. No approval or
baseline write ran.

Process wall time, measured around the whole test binary and including output
writes, had a 3.99 s median and 3.88–4.38 s range. Maximum RSS was
350,978,048 bytes. Each run wrote 727,524 bytes. These measurements passed the
1 GiB/process, 3,600 s/pass, and 259,200 s cumulative caps. Host load was
elevated (one-minute averages 60.74–75.63 on 18 logical CPUs), so this is
resource-feasibility and output-equivalence evidence, not a timing comparison.
The harness JSON field `end_to_end_ns_includes_parse_validate_encode_decode_digest_write`
is mislabeled: its timer stops before `fs::write`, and therefore excludes
output writes. Use the process-level timing for the end-to-end cap comparison.

The source record, raw run data, overlay, inputs, and output hashes are local
untracked evidence at `.audit/benchmark-evidence/public-writer-9e8eab42/` in
the original workspace; they are not present in a fresh clone or this
documentation-only checkout. Reproduction requires the preserved capture
archive and matching prior output reference, the exact commit above, the
test-only overlay, macOS arm64, Rust 1.99.1, and the recorded release test
binary/build procedure. The evidence README documents the commands and hashes.

## Earlier final-source public-path follow-up

A later macOS arm64 run exercised the public writer from the final reviewed
source, including sample reconstruction, validation, pixel re-rendering,
encoding, decoding, sample equality, digest checks, and output writes.
All 20 captured samples passed in each of five runs, and all 100 encoded files
were byte-identical to their earlier level-22 counterparts. Process wall time
had a median of 3.16 s and a range of 3.12–4.63 s; maximum RSS was
341,000,192 bytes. The host remained contended, so these results establish
only that this workload stayed within the stated absolute caps.

Five CLI checks passed against one copied, already-approved pixel sample.
One render and one report over an empty artifact directory also passed. This
is not an accept benchmark, a report benchmark over realistic evidence, or a
CLI check over the full capture corpus. The raw records and test-only harness
are preserved locally under `.audit/benchmark-evidence/final-source/`; the
harness archive is not a production sample reader. Native Linux behavior and independent regeneration of the captures remain
unverified.

## Synthetic Git history follow-up

The final public writer reproduced all 25 encoded inputs in the earlier
synthetic history byte for byte. Two new eight-commit history builds also
matched the archived checkout sizes and pack hashes: JSON occupied
26,678,979 checkout bytes and 1,251,002 packed bytes; Protobuf/Zstandard 22
occupied 726,450 checkout bytes and 808,289 packed bytes. This is a storage
comparison for that synthetic history, not a timing or platform claim.
The replay inputs, source, scripts, results, and checksums are preserved
locally in `.audit/benchmark-evidence/tuiscotti-final-git-evidence-20261106.tar.zst`
(SHA-256 `e35f7749d9583feb1a85fdc96e2e982ea516885ed5390a53a7cd00ed63d163ed`).
The archive is local untracked evidence; it is not included in a fresh clone.

A separate macOS arm64 CLI follow-up generated a real `missing` check record
and four evidence files in an empty disposable baseline root, then produced a
one-entry report five times. Every report and evidence link resolved and the
HTML bytes matched. The reported 0.01 s wall times are below useful
resolution on the contended host; the run is evidence of this workflow, not a
speed comparison. Resource caps were recorded but not enforced in that run.
The local untracked evidence archive is
`.audit/benchmark-evidence/tuiscotti-cli-report-evidence-20261106-reviewfix-v3.tar.gz`
(SHA-256 `f32b07f7f93fdc9e6095b8ffaa57af4d9a2e3ce5d84cdaeeac57ed5000d0c247`).
It contains replay checks and raw records. It did not invoke approval or inject
CLI/report failures, and it does not establish Linux behavior.

## Real downstream-history migration measurement

A separate storage experiment used 105 unapproved candidate snapshots against
a real downstream JSON-history prefix. It replaced 104 matching `.tui.json`
files with the captured `.tui.pb` bytes, added one new PB snapshot, and retained
five unmatched JSON files. It did not call `accept`, modify an active baseline,
or rewrite full historical revisions. In the measured checkout, the functional
baseline changed from 109 JSON files (125,135,015 bytes) to 105 PB files
(280,900 bytes) plus five retained JSON files (5,635,106 bytes), 5,916,006
bytes total.

With the recorded Git 2.54.0 repack settings, the real-history pack grew by
292,207 bytes (30,929,432 to 31,221,639 bytes); the one-commit thin fetch pack
was 286,339 bytes. A fresh PB-only history containing the same 105 candidate
inputs produced a 286,025-byte pack. These are Git storage observations, not
codec speed results. The candidate captures are unapproved and no equivalence
between these PB bytes and older JSON revisions is claimed. The dataset is a
single current snapshot, not a multi-revision PB update history; complete
historical migration evidence requires exact captures from each historical
application revision. The raw results, inputs, scratch repositories, and
checksums are local untracked evidence in the original workspace under
`.audit/real-history-migration-benchmark-20261106/`; they are not included in
a fresh clone. Reproduction depends on that evidence, source prefix
`60b5fb2762d91a3549bd4ac63b1d77d534ee53d2`, the exact captured inputs, and the
recorded Git version and repack flags.
