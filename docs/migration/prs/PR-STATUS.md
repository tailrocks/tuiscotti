# Upstream PR status (all 11 drafts)

Probed 2026-10-26 (UTC+7 ~00:30): #276-#286 all OPEN, reviewDecision REVIEW_REQUIRED, no maintainer reviews yet.

Only check reporting: `license/cla` = SUCCESS on all 11. No Rust CI workflow runs on these PR branches (only the CLA policy check appears in `statusCheckRollup` / `gh pr checks`). Consequence: there is no upstream-CI proof of the per-task test suites; validation rests on the local greens recorded per task, downstream compilation of the qual stack (D-GUARD-1 / D-QUAL-2), and any CI that runs at maintainer merge time.

Action: keep monitoring for maintainer feedback; re-probe checks before R02.

## Re-probe 2026-10-26 (~01:00 +07:00)

All 11 drafts still OPEN/REVIEW_REQUIRED, no maintainer reviews, only
`license/cla` checks. Upstream `main` HEAD unchanged (`f276059`).
No registry release containing the new APIs. R02 stays pending.

## Re-probe 2026-10-26 (~18:44 UTC) + round-3 heads

All 11 drafts still OPEN/DRAFT/REVIEW_REQUIRED, no maintainer
reviews, no submitted GitHub reviews, only `license/cla` checks
(spot-checked #276, #280, #286 via `gh pr checks`). Upstream `main`
HEAD unchanged (`f276059`). No registry release containing the new
APIs. Consequence unchanged: no upstream-CI proof of the per-task
suites; validation rests on the local greens, the downstream
compilation of the qual stack, and any CI that runs at maintainer
merge time.

Round-3 heads: fork tip == PR head == local task-worktree HEAD for
all 11 (verified via `git ls-remote`, `gh pr view --json
reviewDecision,state,isDraft,headRefOid`, and local `git rev-parse`).

| PR | Task | Branch | Round-3 head | Status |
|---|---|---|---|---|
| #276 | U01 | goal/u01-strict-snapshots | c14303e | reworked-verified; awaiting-maintainer-CI |
| #277 | U06 | goal/u06-pixel-comparison | 4fb316a | reworked-verified; awaiting-maintainer-CI |
| #278 | U02 | goal/u02-owned-snapshot | cb40f25 | reworked-verified; awaiting-maintainer-CI |
| #279 | U09 | goal/u09-raw-bytes | 97f09f0 | reworked-verified; awaiting-maintainer-CI |
| #280 | U03 | goal/u03-offline-rendering | a6adbcc | reworked-verified; awaiting-maintainer-CI |
| #281 | U10 | goal/u10-request-waits | 5472eea | reworked-verified; awaiting-maintainer-CI |
| #282 | U07 | goal/u07-checked-input | 212ba1d | reworked-verified; awaiting-maintainer-CI |
| #283 | U11 | goal/u11-underline-color | 7a1c7c9 | reworked-verified; awaiting-maintainer-CI |
| #284 | U04 | goal/u04-pinned-fonts | d5b1469 | reworked-verified; awaiting-maintainer-CI |
| #285 | U05 | goal/u05-graphemes | ee633fa | reworked-verified; awaiting-maintainer-CI |
| #286 | U08 | goal/u08-launch-options | 8b44501 | reworked-verified; awaiting-maintainer-CI |

Full head SHAs (PR head OIDs):

```text
#276 c14303eb7d5ebacaebbe455482cd4781b8e53383
#277 4fb316aa56c65cc659b4a43ee4921c45e33cecc0
#278 cb40f257135cab8716ebb93bd23f2ee9f944f8e9
#279 97f09f05453ce605fa4bc1bbeef245b320148dcb
#280 a6adbcc0453ef4501dfbd3fc62f2ea944152371e
#281 5472eeaa2eab433b53c4deedd1c8a5d83acf8167
#282 212ba1db26987a45537708e98c82db77c90bb734
#283 7a1c7c989527b95cf01b69638696ab8a4d81169f
#284 d5b14695458b651986bd6916df433487b6e94d13
#285 ee633fa5146110b5a792c3ebaa651f1a66d50af3
#286 8b4450178025bb8976604ad10b830ce7f2ffac6c
```

Reviews: the tasking session reports third-review ACCEPT verdicts
on the round-3 rework; the fourth review round is pending. Action:
keep monitoring for maintainer feedback; re-probe checks before R04.

## Tuiscotti PR

- [tailrocks/tuiscotti#15](https://github.com/tailrocks/tuiscotti/pull/15):
  `donbeave:goal/tui-test-extension` → `tailrocks:main`. Opened with the
  full migration (D01–D07 accepted, V01 167/167 green). Depends on the
  provisional pinned fork (`donbeave/tui-test` `goal/qual-stack-r3`
  @ `9e97999`) until the upstream PRs merge and a registry release
  exists. Re-pinned from `goal/qual-stack-r2` @ `6eb3b14` (D-PIN-1)
  to `goal/qual-stack-r3` (round-2 integration). Downstream re-verified
  at HEAD `1501d5a` (205/205 both lanes, fmt/clippy clean; see
  `docs/migration/verification.md`). DCO-fixed force-push landed:
  all branch commits carry `Signed-off-by`, fork tip == `1501d5a`,
  PR head `1501d5a` READY (updated 2026-10-26T18:40:48Z).

## Fork maintenance

- `donbeave/tui-test`: 11 `goal/uXX-*` task branches + `goal/qual-stack`
  pushed. Additive merges only; on task-branch moves, re-merge forward,
  bump the pinned rev, re-run gates, re-review.
- `donbeave/tui-test` `goal/qual-stack-r3` tip `9e97999`
  (`9e97999763244b817781b4f49064a9b76d0a2632`): round-2 integration
  (merges corrected U02–U11 rework heads) + merge-fallout fix (dup
  `last_output_time` from U08+U10). Stack suites 129/129 integration
  + 567/567 lib, session-verified locally.
- `donbeave/tuiscotti`: `goal/tui-test-extension` pushed, PR #15 open.
  If `tailrocks/main` moves, merge forward and re-run gates.
