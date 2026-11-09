# Compressed Tuiscotti downstream regeneration and upgrade runbook

Observed 2026-11-05 from GitHub read-only APIs. Disposable representative rehearsals are recorded below. No downstream ref, active baseline, published pin, PR head or remote was written. Final Tuiscotti merge SHA is a prerequisite and is intentionally not fabricated.

## Live refs and sample inventory

| Target | Live ref (verified) | Current state and Tuiscotti | Current inventory |
|---|---|---|---|
| `tailrocks/terminal-components-baseline` main | `7f385611fba0f8d4a77bb02e0a991ad660b48746` (queried `gh api .../commits/main`) | default branch `main`; current docs specify Tuiscotti CLI/lib 0.4.0 at rev `d8d54155947f6303a41dfd6079b53c0abc16f29a`, live driver `tuiscotti-tui-test =0.2.1`. | Git tree contains 157 `.tui.json`: 101 under `tests/functional/baselines/functional`, one showcase baseline under `tests/functional/baselines/showcase`, and 55 elsewhere. Audit each of the 55 paths by role; do not assume all 157 are active samples or delete non-sample JSON. |
| `tailrocks/terminal-components-baseline` PR #2 | `60b5fb2762d91a3549bd4ac63b1d77d534ee53d2`, open, branch `test/pre-refactor-protection`, base `main`, merge status `UNKNOWN` per live `gh pr view`. | exact pin in Cargo.toml: lib renamed `tuiscotti-latest`, rev `d8d54155947f6303a41dfd6079b53c0abc16f29a`, `tui-test =0.2.1`; acceptance invokes installed CLI. | 109 active `tests/functional/baselines/functional/*.tui.json`; 1,868 frozen `tests/functional/baselines/pre_refactor/**/*.tui.json`; one showcase path; eight coverage-fixture sentinels. `scripts/check-refactor-coverage.py` expects inventory/test/receipt/disposition/baseline agreement and must be run after migration. Historical pre_refactor records and receipt evidence are retained. |
| `jackin-project/jackin` PR #1264 | `480ba8faab70e87d9c276da6ebd3985c8f92c258`, open, branch `feature/openshell-native`, base `main`, merge state `BLOCKED`, live `gh pr view`. (This is newer than preliminary inventory SHA `52fabbc…`.) | Current Cargo.toml pins component crates to `terminal-components-baseline` rev `a901c275055091810ae0d0ac30ec5fc09ce595d9` and Tuiscotti `0.4.0` at rev `d8d54155947f6303a41dfd6079b53c0abc16f29a`; live driver `=0.2.1`. | Git tree has zero `.tui.json` samples. `crates/jackin-testkit/tests/identity/coverage.rs` checks baseline journey capture identity, expected approved-file inventory, and currently recognizes `.tui.json` filenames. `identity/baseline.rs` and coverage tests need format-aware updates. Separate baseline source is the pinned terminal-components-baseline commit above. |

The terminal-components-baseline local checkout is exactly PR #2's live SHA but has unrelated modified/untracked files. The jackin local checkout is `a901c275…`, not the live PR head. Neither local checkout was edited or used as the source of current jackin facts; live API was used. Main's local `origin/main` is stale and was not used for the current main SHA/count. The preliminary `.audit/downstream-inventory.md` is stale on main and jackin heads.

## Capture and approval facts

At main, `docs/functional-testing.md` says the ordinary capture/validation journey is `mise install` then `cargo nextest run --locked --workspace --profile functional --no-tests fail`; each test ends in `App::visual`, and the named functional target is also `cargo nextest run --locked --test functional --no-tests fail`. Current approved samples are checked by FrozenStore. It documents the sole approval interface as:

```sh
tuiscotti accept --candidate <sample> --candidate-sha256 <sha256> \
  --expect-baseline missing --baselines tests/functional/baselines
```

PR #2 has the same journey and an explicit workstation-only wrapper `scripts/accept-baseline.py`. Its interface requires candidate path and SHA, expected prior baseline digest/state, baseline root, receipt path, precondition, binary path and SHA, plus manifest and optional fixture args/environment; it refuses if `FUNCTIONAL_CANDIDATE_GATE=1`, calls `tuiscotti accept`, and writes the receipt only on success. It refuses a receipt inside the baseline root. Its coverage script checks that PASS rows have receipts and live baseline files, but does not prove receipt candidate bytes equal the installed file after acceptance. Therefore staging needs an independent post-accept byte/hash verification; do not trust receipt presence alone. There is no documented bulk candidate capture workflow on PR #2. Exact candidate output naming/environment must be learned from the live helper and demonstrated in a disposable copy before issuing an executable bulk command.

Jackin PR #1264 has no committed samples. Its coverage test compares test journey names and `.visual("id")` lists against the separately pinned component baseline journey source, checks approved filenames under the baseline crate's `functional` root, and checks four known no-file scenarios. It mutates a copied JSON baseline to assert changed-cell failure. Update those paths/fixture mutations to `.tui.pb` and preserve the semantic assertions; do not copy sample files into jackin unless the revised design explicitly changes ownership.

## Required rollout sequence (after implementation merge)

1. Record the full merged Tuiscotti commit SHA. Re-query live main and PR heads and read current `AGENTS.md`, candidate helpers, workflows and scripts. The merged SHA, not a branch name, is the source pin.
2. For each target, create a disposable worktree/copy at its then-current head. Do not use or replace active baseline roots. Update Cargo git rev to the merged SHA, regenerate lockfile, and build the new offline `tuiscotti` CLI from that exact SHA. Keep `tuiscotti-tui-test` at its compatible exact registry pin unless the implementation contract explicitly requires an upgrade.
3. Use the repository's ordinary real-PTY functional test path to capture fresh candidates. Capture all active IDs on each supported OS/arch suffix required by `docs/SNAPSHOTS.md`; preserve IDs and `state-and-pixels` suffix pairing. Do not convert JSON. Record candidate path, ID, platform, executable hash, Tuiscotti SHA, command, and output.
4. Make a separate staging root from scratch. Review each candidate against its current approved sample/render and intended behavior. Explicitly approve every reviewed candidate on a workstation with `CI` absent using CLI `accept`. For a first binary approval into an empty staging root, use `--expect-baseline missing` even when a JSON sample exists for review; record that old JSON digest separately as review provenance. When replacing an already staged `.tui.pb`, use its exact current digest. Never batch auto-accept, and never use `accept` in tests or CI.
5. For PR #2, refresh and validate the reference manifest against the actual capture source, lockfile, comparator, and Tuiscotti revision before approval; route each approval through the policy wrapper and store receipts outside the root. Independently compute candidate SHA-256 before approval and staged `.tui.pb` SHA-256 after approval, and require equality for every ID; verify receipt records. The current checker does not establish installed-file digest equality. Keep historical pre_refactor data, receipts and unrelated JSON. Review generated CI updates through its source generator, not hand edits.
6. Run the offline `tuiscotti check`/frozen gate against the staged root, and run the normal Nextest functional suite and all repository coverage/CI checks from the disposable tree. For PR #2 run `scripts/check-refactor-coverage.py` with the current inventory, nextest list, dispositions, receipts, and staged baseline path; preserve its path arguments from the invocation in the current CI workflow. On each platform, verify the required capture IDs and byte identities. Review additions/removals and every changed visual before promoting the staged files in a downstream review.
7. Only after successful staging/review, prepare downstream commits/PR updates. Leave historical records intact. Rollback is reverting the downstream commit to its prior Git revision and restoring the old dependency pin; there is no claim that replacing a baseline directory is atomic. Publish neither pins nor artifacts until the appropriate release phase is authorized.

## Exact checks already available at PR #2

```sh
mise install
cargo nextest run --locked --workspace --profile functional --no-tests fail
cargo nextest run --locked --test functional --no-tests fail
# Per-ID acceptance must be explicit; see scripts/accept-baseline.py --help.
python3 scripts/check-refactor-coverage.py --fixtures scripts/coverage-fixtures
```

For exact coverage-checker production arguments, use its current workflow invocation rather than guessing. The script's required inputs are `--inventory`, `--nextest-list`, `--baselines`, `--dispositions`, and `--receipts` (verify against `--help` and workflow at the rollout head). Standard repository Rust gates follow that repository's own current AGENTS instructions.

## Risk and remaining rollout work

The representative capture/approval/check workflow is now exercised once per target on macOS, using only local path patches to the immutable implementation snapshot. The complete bulk regeneration, every active ID, cross-platform suffix lanes, full target CI gates, actual post-merge SHA pin, lockfile update, and jackin identity/coverage source migration remain unverified. The current zstd level 3 is provisional pending final benchmark evidence. Do not treat the local staging approvals as human approvals or published downstream baseline changes.

## Representative disposable rehearsal evidence (2026-11-05)

### Immutable implementation input and isolation

- Input: `/private/tmp/tuiscotti-rehearsal-input-20261105`; `shasum -a 256 -c SHA256SUMS` passed all 673 entries. SHA-256 of the manifest: `c159934ac6b9c878d2bdb248340aadf69144eb7774a34fce2a931e96a2bcb4de`.
- A writable setup copy was made at `/private/tmp/tuiscotti-rehearsal-disposable-20261105`; downstream `.cargo/config.toml` in each rehearsal points the Git source `https://github.com/tailrocks/tuiscotti` to the immutable input's `crates/tuiscotti` path. This lets Cargo test a local path implementation without changing or publishing any dependency pin.
- `MISE_CI=1 mbx doctor` in the implementation copy and Jackin disposable clone: 0 failures, two atime warnings. Rust/Cargo 1.99.1 and mise Cargo wrapper were active. Every Cargo build/test below was invoked as `MISE_CI=1 timeout 230 mbx ...`; the 230-second timeout is a hard per-command cap.
- Work areas: PR2 via `git archive` at exact PR head in `/private/tmp/tuiscotti-tcb-pr2-rehearsal-20261105`; main clone at exact main SHA in `/private/tmp/tuiscotti-tcb-main-rehearsal-20261105`; Jackin detached clone at exact PR head in `/private/tmp/tuiscotti-jackin-rehearsal-20261105`. Additional acceptance roots are each isolated under `/private/tmp/tuiscotti-staging-<target>-20261105/`.

### terminal-components-baseline main, SHA `7f385611fba0f8d4a77bb02e0a991ad660b48746`

The command `MISE_CI=1 timeout 230 mbx nextest run --offline --workspace --test functional --no-tests fail -E 'test(showcase_button_hover_hold_release_cancel_and_disabled_truecolor)' --no-capture` first stopped at Cargo target selection because `jackin-console` requires feature `demo`. The narrower actual producer command succeeded in building/running the test and predictably failed only because `.tui.pb` was missing while the active baseline still existed as `.tui.json`:

```sh
MISE_CI=1 timeout 230 mbx nextest run --offline -p junie-tui --test functional \
  --no-tests fail \
  -E 'test(showcase_button_hover_hold_release_cancel_and_disabled_truecolor)' \
  --no-capture
```

Nextest ran one test, 116 skipped, 0 passed / 1 failed as expected for the missing binary baseline. Candidate artifact: `functional/showcase_buttons_truecolor`, 4,057 bytes, SHA-256 `ba89bc1ef4f84eac62fe2700bc8b6c12e023f2b88c4a2dec7ee303213861fd9b`. It was copied to `staging-main/candidates/showcase_buttons_truecolor.tui.pb`. The candidate renderer produced PNG SHA `5df87092160b2d5fac61e0b83f80a7f09f06b25a842fb5bf703dbc025f80c55f`, 1600×1000; this was byte-identical to the PR2 render. A one-off inspector using the public `Sample::from_bytes` API confirmed all 11 state-gate fields matched the current approved JSON snapshot. Main and PR2 candidate file digests differ although state-gate fields and rendered PNG match; the specific non-state byte difference was not isolated. I explicitly approved this one state sample into the disposable root only:

```sh
unset CI
/private/tmp/tuiscotti-rehearsal-disposable-20261105/target/debug/tuiscotti accept \
  --candidate /private/tmp/tuiscotti-staging-main-20261105/candidates/showcase_buttons_truecolor.tui.pb \
  --candidate-sha256 ba89bc1ef4f84eac62fe2700bc8b6c12e023f2b88c4a2dec7ee303213861fd9b \
  --expect-baseline missing --baselines /private/tmp/tuiscotti-staging-main-20261105/baselines
/private/tmp/tuiscotti-rehearsal-disposable-20261105/target/debug/tuiscotti check \
  --sample /private/tmp/tuiscotti-staging-main-20261105/candidates/showcase_buttons_truecolor.tui.pb \
  --baselines /private/tmp/tuiscotti-staging-main-20261105/baselines \
  --artifacts /private/tmp/tuiscotti-staging-main-20261105/check-artifacts
```

Observed `accept: pass`, `check: pass`; candidate and installed `.tui.pb` compared equal and both hashed to `ba89…fd9b`. This main revision has no receipt wrapper; its docs identify the CLI acceptance as the approval path. A local view-image call once showed a blank rendering, but re-rendered files were byte-identical to the fully rendered PR2 screen and the public decoded snapshot matched every gate field. This was a viewer display artifact, not a sample mismatch.

### terminal-components-baseline PR #2, SHA `60b5fb2762d91a3549bd4ac63b1d77d534ee53d2`

The actual active sample producer test was run from the archive copy:

```sh
MISE_CI=1 timeout 230 mbx nextest run --offline --workspace --test functional \
  --no-tests fail \
  -E 'test(showcase_button_hover_hold_release_cancel_and_disabled_truecolor)' \
  --no-capture
```

One test ran, 1,095 skipped, 0 passed / 1 failed because `.tui.pb` did not exist; the active `.tui.json` remained untouched. Candidate ID `functional/showcase_buttons_truecolor`, 4,057 bytes, SHA `9ea5d8443d0fc02d432deedea1f3a689205a4e34dd96d10d6c5de46738a708fc`. All 11 state-gate fields matched the approved current JSON and the rendered candidate was reviewed. The explicit local staging approval used the existing policy wrapper, outside candidate gate, with receipt outside baselines:

```sh
unset CI FUNCTIONAL_CANDIDATE_GATE
python3 scripts/accept-baseline.py \
  --candidate staging-pr2/candidates/showcase_buttons_truecolor.tui.pb \
  --candidate-sha256 9ea5d8443d0fc02d432deedea1f3a689205a4e34dd96d10d6c5de46738a708fc \
  --expect-baseline missing --baselines staging-pr2/baselines \
  --receipt staging-pr2/receipts/showcase_buttons_truecolor.json \
  --precondition 'Disposable rehearsal only: rendered PNG reviewed and all state-gate fields equal the old sample.' \
  --binary <captured-showcase-binary> --binary-sha256 <captured-binary-sha256> \
  --fixture-arg=--page --fixture-arg=buttons --fixture-arg=--motion --fixture-arg=paused \
  --env=TERM=xterm-256color --env=COLORTERM=truecolor \
  --env=JUNIE_TUI_CLIENT_PLATFORM=macos --tuiscotti <local-input-cli>
```

The actual binary and SHA, plus complete receipt, are at `staging-pr2/receipts/showcase_buttons_truecolor.json`. Its source, lockfile, comparator, and Tuiscotti provenance fields came from the historical reference manifest and do not identify the rehearsal implementation; refresh and validate that manifest before any rollout receipt. The script returned `accept: pass`. An independent `cmp` and SHA after acceptance proved the installed `.tui.pb` digest equaled the candidate digest `9ea5…708fc`; offline CLI check passed. A repeat attempt with `--expect-baseline missing` correctly failed `STALE_BASELINE` because the first explicit staging approval had installed the sample. A separate `holla_help_overlay` run produced a fresh candidate but its rendered state did not match the old sample; it was deliberately not approved. This confirms the runbook must retain human visual review per sample, not bulk-bless.

### jackin PR #1264, SHA `480ba8faab70e87d9c276da6ebd3985c8f92c258`

The real identity test, with only the Tuiscotti Git source path-patched to immutable local input, ran:

```sh
MISE_CI=1 timeout 230 mbx nextest run --offline -p jackin-testkit --test identity \
  --no-tests fail \
  -E 'test(an_unchanged_approved_copy_passes_the_journey_check)' \
  --no-capture
```

One test ran, 46 skipped, 0 passed / 1 failed as expected: current pinned component baseline had only `.tui.json`, and the new library correctly returned `Missing` for `.tui.pb`. Fresh sample ID `functional/jackin_launch_reconnect`, 4,148 bytes, SHA `24322d73987166a5c8a6aa58cb1d3316ec0ad8b058e1fe144eadf42e17a7bcb7`. A disposable inspector confirmed all 11 gate fields equal the old approved JSON baseline. Candidate and old reference PNGs also hashed identically (`33651450e5292c0d2550f5d04f5871615aabf6524b3ebd3cdd50cb17cd4974a7`), even though one `view_image` display appeared different; use file/hash and decoded state evidence for this sample. After that local review, the CLI explicitly accepted into an empty Jackin staging root and `check` passed:

```sh
unset CI
/private/tmp/tuiscotti-rehearsal-disposable-20261105/target/debug/tuiscotti accept \
  --candidate /private/tmp/tuiscotti-staging-jackin-20261105/candidates/jackin_launch_reconnect.tui.pb \
  --candidate-sha256 24322d73987166a5c8a6aa58cb1d3316ec0ad8b058e1fe144eadf42e17a7bcb7 \
  --expect-baseline missing --baselines /private/tmp/tuiscotti-staging-jackin-20261105/baselines
/private/tmp/tuiscotti-rehearsal-disposable-20261105/target/debug/tuiscotti check \
  --sample /private/tmp/tuiscotti-staging-jackin-20261105/candidates/jackin_launch_reconnect.tui.pb \
  --baselines /private/tmp/tuiscotti-staging-jackin-20261105/baselines \
  --artifacts /private/tmp/tuiscotti-staging-jackin-20261105/check-artifacts
```

Candidate and installed `.tui.pb` were byte-identical with SHA `24322d…bcb7`. This stage is outside Jackin and outside terminal-components-baseline; the actual dependency's baseline tree was read only.

A temporary inspector test was added only to each disposable source copy to print decoded sample state and compare public decoded fields to the old JSON object. It was not added to any repository branch. These semantic reads were verification only; no JSON was converted or used as a fallback by Tuiscotti.

### Exact live-head evidence and remaining blockers

Final read-only recheck immediately before this update returned:

- `gh api repos/tailrocks/terminal-components-baseline/commits/main --jq .sha` → `7f385611fba0f8d4a77bb02e0a991ad660b48746`.
- `gh pr view 2 -R tailrocks/terminal-components-baseline --json headRefOid,state,mergeStateStatus,url` → open `60b5fb2762d91a3549bd4ac63b1d77d534ee53d2`, merge status `DIRTY`.
- `gh pr view 1264 -R jackin-project/jackin --json headRefOid,state,mergeStateStatus,url` → open `480ba8faab70e87d9c276da6ebd3985c8f92c258`, merge status `BLOCKED`.

The main and Jackin detached disposable clones resolve to those exact hashes; the PR2 archive was created at its verified live head. No merge SHA exists, so do not update any published Cargo pin yet. The sample-level rehearsal proves producer, review, approval, post-install digest equality, and check for one scenario on each target, not a full baseline conversion/regeneration. The Jackin identity coverage currently hardcodes `.tui.json` lookup/mutation; it must be deliberately updated to `.tui.pb` in a later downstream PR. The complete PR2 coverage checker and all repository CI suites have not been run; its scripts and generated CI must be updated/reviewed from their current generators before rollout. Only macOS was available here; Linux and any architecture-specific suffix verification remain NOT VERIFIED.


A first standalone sample-inspector attempt used a separate MBX target and began compiling the full dependency graph; I interrupted it and switched to one-off inspector tests inside the already-built disposable consumer workspaces. No build remains running. All later commands were capped at 230 seconds. The one-off inspector sources exist only in the disposable work copies and are not repository changes.
