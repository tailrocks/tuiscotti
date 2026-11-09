# Unarchived local scratch, 2026-11-09

This branch keeps tuiscotti scratch that was not already a blob on origin.

Already on origin, and not copied again here:

- `main` and the merged pull requests
- `codex/preserve-local-source-20261109`, the primary checkout's source snapshot
- `codex/preserve-scratch-worktrees-20261109`, the worktree archive whose SHA-256 is `4d36279e3bf638820aad31f7b199e178d5e9cc4e607251554da195a8e955271f`
- the other `codex/preserved-*` branches

`from-icc-evidence-fix` and `from-error-envelope` are the only dirty scratch files whose bytes matched neither that worktree archive nor any object already stored in this repository. `docs/COMPRESSED-BASELINES.md` from the same icc tree is already in `codex/preserve-local-source-20261109`. The icc `docs/DECISIONS.md` copy was only a shorter form of the file on `main`, so it is not here.

`audit-notes` are the local design notes and the rollout index. `evidence` is each archive under `.audit` whose git blob was not already in the repository. `bundles` are local git bundles whose tip commits were not on `tailrocks/tuiscotti`, `tailrocks/terminal-components-baseline`, or `jackin-project/jackin`. Several of those bundles are thin and still need their recorded prerequisite commits.

Build directories, unpacked copies of these archives, and checkouts of commits already on a remote are not included.
