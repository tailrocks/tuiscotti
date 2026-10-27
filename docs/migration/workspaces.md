# Workspaces

Record P01 branch, commit, and path facts for the migration.

## Checkouts

| Repository | Local path | Remote | Base commit | Branch |
|---|---|---|---|---|
| Downstream `tailrocks/tuiscotti` | `repos/downstream` | `origin` `https://github.com/tailrocks/tuiscotti.git` | `a1ea27f757d75d07b05499276cd932e9f8e067ad` | `goal/tui-test-extension` |
| Upstream `microsoft/tui-test` | `repos/upstream` | `origin` `https://github.com/microsoft/tui-test.git` | `f2760592ed00e90436f8c3bae124dab712eaecf8` | `main` (task branches `goal/u01-*` through `goal/u11-*` created per task) |
| Upstream fork | remote `fork` in `repos/upstream` | `https://github.com/donbeave/tui-test.git` | tracks `origin/main` | PR branches pushed here |

## Permissions

- Upstream `microsoft/tui-test`: viewer permission `READ`. No push to protected branches. Draft PRs go to the `donbeave/tui-test` fork (viewer permission `ADMIN`).
- Downstream `tailrocks/tuiscotti`: viewer permission `ADMIN`. Integration branch `goal/tui-test-extension` created at the recorded base commit.

## Isolation

Each task writer gets an isolated worktree under `repos/` with one owner per task. Two agents never change the same file at the same time. Dependent tasks stay ordered per the task board in `docs/migration/tasks.md`.
