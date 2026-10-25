# Decision log

## D-P01-1: Work placement

Worktrees live under `/Users/donbeave/tuiscotti/repos/` (`downstream`, `upstream`, `upstream-wt-*`). Migration records live in the downstream repository under `docs/migration/`. Evidence: `docs/migration/workspaces.md`.

## D-P02-1: Palette method

The built-in view palette comes from an in-memory `AlacrittyEmu` with `Profile::default()` through `Emulator::colors()`. No child, PTY, or session starts. Corroborated by `Colors::default()` source. Canonical digest `92757b147e00dec394b10b7be2fa42f782f0d30325a5922570080f91d60f641e`. Never recalculate from a later dependency version.

## D-P02-2: Font bytes

Upstream bundled JetBrains Mono bytes are `pub(crate)`. Downstream keeps licensed font assets under version control. U04 decides whether upstream exposes bytes publicly. Evidence: `render/font.rs` `REGULAR_FONTS`/`STYLED_FONTS` visibility; font table in `api-contract.md`.

## D-U01-1: CreateMissing without direct create-new

`CreateMissing` writes through a same-directory temp file plus `hard_link` instead of a direct `create_new` on the final path. Same failure semantics, and readers never see a partial baseline. Found through a real race flake during writer testing. Evidence: `snapshot_policy.rs` concurrency tests, 15/15 stress runs green.

## D-U01-2: Rejected review suggestions

U01 review (verdict accept) left two SUGGESTIONs, both rejected with reason:

- Temp-file `create_new` collision under PID reuse returns a typed `NotFound` error, never wrong data. A retry loop adds its own edge cases for a state that needs a stale temp file plus PID reuse at once. Rejected: safe typed error beats added machinery.
- Crash residue `.<stem>.new-*` dotfiles stay. A best-effort sweep risks deleting a live writer's temp file. Rejected: residue is the documented cost of atomic writes.

## D-U06-1: png as dev-dependency

U06-B decodes through the workspace `png` crate declared as a dev-dependency of `tui-test-rs`, mirroring `tui-test-cli`. No new package version enters the graph. The library keeps no PNG dependency. Evidence: `cargo tree -i png`, unmodified `Cargo.lock`.

## D-SIGN-1: Sign-off identity

Upstream history carries `Signed-off-by` on maintainer commits. Task commits get `Signed-off-by: Alexey Zhokhov <alexey@zhokhov.com>`, which matches both the repository git config and the `donbeave` GitHub identity (`gh api user`). No invented identity.
