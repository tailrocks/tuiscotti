# Consumer inventory

Recheck paths and exact interfaces after the mapper lands. The following is a read-only inventory of the current refactored checkout, not proof that binary persistence exists.

- `sample/record.rs` owns existing sample serialization, byte loading, validation and digest entry points. Remove complete Sample serde and all JSON sample readers/writers when replacing them with the private binary codec. Preserve constructors and accessors.
- `check/store.rs` and `approve/paths.rs` set `.tui.json` baseline suffixes. `check/evidence.rs` writes `sample.tui.json` and `expected.tui.json`; update these and all corresponding links together to `.tui.pb`.
- `approve/transaction.rs`, `approve/paths.rs` and `check/approval_lock.rs` own the existing lock, re-read, exact-byte hash, stale checks, re-render, comparison and atomic install sequence. Keep original candidate bytes throughout.
- CLI help and input/operations live in `crates/tuiscotti-cli/src/`. Preserve current flags, exit codes, result JSON and its file-byte digest meanings. `report/index.rs` reads result JSON, not sample files.
- Functional test file names and expectations occur under `crates/tuiscotti-cli/tests/functional/`; retain the four original test IDs. Keep result.json, CLI JSON and private digest projection fixtures distinct from removed JSON Sample support.
- Documentation updates include root and crate READMEs, `docs/API.md`, `docs/SNAPSHOTS.md`, `docs/CLI.md`, `docs/FIRST-SUCCESS.md`, module docs and the compressed-baseline spec. Frozen migration records remain unchanged. Record decisions in `docs/DECISIONS.md`.

Required new tests cover full-value binary round trips, old semantic/digest goldens, malformed/oversized and JSON rejection, no baseline writes from check, exact-byte evidence and acceptance including legal noncanonical wire, CI refusal, stale digests, locking, concurrent evidence and read consistency. Recheck CLI output/error codes and report links through the actual binary. Generated CI changes go through `.velnor/config.toml` and Velnor regeneration.
