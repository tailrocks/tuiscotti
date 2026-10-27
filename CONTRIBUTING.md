# Contributing

Pinned toolchain: Rust **1.98.1** (`rust-toolchain.toml`), edition 2024.
`cargo` on PATH may be a cache shim; behavior is identical.

## Setup

```sh
cargo fetch --locked        # warm the registry (offline builds after this)
cargo build --locked --offline
cargo nextest run --locked --offline   # full suite (preferred runner)
```

Pure-view execution is structural: tuiscotti holds no PTY, renderer,
engine, or approval-adapter dependency, and the workspace defines no
cargo features. Live sessions, input, locators, waits, capture,
rendering, and comparison come from `tui-test-rs`; application tests
drive `tui_test::Session` directly.

## Gates (run before every push)

```sh
cargo fmt --check
cargo clippy --locked --offline --all-targets --all-features -D warnings
cargo nextest run --locked --offline --all-features
cargo test --locked --offline --doc   # doctests ride separately
```

Workspace lints forbid `unwrap_used`, `expect_used`, `panic!`,
`todo!`, `dbg!` in shipped code; tests use the standard helpers.
`unsafe_code` is forbidden. DCO signoff required: `git commit -s`.

## Layout

- `crates/tuiscotti` — public facade: `view` (Ratatui buffer capture),
  `sample` (versioned visual samples), `check` (frozen-store checks),
  `approve` (`ApprovalStore` transaction), `report` (static evidence),
  `error`, plus the usage examples.
- `crates/tuiscotti-cli` — the `tuiscotti` offline binary
  (`check`/`accept`/`render`/`report`). Thin: parse args, call the facade.

Details: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Rules that bite

- **No legacy code.** Remove old paths completely — no shims, aliases,
  or deprecation periods. Breaking changes are preferred.
- **CI must never auto-accept.** There is no bless flag or variable;
  do not add one. Approval is `ApprovalStore::accept` /
  `tuiscotti accept`, on a workstation only.
  See [docs/SNAPSHOTS.md](docs/SNAPSHOTS.md).
- **Fail closed.** Missing or corrupt approvals fail the gate; unknown
  input to a guard fails rather than passes. New statuses go through
  the typed outcomes (`check::CheckOutcome`, `approve::AcceptOutcome`)
  and both stores.
- **Determinism.** Same frame + same profile + same vendored font
  bytes = byte-identical PNGs. No system fonts, no wall-clock in
  artifacts (provenance timestamps excepted), no unordered maps in
  canonical output.
- **Docs match code.** No automated fence-lock test remains
  (`readme_lock.rs` was removed with the old CLI surface at D07). If
  you change a documented behavior, update the README in the same
  commit; reviewers verify claimed flags and flows against `--help`
  and the CLI guide on doc diffs.
- **Link check.** Every `docs/*.md` + `README.md` link must resolve;
  follow the procedure in [docs/TESTING.md](docs/TESTING.md) before
  pushing doc edits.

## Reviews and merges

One working branch (`goal/tui-test-extension`); small
PRs, merged promptly after all gates pass. Before merge, read every
review/comment/thread; verify findings against code, tests, and docs;
fix + verify + commit + push + reply with the fixing commit URL, or
reply with evidence before resolving. Merge only with zero
unaddressed feedback, zero unresolved threads, and all required
checks green at the final head SHA.
