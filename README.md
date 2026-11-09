# Ten-scenario same-snapshot pixel control

This evidence records ten fresh live native macOS arm64 captures selected by `input/scenarios-10.json` (SHA-256 `f07a3d2c643c5633b81221e1bf407eded40b847808656829f9e42c33ecec9924`). For each capture, the pinned old and new public Tuiscotti `Sample` writers and readers consumed that one exact TerminalSnapshot JSON. All ten captures and all ten comparisons succeeded. The independent audit verifies complete typed projection equality per scenario, including metadata, render contract, PNG bytes, and decoded RGBA bytes. No baselines were accepted or written.

Sources and artifacts are pinned in `provenance/`:

- TCB commit `60b5fb2762d91a3549bd4ac63b1d77d534ee53d2`, tree `ab75046874d68a3d954dc2b50f440243fc81c14e`.
- Old Tuiscotti commit `d8d54155947f6303a41dfd6079b53c0abc16f29a`; new Tuiscotti commit `92ef2d9ee082b35f68e066c5129fedbf07675a80`.
- Exact scenario source manifest, arm64 TCB reference executable SHA-256 and Git blob identities, old/new writer/reader executable identities, source hashes, prior build records and logs, and the two Seatbelt profiles.
- Per-scenario capture commands and input hashes are under `capture/provenance/`; stdout/stderr are under `logs/capture/`. Raw snapshots are under `capture/snapshots/`.
- Per-scenario paired samples, full reader projections, pair manifests, success markers, source/executable hashes, commands and logs are under `compare-runs/`, `compare/provenance/`, and `logs/compare/`.
- `audit/ten-scenario-audit.json` and `.tsv` contain the independent full-field, metadata, render-contract, PNG and RGBA checks with per-scenario hashes. The audit also checks each fresh Tuiscotti old-writer pixel sample against the old producer output and the metadata-free projections/image hashes against the preserved original old records in `reference/original-old-records/`.

The producer used the exact TCB commit's tracked aarch64 reference binaries for Showcase, TablePro, Holla, and Jackin. Their hashes match the archived prior capture records for every scenario. Their captures have fresh metadata; equality is asserted between old and new readers for each newly captured snapshot. The old archived records are used only as an additional check of metadata-free snapshot projection, PNG and RGBA digests.

## Capture and driver note

The PTY capture profile allows writes only under this scratch root, `/dev/null`, and the PTY/TTY endpoints; it denies network access. The comparison profile allows writes only under this scratch root and `/dev/null`; it also denies network access. The old/new writer and reader executables were copied from the previously reviewed build outputs only after their expected SHA-256 values matched. No Cargo, mise, Rustup, or shared cache commands were run for this capture.

All ten capture subprocesses exited successfully and wrote complete records and snapshots. The first outer driver then hit a `TypeError` during its final summary aggregation after those runs had completed. The exact executed driver source and exception are preserved at `capture/capture_all.executed.py` and `logs/capture-driver-summary-error.txt`; `scripts/summarize_captures.py` aggregated the saved results to 10/10 successes. `scripts/capture_all.py` contains the corrected count expression for a fresh run. This post-capture aggregation error did not alter any snapshot, sample, binary, or per-scenario result record.

## Check the evidence

From this directory, verify every preserved file except the manifest and the outer marker:

```sh
shasum -a 256 -c SHA256SUMS
```

`RUN_SUCCESS` contains the SHA-256 of `SHA256SUMS`; verify it with:

```sh
printf '%s  %s\n' "$(cat RUN_SUCCESS)" SHA256SUMS | shasum -a 256 -c
```

The run summaries bind the original scenario manifest. To rerun the independent audit on this completed tree:

```sh
/usr/bin/python3 scripts/audit_all.py
```

This is a correctness control for these ten scenarios on one macOS arm64 host. It is not a timing benchmark, a full corpus claim, a cross-platform claim, or a baseline approval.
