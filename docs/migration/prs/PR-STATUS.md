# Upstream PR status (all 11 drafts)

Probed 2026-10-26 (UTC+7 ~00:30): #276-#286 all OPEN, reviewDecision REVIEW_REQUIRED, no maintainer reviews yet.

Only check reporting: `license/cla` = SUCCESS on all 11. No Rust CI workflow runs on these PR branches (only the CLA policy check appears in `statusCheckRollup` / `gh pr checks`). Consequence: there is no upstream-CI proof of the per-task test suites; validation rests on the local greens recorded per task, downstream compilation of the qual stack (D-GUARD-1 / D-QUAL-2), and any CI that runs at maintainer merge time.

Action: keep monitoring for maintainer feedback; re-probe checks before R02.

## Re-probe 2026-10-26 (~01:00 +07:00)

All 11 drafts still OPEN/REVIEW_REQUIRED, no maintainer reviews, only
`license/cla` checks. Upstream `main` HEAD unchanged (`f276059`).
No registry release containing the new APIs. R02 stays pending.

## Tuiscotti PR

- [tailrocks/tuiscotti#15](https://github.com/tailrocks/tuiscotti/pull/15):
  `donbeave:goal/tui-test-extension` → `tailrocks:main`. Opened with the
  full migration (D01–D07 accepted, V01 167/167 green). Depends on the
  provisional pinned fork (`donbeave/tui-test` `goal/qual-stack-r2`
  @ `6eb3b14`) per D-R02-1 until the upstream PRs merge and a registry
  release exists.

## Fork maintenance

- `donbeave/tui-test`: 11 `goal/uXX-*` task branches + `goal/qual-stack`
  pushed. Additive merges only; on task-branch moves, re-merge forward,
  bump the pinned rev, re-run gates, re-review.
- `donbeave/tuiscotti`: `goal/tui-test-extension` pushed, PR #15 open.
  If `tailrocks/main` moves, merge forward and re-run gates.
