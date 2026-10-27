# Removals (D07, §3.3)

Method: `git ls-tree` file counts and `wc -l` over `crates/**/*.rs`
at `5afe5d7` (pre-removal) vs the F01 HEAD (`11a9203`); dependency
lists from the workspace `Cargo.toml` files. Line counts are
approximate (comments and blanks included, no density target used).
The After column is the F01 vintage, not current HEAD — the tree has
grown since (approval-port merge, qual-stack-r2 re-pin).

## Files

| Scope | Before (`5afe5d7`) | After (F01 `11a9203`) |
|---|---|---|
| `crates/**/src/*.rs` | 249 | 16 |
| `crates/**/tests,examples/*.rs` | 63 | 17 |
| `crates/**/*.rs` total | 311 | 33 |
| Rust lines (`wc -l`) | 65,679 | 14,973 |
| `git diff --stat` D07 range | — | 382 files, +251 / −57,797 |

## Crates removed (all source, tests, examples, fixture binaries)

`tuiscotti-core`, `tuiscotti-render`, `tuiscotti-runtime`,
`tuiscotti-insta`, `tuiscotti-fixtures`, `tuiscotti-bench`, `xtask`.
Also deleted: `crates/tuiscotti/tests/cargo_bin_unified.rs`
(piped-capture/runner contract) and the untracked fixture/insta husks.
Workspace members now: `tuiscotti`, `tuiscotti-cli` only.

## Direct dependencies removed with the crates

`termpane`, `crossterm` (direct), `swash`, `image-compare`, `insta`,
bench and fixture-only deps, all xtask deps. Facade direct deps now:
`tui-test-rs`, `base64`, `fs2`, `image`, `ratatui`, `serde`,
`serde_json`, `sha2` (workspace-inherited versions where applicable).

## Docs and automation removed

Removed-contract lanes in generated `ci.yml` (14 jobs + needs/expected
strings, via the source config); `mise.toml` `ci-xtask-*` / `ci-package`
tasks; contributor-guide sections presenting removed crates, the `pty`
feature, `Store`/`GroupedStore`/`cargo insta review` approval, and
`readme_lock.rs`; `PERFORMANCE.md` bench commands (retired under a
superseded banner, numbers kept as history); `TESTING.md` and
`SNAPSHOTS.md` rewritten for the two-crate tree.

## Kept

Two shipping crates (`tuiscotti`, `tuiscotti-cli`, executable name
`tuiscotti`); five guides (README, API, CLI, architecture, limitations)
plus corrected TESTING/SNAPSHOTS/PERFORMANCE/CONTRIBUTING; migration
records under `docs/migration/` (history stays in the record, never as
live documentation).
