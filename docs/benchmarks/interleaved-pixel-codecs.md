# Interleaved pixel codec results

This page records a five-pass compression comparison on ten frozen pixel samples. It compares zstd, xz, Brotli, and gzip across JSON and protobuf inputs. It measures compression work on one native macOS arm64 machine. It does not measure a complete Tuiscotti check or approval flow.

## The pixel samples matched before compression

The cohort has ten scenarios across four reference apps and four terminal grids:

| App | Grids |
| --- | --- |
| Holla | 80×24, 120×40 |
| Jackin | 80×24, 120×40 |
| Showcase | 80×24, 120×40, 160×50, 240×80 |
| Tablepro | 80×24, 120×40 |

Each scenario was captured once. The old JSON sample and new protobuf sample have different serialized bytes. Their readers decoded them into projections with equal fields, including metadata, PNG bytes, and decoded RGBA. The separate [same-snapshot pixel evidence archive](evidence/same-snapshot-pixel-10scenario-20261109.tar.zst) preserves the 409-file capture and comparison evidence. Its SHA-256 is `f5706b93f30f92bb62cf59c37f6d127b01c607d44ca921ac59d97260ac80cf11`. The core `SHA256SUMS` SHA-256 is `7a2a9ee25979b78c1dd7c8f1e156fe5d0fe06860273c89759a9e3bdf6e56b64f`.

The four reference executables are stored as Git blobs in TCB commit `60b5fb2762d91a3549bd4ac63b1d77d534ee53d2`, tree `ab75046874d68a3d954dc2b50f440243fc81c14e`. The frozen reference manifest says the apps were built from `fffe002e936d3621f22f9132144066a1374d5d74`. Those are separate revisions. The TCB commit stores the binaries. It does not identify their build source.

The old and new Tuiscotti helper programs use dependency commits `d8d54155947f6303a41dfd6079b53c0abc16f29a` and `92ef2d9ee082b35f68e066c5129fedbf07675a80`. The pixel archive contains both writer and reader executable sets, plus the helper source programs, manifests, lockfiles, build records, and hashes. It omits the full dependency trees. Fetch the pinned commits with `git fetch --no-tags https://github.com/tailrocks/tuiscotti d8d54155947f6303a41dfd6079b53c0abc16f29a 92ef2d9ee082b35f68e066c5129fedbf07675a80`.

The capture driver raised a `TypeError` while it assembled its final summary, after all ten per-scenario producer processes exited successfully. The archived per-scenario results and independent audit report ten captures and ten equal pairs. The error and driver output remain in the pixel archive.

## Streaming zstd preserved the frozen frame

The first one-shot zstd trial failed the exact writer-frame gate. Four of ten protobuf rows matched the existing frame, and six differed in each of five completed passes. The fail-closed aggregator exited with `RuntimeError: writer frame mismatch` and emitted no aggregate. The matrix archive retains this attempt under `failed-one-shot/`, including the audit, command transcript, five q22 protobuf output sets, and their payloads.

The passing run uses `ZSTD_compressStream2` with 65,536-byte input and output chunks and a pledged source size. It retains the original zstd parameters. The frame-identity check passes on all ten rows in all five passes, for 50 of 50 exact matches.

The full matrix covers 15 codec settings across JSON and protobuf, ten samples per treatment, and five interleaved passes. It records 1,500 measurements across repeats 4–8. The 15 settings are six zstd levels (22, 21, 19, 15, 9, and 3), three xz levels (9 in extreme mode, 6, and 3), three Brotli qualities (11, 9, and 5), and three gzip levels (9, 6, and 1).

## Results on the ten-sample cohort

The input set contains 15,263,428 canonical JSON bytes and 4,651,425 raw protobuf bytes. The current ten protobuf containers total 701,239 bytes, including ten 32-byte Tuiscotti headers.

| zstd setting | Compressed payload bytes, median | Encode wall, median | Encode CPU, median | Decode wall, median | Decode CPU, median | Largest reported process high-water (`ru_maxrss`) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Level 22, protobuf | 700,919 | 0.925 s | 0.909 s | 4.70 ms | 4.62 ms | 83,591,168 B |
| Level 22, JSON | 1,019,534 | 2.589 s | 2.493 s | 8.37 ms | 8.31 ms | 247,889,920 B |

The payload totals cover all ten samples. The protobuf total is 320 bytes smaller than the current containers because it excludes their ten 32-byte headers. It is not a checkout or Git-history size measurement.

Across the 30 codec and input-format combinations, the median compressed total ranges from 687,052 to 1,567,766 bytes. The median encode wall time ranges from 12.9 ms to 9.49 s, and median encode CPU ranges from 12.1 ms to 9.41 s. Median decode wall time ranges from 3.79 ms to 78.1 ms. Median decode CPU ranges from 3.71 ms to 74.1 ms. The median reported batch high-water (`ru_maxrss`) ranges from 32,210,944 to 244,105,216 bytes. The largest reported process high-water is 247,889,920 bytes. The worker also sampled RSS every 0.2 seconds under a 1,000,000 KiB guard. That polling is separate from `ru_maxrss`, and it is not a hard operating-system limit.

These ranges describe the medians across the 30 combinations, where each median summarizes five passes. The nearest-rank p95 over five observations is the maximum. It is not a stable estimate of tail latency. Per-file encode and decode timings exclude input reads, hashing, and output writes.

The following table lists the median compressed payload total for each of the 15 settings. Every value covers the ten-sample cohort and excludes container headers. The last column adds ten 32-byte headers. That total describes the existing container only for zstd level 22 protobuf. Other header-adjusted totals are hypothetical because Tuiscotti does not support those codec containers.

| Codec setting | JSON payload bytes | Protobuf payload bytes | Protobuf plus 10 headers |
| --- | ---: | ---: | ---: |
| zstd 22 | 1,019,534 | 700,919 | 701,239 |
| zstd 21 | 1,020,546 | 701,193 | 701,513 |
| zstd 19 | 1,021,413 | 701,473 | 701,793 |
| zstd 15 | 1,043,574 | 777,808 | 778,128 |
| zstd 9 | 1,049,482 | 788,145 | 788,465 |
| zstd 3 | 1,140,500 | 855,594 | 855,914 |
| xz 9, extreme | 953,064 | 687,052 | 687,372 |
| xz 6 | 954,864 | 690,036 | 690,356 |
| xz 3 | 997,072 | 712,032 | 712,352 |
| Brotli 11 | 969,556 | 694,536 | 694,856 |
| Brotli 9 | 1,033,674 | 742,621 | 742,941 |
| Brotli 5 | 1,055,592 | 761,266 | 761,586 |
| gzip 9 | 1,366,574 | 903,469 | 903,789 |
| gzip 6 | 1,376,528 | 908,971 | 909,291 |
| gzip 1 | 1,567,766 | 1,011,557 | 1,011,877 |

The smallest median payload totals came from xz level 9 in extreme mode: 687,052 bytes for protobuf and 953,064 bytes for JSON. The zstd level 22 timing table highlights the setting that also reproduces the frozen protobuf frame, not the smallest payload.

## Scope and limits

This is one native macOS arm64 capture cohort and one codec run. It covers ten pixel scenarios, not the state-only corpus. It does not measure capture, full end-to-end check time, approval, Git update history, or another platform. The run wrote no baselines and performed no acceptance.

The full matrix archive depends on the separate pixel archive linked above. The matrix archive omits duplicate copies of the captured projections and snapshots. Its omission ledger lists the exact paths and binary source requirements. The package records the measured zstd 1.5.7, Brotli 1.2.1, xz 5.8.4, and Python 3.15.2 library or executable hashes, plus their official source archive URLs and checksums. It does not include those runtime binaries, the compiler, or the full Python installation.

## Verify and replay the aggregate

The matrix evidence is a 30,550,663-byte zstd-compressed tar archive. Its SHA-256 is `31808a14ca18831064ada5cc5fcd3ca0ecc4f4a40b9d65c5964916409afd6830`. The archive uses tar hard links for identical files. Extraction restores every checksummed path and byte sequence.

Run these commands from the repository root. They use a fresh temporary directory and verify the two archives separately.

```sh
REPO_ROOT="$(pwd)"
SCRATCH="$(mktemp -d "${TMPDIR:-/tmp}/tuiscotti-pixel-codecs.XXXXXX")"
MATRIX_ARCHIVE="docs/benchmarks/evidence/interleaved-pixel-codecs-20261109.tar.zst"
PIXEL_ARCHIVE="docs/benchmarks/evidence/same-snapshot-pixel-10scenario-20261109.tar.zst"
printf '31808a14ca18831064ada5cc5fcd3ca0ecc4f4a40b9d65c5964916409afd6830  %s\n' "$MATRIX_ARCHIVE" | shasum -a 256 -c -
printf 'f5706b93f30f92bb62cf59c37f6d127b01c607d44ca921ac59d97260ac80cf11  %s\n' "$PIXEL_ARCHIVE" | shasum -a 256 -c -
zstd -t "$MATRIX_ARCHIVE"
zstd -dc "$MATRIX_ARCHIVE" -o "$SCRATCH/matrix.tar"
tar -xf "$SCRATCH/matrix.tar" -C "$SCRATCH"
cd "$SCRATCH/tuiscotti-pixel-streaming-matrix-20261109"
shasum -a 256 -c SHA256SUMS
python3 verify_bundle.py "$REPO_ROOT/$PIXEL_ARCHIVE"
python3 replay_aggregate.py "$REPO_ROOT/$PIXEL_ARCHIVE"
cd "$REPO_ROOT"
```

The pixel archive has SHA-256 `f5706b93f30f92bb62cf59c37f6d127b01c607d44ca921ac59d97260ac80cf11`. The replay helper verifies the matrix and pixel archives, rebases recorded temporary paths in a disposable copy, and runs the unchanged fail-closed aggregator. It exits with an error if the resulting aggregate hash differs from the recorded SHA-256: `ba247d5a07ecf79ce27320da7a11484cbd049f09b0d71aa9316f9445f74ca485`.

To verify the pixel archive on its own, extract it to a separate fresh directory and run its core and supplement verifiers:

```sh
REPO_ROOT="$(pwd)"
PIXEL_SCRATCH="$(mktemp -d "${TMPDIR:-/tmp}/tuiscotti-pixel-proof.XXXXXX")"
PIXEL_ARCHIVE="$REPO_ROOT/docs/benchmarks/evidence/same-snapshot-pixel-10scenario-20261109.tar.zst"
zstd -dc "$PIXEL_ARCHIVE" -o "$PIXEL_SCRATCH/pixel.tar"
tar -xf "$PIXEL_SCRATCH/pixel.tar" -C "$PIXEL_SCRATCH"
cd "$PIXEL_SCRATCH/tuiscotti-pixel-10scenario-20261109"
shasum -a 256 -c SHA256SUMS
python3 scripts/verify_evidence.py
cd ../publication
shasum -a 256 -c SHA256SUMS
cd "$REPO_ROOT"
```

The core verifier reports ten successful captures, ten equal pairs, and zero failures.
