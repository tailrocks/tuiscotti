# Five-pass interleaved state-codec evidence

Status: **measured state-only evidence; the level-22 decision remains provisional.** This record summarizes the reviewed aggregate for the warm, sequential, arm64 treatment-level interleave (passes 4–8). It does not complete the broader evidence needed to make level 22 final.

## Corpus and method

The aggregate covers 1,872 ordered occurrences per treatment and 1,868 unique test IDs. One ID occurs five times, contributing four extra occurrences; the separate one-per-ID summaries are available in the aggregate. Input representations were canonical JSON (1,919,357,634 raw bytes) and raw Protobuf messages (410,834,711 raw bytes). JSON and Protobuf were compressed independently; their byte totals must not be added as though a single representation were used.

Five warm, sequential passes interleaved treatments in predeclared order. They ran on arm64 and compared Brotli, gzip, XZ, and Zstandard settings. The aggregate records codec versions observed consistently across rows and passes, but the provenance's pilot-version fields are null, so the exact version identities are not independently pinned by this aggregation. Version labels below are therefore the versions recorded in the data, not a claim of independently verified tool identity.

The aggregate is content-addressed in the archive. Its `BUNDLE-MANIFEST.json` entry names it `aggregate/real-aggregate-01/aggregate.json` and preserves SHA-256 `2cabebeb545c6038b5fcab266d41e150a679f4004b5039ee5e00a2c5c5894a1f`; that path exists after restoring the manifest with the included `archive-tools/restore_bundle.py`. The [slim v3 evidence archive](../../benchmark-evidence/tuiscotti-compression-evidence-slim-v3-20261109.tar.zst) is 70,891,144 bytes (SHA-256 `f1c1f947cf60228cba5592f92ac67aec626cfebad823829089f48962805d05b6`). Its logical archive size is 70,891,144 bytes. In an independent disposable bare Git database with Git 2.54.0 (Apple Git-157), storing exactly this archive as one blob and repacking without deltas produced a 70,898,173-byte `.pack`; `.pack` + `.idx` + `.rev` occupied 70,899,329 bytes total (8,185 bytes above the archive). This is a local one-blob storage measurement, not a GitHub server estimate, sample-storage cost, or codec improvement. The retained [summary](../../benchmark-evidence/tuiscotti-v3-git-cost-20261109/GIT-COST-SUMMARY.json) (SHA-256 `72c2ee1f33e7fb89fe63b5f1f695dfbfd396f7bfab2d8f06fee4f0af05097463`) and [commands](../../benchmark-evidence/tuiscotti-v3-git-cost-20261109/COMMANDS.md) (SHA-256 `14573699140351cd33bc02387194cab460317e0f90788a7d1a7a100f1ae6306f`) preserve this measurement and its scope.

Verify and extract it from the repository root:

```sh
shasum -a 256 benchmark-evidence/tuiscotti-compression-evidence-slim-v3-20261109.tar.zst
evidence_dir="$(mktemp -d "${TMPDIR:-/tmp}/tuiscotti-codecs.XXXXXX")"
zstd -dc benchmark-evidence/tuiscotti-compression-evidence-slim-v3-20261109.tar.zst | \
  tar -xf - -C "$evidence_dir"
```

Then follow the extracted `README.md` and `ARCHIVE-COMMANDS.md`. `archive-tools/restore_bundle.py` restores the manifest to two distinct output subtrees and verifies each object hash. The original root prefixes recorded in `BUNDLE-MANIFEST.json` are `/private/tmp/tuiscotti-interleaved-aggregate-design-20261109` and `/Users/donbeave/Projects/tailrocks/tuiscotti-benchmark-closure-20261108/paired-oracle-20261108/fresh-capture-prototype/coverage-run`; they are provenance, not durable paths. For example, after extraction, set a fresh destination and map those original prefixes to it:

```sh
restore_dir="$(mktemp -d "${TMPDIR:-/tmp}/tuiscotti-codecs-restore.XXXXXX")"
python3 "$evidence_dir/archive-tools/restore_bundle.py" \
  --manifest "$evidence_dir/BUNDLE-MANIFEST.json" \
  --objects "$evidence_dir/objects" \
  --root-map "/private/tmp/tuiscotti-interleaved-aggregate-design-20261109=$restore_dir/aggregate-source" \
  --root-map "/Users/donbeave/Projects/tailrocks/tuiscotti-benchmark-closure-20261108/paired-oracle-20261108/fresh-capture-prototype/coverage-run=$restore_dir/coverage-run" \
  --root-map "/private/tmp/build_tuiscotti_archive.py=$restore_dir/archive-tools/build_tuiscotti_archive.py"
```

This restores all listed files to two root-mapped subtrees plus the standalone builder file, and can require substantial disk space; choose a destination accordingly. After restoration, the aggregate is at `$restore_dir/aggregate-source/real-aggregate-01/aggregate.json`. `archive-tools/reconstruct-worker-jsonl.py` recreates the 4,500 omitted worker logs byte-for-byte and verifies output sizes and hashes. The slim bundle omits 280,800 derived compressed payload files (their expected paths, sizes, and hashes are preserved), plus those reconstructible worker logs and 273 old JSON / 273 snapshot-transport files from failed, unpaired `p03`. The failed `p03` files cannot be reconstructed and are retained only in the full or prior compact local archives. The slim archive preserves selected inputs, recorded measurements, and payload hashes, but does not contain the derived payload bytes themselves for direct inspection. Some archived source and command records may contain original absolute paths.

## Compressed payload totals

These are aggregate compressed payload bytes across the 1,872 occurrences, excluding the Tuiscotti container header. JSON and Protobuf results are separate. Zstandard 22 is the smallest observed Zstandard output in each representation. XZ preset 9 uses extreme mode; the aggregate records an estimated encoder memory of 705,794,571 bytes for that setting. These results are for this corpus only.

| Codec (recorded version) | Setting | JSON bytes | Protobuf bytes |
|---|---:|---:|---:|
| Brotli (1.2.1) | 5 | 6,573,939 | 5,475,843 |
| Brotli (1.2.1) | 9 | 6,226,890 | 5,263,864 |
| Brotli (1.2.1) | 11 | 7,859,890 | 4,781,100 |
| gzip (zlib 1.2.12) | 1 | 29,251,097 | 13,538,701 |
| gzip (zlib 1.2.12) | 6 | 16,436,124 | 8,495,639 |
| gzip (zlib 1.2.12) | 9 | 14,487,573 | 7,583,792 |
| XZ (5.8.4) | preset 3 | 6,736,008 | 5,454,040 |
| XZ (5.8.4) | preset 6 | 7,162,088 | 5,395,176 |
| XZ (5.8.4) | preset 9 (extreme) | 6,505,300 | 4,947,916 |
| Zstandard (1.5.7) | 3 | 8,680,246 | 6,844,418 |
| Zstandard (1.5.7) | 9 | 7,334,069 | 5,733,649 |
| Zstandard (1.5.7) | 15 | 6,904,535 | 5,246,727 |
| Zstandard (1.5.7) | 19 | 6,736,633 | 5,006,339 |
| Zstandard (1.5.7) | 21 | 6,548,754 | 4,974,298 |
| Zstandard (1.5.7) | 22 | 6,405,022 | 4,909,745 |

A Tuiscotti Protobuf container adds 32 bytes per sample. The Zstandard 22 Protobuf rows match the original writer frames, so these are actual writer-container totals: 4,909,745 payload bytes plus 59,904 header bytes across 1,872 occurrences, totaling 4,969,649 bytes. The separate one-per-ID total is 4,902,187 payload bytes plus 59,776 header bytes across 1,868 IDs, totaling 4,961,963 bytes. Neither figure is a Git checkout or history size. The other codec rows are raw codec comparisons and do not imply that those codecs are supported container formats.

## Timing and observed memory

For Zstandard 22, per-treatment medians across five passes were:

| Input | Encode wall median (range) | Decode wall median (range) | Treatment wall median (range) | Maximum observed batch process RSS |
|---|---:|---:|---:|---:|
| JSON | 340.970 s (277.074–458.629 s) | 0.422 s (0.359–0.464 s) | 353.605 s (287.187–471.949 s) | 177,897,472 bytes |
| Protobuf | 87.286 s (68.888–120.061 s) | 0.242 s (0.207–0.254 s) | 96.868 s (77.344–133.639 s) | 72,597,504 bytes |

The aggregate includes medians, minimums, maximums, nearest-rank p95, and median absolute deviations for five observations. With only five observations, p95 is the maximum and is not a stable tail estimate. Encode/decode timings sum per-file codec calls and exclude reads, hashing, and writes. Treatment wall includes orchestration and output writes. RSS is a maximum process high-water observation across batches, not a hard cap, per-codec allocation measure, or guarantee that memory stayed below a threshold; RSS polling is sampled.

## Scope and decision status

This is state-only evidence. It contains no image or pixel workload and does not measure end-to-end check, accept, capture, render, or report behavior. Execution was restricted to arm64; Linux results, non-arm64 behavior, and cross-platform determinism are unverified. This evidence also does not establish Git-history/check-out size. The aggregate also excludes fixed-order passes 1–3 and uses warm runs; it is not cold-cache evidence.

Zstandard level 22 remains a provisional choice pending the missing broader requirements, including representative pixel-bearing workloads and end-to-end and history measurements. Linux behavior and cross-platform determinism remain unverified under the arm64-only execution restriction. The evidence here supports only the stated state-corpus observations and must not be read as closing those requirements.
