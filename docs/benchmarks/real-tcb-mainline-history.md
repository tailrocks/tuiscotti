# Actual TCB mainline history measurements

## Source identity

Repository: `https://github.com/tailrocks/terminal-components-baseline.git`.
At measurement time (`2026-11-08`), `git ls-remote` returned `534e0ff5af75b840d93038dd02ece0839cf4f71b` for `refs/heads/main`.

Two successive real mainline commits were measured:

| State | Commit | Parent | Purpose |
|---|---|---|---|
| JSON baseline history | `f539d9a61b1fe1e9fab13f4e976c872b09b8fb9a` | earlier main history | Parent of binary migration. |
| Binary migration | `1d7f63dbb13ecb2bf3e3295e7a296dc2080a62b1` | `f539d9a61b1fe1e9fab13f4e976c872b09b8fb9a` | `feat: migrate TCB visual baselines to protobuf (#6)`. |
| PR2 mainline update | `534e0ff5af75b840d93038dd02ece0839cf4f71b` | `1d7f63dbb13ecb2bf3e3295e7a296dc2080a62b1` | `Test/pre refactor protection (#2)`. |

Each transition is exactly one commit. No synthetic commits or edits were inserted into these histories.

## Repacked history and incoming commit sizes

Measurements used Git `2.54.0 (Apple Git-157)` on Darwin 27.2 arm64. Each commit was fetched and checked out in one isolated repository, then repacked with `git repack -Adf --window=10 --depth=50`. Full-history pack sizes come from the `.pack` file byte length. Thin incoming packs were generated against the exact first parent with the same window/depth settings.

| State / transition | Repacked full-history pack | Pack SHA-256 | Thin incoming pack | Thin SHA-256 |
|---|---:|---|---:|---|
| `f539d9a` JSON history | 3,439,656 B | `bc37313be8811cebe9c7509c7b7b861ce550016d50dc68ab56bb32e7cc799e25` | — | — |
| `f539d9a` → `1d7f63d` migration | 4,050,026 B | `fa4d6419386697dd5141684908be8bb47213d5b6c9ecee2610932448f4f0a6a0` | 446,672 B | `d0b0bb649805f27f18c7b2bcbbfaede5b1702da44f1068c6c00936b69c9ec5b3` |
| `1d7f63d` → `534e0ff` PR2 mainline update | 40,830,792 B | `232a3583df523b27eda41713ddae537cca0d378f7c4267251b7c358599c46088` | 36,877,913 B | `4c3c32dd6f8dc017abe5147d8aa69527fc7f8fd7a677c3a8374142cb11acc10d` |

The full repacked history increased by 610,370 B for the migration and 36,780,766 B for the later PR2 commit. These full-commit costs include every file changed by each real commit, not only active baselines. Repacked size depends on Git pack construction; the sequential single-repository procedure above is the comparable measurement. A separate clean-clone attempt produced a different pack layout and is not used in this table.

## Tracked sample-tree scope

The selected current functional roots counted here are `tests/functional/baselines/functional`, `apps/jackin/tests/functional/baselines/functional`, and `tests/functional/baselines/showcase/buttons`.

| Commit | Selected current functional-root files | JSON files / bytes | PB files / bytes |
|---|---:|---:|---:|
| `f539d9a` | 161 | 161 / 187,110,263 B | 0 / 0 B |
| `1d7f63d` | 161 | 0 / 0 B | 161 / 416,750 B |
| `534e0ff` | 179 | 0 / 0 B | 179 / 459,282 B |

The migration commit changes the active tree from 161 JSON files to 161 PB files. The next commit modifies 87 active PB files and adds 18, for a net active-tree increase of 42,532 bytes. The tree byte counts describe committed files; they do not establish logical equivalence between old and new sample contents.

The `534e0ff` commit also adds 1,868 protected historical-root PBs under `tests/functional/baselines/pre_refactor/` (4,962,965 checkout bytes), 416 acceptance-receipt files (60,574,883 checkout bytes), and extensive test/reference-app material. These historical-root PBs are tracked test baselines with live consumers: `tests/functional/support.rs::baseline_root` routes `pre_refactor/` IDs to that tree, and `tests/functional/cases/j01_refresh.rs` constructs such IDs. They are separate from the selected current functional roots counted above; some historical-root samples are exercised by tests. The whole commit changes 3,044 paths (352,748 insertions, 1,539 deletions). The 1,868 paths are additions in this commit. They materially contribute to its 36.9 MB incoming pack, so that number is the actual PR commit cost and must not be presented as the isolated Git cost of the selected current functional-root update.

## Limits and missing inputs

This is a real TCB mainline migration followed by one real PB baseline update. It does not provide a repeated sequence of post-migration baseline-update commits. The active mainline at `534e0ff` contains only one commit after the format migration that modifies active PB baselines. The first transition is the format migration itself, not an additional ordinary PB update.

There is no paired JSON representation of the `534e0ff` selected current functional-root update. The protected `pre_refactor` PB set provides historical-root samples, but all were added in one commit; it is not a sequence of paired JSON/PB captures aligned across successive actual mainline baseline-update commits. These observations therefore do not isolate the Git cost of storing the same logical update sequence as JSON versus PB, and do not close the multi-revision history requirement. That comparison needs exact PB outputs (or fresh captures at historical application revisions) for at least two successive real baseline updates, with matching logical-state and embedded-PNG verification against each JSON version. Do not synthesize intermediate changes by editing sample bytes.

No pack-generation duration, clone/checkout duration, or filesystem cold-cache measurement was collected. The metadata archive contains commands, raw `count-objects` output, pack hashes, and commit/tree/blob identity records. Full isolated Git repositories and pack outputs remain in external scratch storage and are not shipped. This report does not claim native Linux or cross-platform results.

The metadata archive is
[`tuiscotti-mainline-git-history-20261108.tar.gz`](../../benchmark-evidence/tuiscotti-mainline-git-history-20261108.tar.gz)
(SHA-256 `517d1ae7053b7cc1e32483f0d727d0cead7186cc97153b7d977254ede6e4f97a`).
It contains the commit object headers, tree/blob manifests, commands, raw
metadata logs and checksums; it omits sample binaries and pack outputs.
