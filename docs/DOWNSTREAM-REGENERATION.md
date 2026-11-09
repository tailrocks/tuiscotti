# Regenerate downstream binary baselines

Use this runbook after the binary-only Tuiscotti implementation has merged. It covers `tailrocks/terminal-components-baseline` main, that repository's PR #2, and `jackin-project/jackin` PR #1264. It does not convert JSON files or approve samples automatically.

Do not upgrade a downstream Cargo pin before the Tuiscotti implementation merges. Capture, review, approval, coverage, and rollback each use separate steps. Run all capture work in a fresh disposable checkout. Keep active baseline roots read-only until every staged sample and required check has passed.

## Check the targets and their current inventory

Recheck the refs before you create disposable checkouts. These read-only commands were used for the evidence dated 2026-11-06:

```sh
gh api repos/tailrocks/terminal-components-baseline/commits/main --jq .sha
gh pr view 2 -R tailrocks/terminal-components-baseline --json headRefOid,state,mergeStateStatus
gh pr view 1264 -R jackin-project/jackin --json headRefOid,state,mergeStateStatus
```

The read-only recheck returned TCB main `7f385611fba0f8d4a77bb02e0a991ad660b48746`, TCB PR #2 `60b5fb2762d91a3549bd4ac63b1d77d534ee53d2` (open, dirty), and Jackin PR #1264 `480ba8faab70e87d9c276da6ebd3985c8f92c258` (open, blocked). Treat these as dated evidence, not current refs. Record the full SHA returned by a new recheck before every run.

| Target | Baseline ownership and current pins at the checked head | Inventory and verified capture status |
|---|---|---|
| TCB main | Owns `tests/functional/baselines`. Tuiscotti lib and CLI are pinned at `d8d54155947f6303a41dfd6079b53c0abc16f29a`; `tuiscotti-tui-test` is `=0.2.1`. | 101 active functional JSON samples. The tree also had one showcase sample and 55 other `.tui.json` files; audit those by role and preserve unrelated JSON. macOS capture produced 101 of 101 active functional candidates, plus one unexpected ID (`showcase/buttons/held`). This is one platform's producer run, not full cross-platform coverage. |
| TCB PR #2 | Owns active and frozen baseline trees. Tuiscotti lib is renamed `tuiscotti-latest` at `d8d54155947f6303a41dfd6079b53c0abc16f29a`; `tui-test` is `=0.2.1`. Approval uses `scripts/accept-baseline.py`. | 109 active functional JSON samples, 1,868 frozen `pre_refactor` records, one showcase path, and eight coverage-fixture sentinels. Partitioned macOS capture found 104 of 109 active candidates. Five active IDs remain unresolved; see the table below. Historical records, receipts, sentinels, and unrelated JSON are not candidates for replacement. |
| Jackin PR #1264 | Owns no sample files. Its identity suite reads `functional` baselines from its separate TCB pin, `a901c275055091810ae0d0ac30ec5fc09ce595d9`. It pins Tuiscotti `0.4.0` at `d8d54155947f6303a41dfd6079b53c0abc16f29a` and `tuiscotti-tui-test =0.2.1`. | Identity source at the checked Jackin head names 55 distinct visual IDs. The recorded single-sample smoke captured `functional/jackin_launch_reconnect`; the other 54 remain uncaptured. Full identity coverage and the `.tui.pb` migration are not verified. Regenerate in the chosen TCB baseline source, then update Jackin's identity path and mutation checks. Do not copy samples into Jackin. |

Native Linux and other required OS and architecture suffix lanes are **not verified**. The old JSON inventories are review provenance only. A `.tui.json` file does not satisfy a `.tui.pb` check.

## Prepare each disposable checkout

After Tuiscotti merges, record its full commit SHA as `TUISCOTTI_MERGED_SHA`. Confirm the commit is reachable from the intended upstream, then pin each downstream disposable checkout to that exact SHA and regenerate its lockfile using that repository's current instructions. Keep `tuiscotti-tui-test` at its exact compatible pin unless the merged contract requires a change. Do not pin a branch name or an unmerged local revision.

For each repository checkout, set `REPOSITORY_URL`, `CHECKOUT`, and `HEAD_SHA` from the repository URL and full SHA returned by the fresh read-only recheck. Use a new checkout path. Fetch that exact SHA and detach at `FETCH_HEAD`:

```sh
: "${REPOSITORY_URL:?set the repository URL}"
: "${CHECKOUT:?set a new disposable checkout path}"
: "${HEAD_SHA:?set the full SHA from the read-only recheck}"
if [ -e "$CHECKOUT" ] || [ -L "$CHECKOUT" ]; then
  printf '%s\n' 'checkout path already exists; choose a new path' >&2
  exit 2
fi
git clone "$REPOSITORY_URL" "$CHECKOUT"
git -C "$CHECKOUT" fetch origin "$HEAD_SHA"
git -C "$CHECKOUT" checkout --quiet --detach FETCH_HEAD
test "$(git -C "$CHECKOUT" rev-parse HEAD)" = "$HEAD_SHA"
```

Use `https://github.com/tailrocks/terminal-components-baseline.git` for TCB main and PR #2. Use `https://github.com/jackin-project/jackin.git` for Jackin PR #1264. Set a distinct `CHECKOUT` and rechecked `HEAD_SHA` for each. Keep each checkout's existing baseline tree unchanged. Do not reuse a previous candidate or staging directory.


Build and select the approval CLI from a fresh Tuiscotti clone whose `HEAD` equals `TUISCOTTI_MERGED_SHA`. Set `TUISCOTTI_SOURCE_DIR` to that clone and use `https://github.com/tailrocks/tuiscotti.git` as its repository URL. Use the checkout procedure above with `HEAD_SHA="$TUISCOTTI_MERGED_SHA"`. Warm the clone's locked dependencies before offline builds. Use the selected binary path for every approval and check command. Do not rely on whichever `tuiscotti` executable happens to be first on `PATH`.

```sh
: "${TUISCOTTI_SOURCE_DIR:?set the fresh Tuiscotti source checkout}"
: "${TUISCOTTI_MERGED_SHA:?set the full merged Tuiscotti SHA}"
cd "$TUISCOTTI_SOURCE_DIR"
mkdir -p run-evidence
test "$(git rev-parse HEAD)" = "$TUISCOTTI_MERGED_SHA"
# Review mise.toml before trusting it.
mise trust
mise install
cargo fetch --locked
MISE_CI=1 cargo build --locked --offline --release -p tuiscotti-cli
TARGET_DIR=$(MISE_CI=1 cargo metadata --locked --offline --no-deps --format-version 1 | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')
export TUISCOTTI_CLI="$TARGET_DIR/release/tuiscotti"
test -x "$TUISCOTTI_CLI"
shasum -a 256 "$TUISCOTTI_CLI" | tee run-evidence/tuiscotti-cli.sha256
"$TUISCOTTI_CLI" --help
```

Record the binary's full path and SHA-256 with each receipt. The explicit `-p tuiscotti-cli` build pins the binary to the checked-out merged source. The metadata command selects the Cargo target directory even when an environment wrapper redirects build output. `mise install`, `cargo fetch --locked`, and the explicit CLI build must succeed before you start offline capture or approval. The capture commands below use `mbx` and GNU `timeout`. Install the tool versions declared by each target's `mise.toml`, make `mbx` available, and make GNU Coreutils `timeout` available under the command name `timeout` on macOS. Review and trust each checkout's `mise.toml` before you run `mise install`.


For the local pre-merge mechanics rehearsal only, the reviewed recipe used an immutable candidate source snapshot and a temporary Cargo source patch. That snapshot and patch are local evidence, not a release dependency or durable input. The complete recipe and verified temporary paths remain in the workspace's local-only `.audit/downstream-bulk-runbook-draft.md`; do not assume any `/private/tmp` snapshot still exists. A fresh clone must use the actual merged Tuiscotti SHA instead.

In each TCB disposable copy, add the capture-only behavior below to the relevant support helper. Do not commit this patch. Do not use it outside a disposable capture checkout.

```rust
if !outcome.is_pass() && std::env::var_os("TUISCOTTI_CAPTURE_ONLY").is_some() {
    self.log(format!("capture-only {test_id}: {outcome:?}"));
    return;
}
assert!(outcome.is_pass(), "visual mismatch for {test_id}: {outcome:?}");
```

In TCB main, apply it to `tests/functional/support.rs::App::visual`. In PR #2, apply it to both `App::visual` and `App::compare_frame`. Before the full capture, migrate the two fixtures that failed the initial run. For `g02_harness::harness_changed_baseline_fails`, save a valid temporary `Sample::state("functional/harness_changed_screen", locator_screen(), Observation::empty())` as `functional/harness_changed_screen.tui.pb`; require `MismatchReason::State` and changed fields `cols` and `rows`, never `Missing`. Compare the active baseline tree's per-path digest map before and after. For `g05_reference_session_receipt_records_identity`, derive the expected evidence root from `TUISCOTTI_CAPTURE_ROOT` when set and otherwise retain the existing `target/functional` default; continue asserting the evidence path is outside baselines. These fixture fixes passed their focused two-test check in a disposable copy, but the full suite remains incomplete. Keep the existing cursor-only fast path. Also add an opt-in `TUISCOTTI_CAPTURE_ROOT` override to the evidence-root helper. Main's `evidence_root()` returns `PathBuf::from(root)` when the variable is set. PR #2's `evidence_directory()` returns `PathBuf::from(root).join(format!("{name}-{}-{nanos}-{seq}", std::process::id()))` when set. Otherwise, each helper keeps its existing default. The exact path behavior was smoke-tested in the disposable copies described in the audit draft.

This override changes the evidence location and the visual mismatch assertion only. PTY startup, interactions, semantic assertions, sample serialization, and evidence writes still run. Capture-only suppresses a failed visual assertion, so a successful Nextest exit does not prove candidate coverage or approval.

## Capture TCB main

Run this in a fresh disposable TCB main checkout after applying the opt-in support changes. The `run-evidence` directory is inside that disposable checkout and outside the active baseline root. Preserve it with the checkout's logs.

```sh
mkdir -p run-evidence
if [ -e run-evidence/capture ] || [ -L run-evidence/capture ]; then
  printf '%s\n' 'capture root already exists; use a fresh disposable copy' >&2
  exit 1
fi
export TUISCOTTI_CAPTURE_ROOT="$PWD/run-evidence/capture"
set +e
TUISCOTTI_CAPTURE_ONLY=1 MISE_CI=1 timeout 230 mbx nextest run --offline \
  -p junie-tui --test functional --no-tests fail --no-capture \
  >run-evidence/functional-capture.log 2>&1
capture_status=$?
set -e
printf '%s\n' "$capture_status" >run-evidence/functional-capture.exit
```

This is the narrowed producer target. The broader `--workspace --test functional` selection fails target selection because `jackin-console` requires feature `demo`. A 230-second timeout is a hard bound, not a runtime estimate. Preserve logs and candidate evidence after any failure or timeout.

The verified macOS run completed 117 of 117 tests and yielded 101 `CAPTURED`, one `UNEXPECTED`, zero `MISSING`, and zero `AMBIGUOUS` active functional IDs. Repeat the capture on every required supported OS and architecture. Preserve the test-ID grammar and the `state-and-pixels` platform suffix rules in [SNAPSHOTS.md](SNAPSHOTS.md).

## Capture TCB PR #2

Run this in its own fresh disposable checkout. Keep the active baselines, frozen `pre_refactor` records, receipts, and coverage fixtures unchanged. Its full capture command was attempted and timed out; use the partitions below to preserve coverage evidence. Each partition needs a new root.

```sh
cd /path/to/fresh/tcb-pr-2
root="$PWD/run-evidence/active-showcase"; if [ -e "$root" ] || [ -L "$root" ]; then exit 2; fi; mkdir -p "$root"
set +e
TUISCOTTI_CAPTURE_ROOT="$root" MISE_CI=1 TUISCOTTI_CAPTURE_ONLY=1 timeout 230 mbx nextest run --offline --workspace --test functional --no-tests fail -E 'test(/^showcase::/)' --no-capture > "$root/nextest.log" 2>&1
capture_status=$?
set -e
printf '%s\n' "$capture_status" > "$root/nextest.exit"
root="$PWD/run-evidence/active-holla"; if [ -e "$root" ] || [ -L "$root" ]; then exit 2; fi; mkdir -p "$root"
set +e
TUISCOTTI_CAPTURE_ROOT="$root" MISE_CI=1 TUISCOTTI_CAPTURE_ONLY=1 timeout 230 mbx nextest run --offline --workspace --test functional --no-tests fail -E 'test(/^holla::/)' --no-capture > "$root/nextest.log" 2>&1
capture_status=$?
set -e
printf '%s\n' "$capture_status" > "$root/nextest.exit"
root="$PWD/run-evidence/active-tablepro"; if [ -e "$root" ] || [ -L "$root" ]; then exit 2; fi; mkdir -p "$root"
set +e
TUISCOTTI_CAPTURE_ROOT="$root" MISE_CI=1 TUISCOTTI_CAPTURE_ONLY=1 timeout 230 mbx nextest run --offline --workspace --test functional --no-tests fail -E 'test(/^tablepro::/)' --no-capture > "$root/nextest.log" 2>&1
capture_status=$?
set -e
printf '%s\n' "$capture_status" > "$root/nextest.exit"
root="$PWD/run-evidence/active-jackin"; if [ -e "$root" ] || [ -L "$root" ]; then exit 2; fi; mkdir -p "$root"
set +e
TUISCOTTI_CAPTURE_ROOT="$root" MISE_CI=1 TUISCOTTI_CAPTURE_ONLY=1 timeout 230 mbx nextest run --offline --workspace --test functional --no-tests fail -E 'test(/^jackin::/)' --no-capture > "$root/nextest.log" 2>&1
capture_status=$?
set -e
printf '%s\n' "$capture_status" > "$root/nextest.exit"
root="$PWD/run-evidence/active-lifecycle"; if [ -e "$root" ] || [ -L "$root" ]; then exit 2; fi; mkdir -p "$root"
set +e
TUISCOTTI_CAPTURE_ROOT="$root" MISE_CI=1 TUISCOTTI_CAPTURE_ONLY=1 timeout 230 mbx nextest run --offline --workspace --test functional --no-tests fail -E 'test(/^lifecycle::/)' --no-capture > "$root/nextest.log" 2>&1
capture_status=$?
set -e
printf '%s\n' "$capture_status" > "$root/nextest.exit"
```

The recorded partition runs completed showcase 59/59, holla 11/11, tablepro 11/11, jackin 10/10, and lifecycle 2/2. They yielded 104 candidates for 109 active IDs. Five old IDs still lack candidates. Some other candidate IDs differ from the old names; do not equate, rename, delete, or approve those entries automatically.

### Inventory and preserve candidates

For a single TCB main run, reconcile captured `sample.tui.pb` files against only the active functional JSON inventory. Set `EVIDENCE_ROOT` to that run's capture root and `ARTIFACT_MARKER=artifacts`. Set `OUT` to a new path outside the active baseline root. The following script copies only unambiguous active IDs and records missing, duplicate, or out-of-scope IDs. It does not decode or approve samples. For PR #2, run the five partitions above and use the multi-root reconciliation below instead.

```sh
: "${EVIDENCE_ROOT:?set isolated capture root}"
: "${ARTIFACT_MARKER:?set to artifacts for main or check-artifacts for PR #2}"
export ACTIVE="${ACTIVE:-tests/functional/baselines}"
export OUT="${OUT:-$PWD/run-evidence/candidates}"
export EVIDENCE_ROOT ARTIFACT_MARKER
python3 - <<'PY'
import hashlib, os, shutil
from pathlib import Path

active = Path(os.environ["ACTIVE"])
functional = active / "functional"
out = Path(os.environ["OUT"])
root = Path(os.environ["EVIDENCE_ROOT"])
marker = os.environ["ARTIFACT_MARKER"]
if out.exists() or out.is_symlink():
    raise SystemExit(f"candidate output already exists: {out}")
out.mkdir(parents=True, exist_ok=True)
expected = {p.relative_to(active).as_posix()[:-len(".tui.json")] for p in functional.rglob("*.tui.json")}
found = {}
for sample in root.rglob("sample.tui.pb") if root.exists() else ():
    parts = sample.relative_to(root).parts
    if marker not in parts:
        continue
    i = parts.index(marker)
    if len(parts) < i + 4:
        continue
    test_id = "/".join(parts[i + 1:-2])
    digest = hashlib.sha256(sample.read_bytes()).hexdigest()
    found.setdefault(test_id, {}).setdefault(digest, sample)

with (out / "inventory.tsv").open("w") as f:
    f.write("status\ttest_id\tsha256\tsource\n")
    for test_id in sorted(expected | set(found)):
        rows = found.get(test_id, {})
        if not rows:
            f.write(f"MISSING\t{test_id}\t\t\n")
        elif len(rows) != 1:
            hashes = ",".join(sorted(rows))
            f.write(f"AMBIGUOUS\t{test_id}\t{hashes}\t\n")
        elif test_id not in expected:
            digest, source = next(iter(rows.items()))
            f.write(f"UNEXPECTED\t{test_id}\t{digest}\t{source}\n")
        else:
            digest, source = next(iter(rows.items()))
            dest = out / (test_id + ".tui.pb")
            dest.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, dest)
            if hashlib.sha256(dest.read_bytes()).hexdigest() != digest:
                raise SystemExit(f"copy digest mismatch: {test_id}")
            f.write(f"CAPTURED\t{test_id}\t{digest}\t{source}\n")
PY
```

For PR #2, reconcile all five partition roots together. The script checks the binary magic, preserves every raw sample, and copies only one distinct digest for an expected active functional ID. Set `OUT` to a new absent destination if you rerun it.

```sh
cd /path/to/fresh/tcb-pr-2
OUT="$PWD/run-evidence/active-candidates-reconciled"
export OUT
python3 - <<'PY'
import hashlib, os, shutil
from pathlib import Path
active = Path('tests/functional/baselines')
roots = [Path('run-evidence') / f'active-{n}' for n in ('showcase', 'holla', 'tablepro', 'jackin', 'lifecycle')]
out = Path(os.environ['OUT'])
if out.exists() or out.is_symlink():
    raise SystemExit(f'refusing existing output: {out}')
out.mkdir(parents=True)
expected = {p.relative_to(active).as_posix()[:-len('.tui.json')] for p in (active / 'functional').rglob('*.tui.json')}
found = {}
for root in roots:
    for sample in root.rglob('sample.tui.pb'):
        data = sample.read_bytes()
        if not data.startswith(b'TUISCPB\0'):
            raise SystemExit(f'bad binary magic: {sample}')
        parts = sample.relative_to(root).parts
        if 'check-artifacts' not in parts:
            continue
        i = parts.index('check-artifacts')
        if len(parts) < i + 4:
            continue
        test_id = '/'.join(parts[i + 1:-2])
        digest = hashlib.sha256(data).hexdigest()
        found.setdefault(test_id, {}).setdefault(digest, sample)
with (out / 'inventory.tsv').open('w') as f:
    f.write('status\ttest_id\tsha256\tsource\n')
    for test_id in sorted(expected | set(found)):
        rows = found.get(test_id, {})
        if not rows:
            f.write(f'MISSING\t{test_id}\t\t\n')
        elif len(rows) != 1:
            f.write(f"AMBIGUOUS\t{test_id}\t{','.join(sorted(rows))}\t\n")
        elif test_id not in expected:
            digest, source = next(iter(rows.items()))
            f.write(f'UNEXPECTED\t{test_id}\t{digest}\t{source}\n')
        else:
            digest, source = next(iter(rows.items()))
            dest = out / (test_id + '.tui.pb')
            dest.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, dest)
            if hashlib.sha256(dest.read_bytes()).hexdigest() != digest:
                raise SystemExit(f'copy digest mismatch: {test_id}')
            f.write(f'CAPTURED\t{test_id}\t{digest}\t{source}\n')
PY
awk -F '\t' 'NR>1 { n[$1]++ } END { for (s in n) print s, n[s] }' "$OUT/inventory.tsv" | sort
```

Read every `MISSING`, `AMBIGUOUS`, and `UNEXPECTED` row. Do not claim full capture until every expected active ID has exactly one candidate and all expected producer tests complete on every required platform. Keep the raw artifact roots and logs. For PR #2, do not treat the `pre_refactor` JSON tree as active candidates.

For PR #2, the five unresolved ID pairs are:

| Existing active ID without a candidate | Current captured ID | Producer test |
|---|---|---|
| `functional/jackin_launch_failure` | `functional/jackin_launch_failed` | `jackin_cockpit_launch_failure_acknowledges` |
| `functional/jackin_manager_stop_dialog` | `functional/jackin_stop_instance_dialog` | `jackin_manager_stop_cancel_then_confirm` |
| `functional/jackin_settings_leave` | `functional/jackin_settings_dirty_leave` | `jackin_settings_unsaved_leave_cancel_then_discard` |
| `functional/showcase_keyboard_activation` | `functional/showcase_keyboard_run_task` | `showcase_keyboard_activates_focused_run_task` |
| `functional/tablepro_query_error_recovery` | `functional/tablepro_query_recovered` | `tablepro_query_select_results_and_error_recovery` |

Preserve all five old files and all new candidates while the human migration owner reviews them. The captured states differ materially from the old states. The checked PR #2 dispositions list these as superseded, but that record is evidence for review, not authorization to rename or delete. The migration owner must decide whether to adopt the dispositions.

Before PR #2 approval, refresh and validate its reference manifest against the actual capture source, current lockfile, comparator, and merged Tuiscotti SHA. Its existing wrapper receipts would otherwise contain misleading provenance. Run `python3 scripts/accept-baseline.py --help` and follow the wrapper's current interface. Keep receipts outside the baseline root. Update `scripts/check-refactor-coverage.py` and its tests to count `.tui.pb` IDs while keeping receipt and metadata JSON distinct. Use the current workflow invocation for production checker arguments; the draft does not provide a complete current invocation. Do not treat `python3 scripts/check-refactor-coverage.py --fixtures scripts/coverage-fixtures` as the production coverage run.

## Stage human-reviewed replacements

Do not convert an old JSON file. Capture fresh binary candidates, then compare each candidate's decoded state and rendered evidence with the old approved sample and intended product behavior. Review every candidate individually. Record its test ID, source checkout SHA, platform, capture command, executable hash, candidate path, candidate SHA-256, review result, and reviewer.

For each explicitly approved candidate, use a separate empty staging baseline root. On a workstation, unset `CI`, `TUISCOTTI_CAPTURE_ONLY`, and `TUISCOTTI_CAPTURE_ROOT`. Approve each candidate through the documented CLI or target wrapper. For the first binary write into an empty root, use `--expect-baseline missing`; record the old JSON digest as provenance only. If replacing a `.tui.pb` already in staging, pass that staged file's exact current digest. Never batch-accept or call `accept` from tests or CI.

For TCB main, the documented approval interface is:

```sh
"$TUISCOTTI_CLI" accept --candidate <sample> --candidate-sha256 <sha256> \
  --expect-baseline missing --baselines <staging-root>
```

For PR #2, use `scripts/accept-baseline.py`; refresh the manifest first. Pass `--tuiscotti "$TUISCOTTI_CLI"` on every wrapper invocation because its default can select an older binary. Follow the current `--help` output for the other per-candidate arguments. After every approval, independently compare the candidate file and installed staged `.tui.pb` bytes and SHA-256. Receipt presence alone does not prove byte equality. Check receipt fields against the reviewed manifest. Require candidate SHA-256 to equal installed baseline SHA-256 for every ID.

Run `"$TUISCOTTI_CLI" check` against each staged candidate and staged baseline root. Then create a second fresh disposable validation checkout for each target from the downstream migration candidate under review. In that copy, repeat the exact merged Tuiscotti dependency pin and lockfile update. Apply the binary-fixture, coverage-checker, identity-test, and generated-CI migrations under review before running tests. Place the staged `.tui.pb` files at the baseline paths that the tests read, or use a separately reviewed temporary root override. TCB functional tests and Jackin identity tests use hardcoded baseline roots; a staging directory alone does not make those suites read staged samples. Keep this validation copy separate from capture and approval work. Run the target's full functional, identity, coverage, and repository CI gates on each required platform. Mark the validation incomplete until those commands finish successfully. For PR #2, copy the exact `scripts/check-refactor-coverage.py` production invocation from the current workflow and provide the current inventory, Nextest list, dispositions, receipts, and staged baseline path. Review all additions, removals, and visual changes before moving staged files into a downstream review.

## Migrate Jackin's identity coverage

Jackin consumes TCB's selected `functional` baseline root. It does not own or receive a copy of those samples. Decide explicitly which TCB revision Jackin will consume. If Jackin keeps the checked `a901c275055091810ae0d0ac30ec5fc09ce595d9` source pin, generate and approve against that revision. If it adopts another approved TCB revision, update the pin in a separate downstream change and generate against that exact revision.

Update `crates/jackin-testkit/tests/identity/baseline.rs` and `coverage.rs` to recognize `.tui.pb`. Replace the raw-JSON color mutation with a valid temporary binary sample and preserve the changed-cell assertion. Keep the journey-to-visual-ID checks, approved-file inventory, and four no-file scenarios. Run the identity coverage test after the selected TCB baseline root has staged `.tui.pb` files. The recorded one-sample `functional/jackin_launch_reconnect` smoke is not full Jackin coverage. At the checked Jackin head, 54 of its 55 distinct visual IDs remain uncaptured. Do not report Jackin coverage complete until all required IDs have reviewed binary files and the identity tests pass.

## Promote and roll back

Promote only the individually reviewed, staged `.tui.pb` files and the necessary downstream code, manifest, coverage, and generated CI updates in a downstream review. Preserve JSON history and PR #2 `pre_refactor` records. Regenerate generated CI from its source configuration; do not hand-edit generated paths. Do not update the downstream pin to an unreleased Tuiscotti commit.

If a promoted downstream change needs rollback, revert that downstream commit so the prior baseline paths and exact dependency pin return together. Do not delete files or assume that replacing a baseline directory is atomic. Keep the previous Git revision and lockfile available until the new pin and baseline review pass.

## Evidence and limits

The dated evidence above came from disposable macOS worktrees. Fresh downstream head queries were repeated read-only on 2026-11-06, but no fresh-head capture was run while preparing this portable document. It did not modify a downstream branch, active baseline, published pin, or PR. The single-sample local accepts in the older mechanics smoke were assistant-run staging actions, not the required explicit human approval. No complete PR #2 rehearsal passed: the full command timed out, 104 of 109 candidates were reconciled, five IDs remain unresolved, and the PR #2 coverage checker has not been migrated to `.tui.pb`. Jackin's full identity coverage and all required cross-platform lanes remain **not verified**. Native Linux remains **not verified**.

The reviewed local audit draft preserves the validated capture-only source patch, exact partition results, artifact-root smoke, and failure analysis. Treat that draft and its temporary paths as local evidence, not as portable inputs or prerequisites for these post-merge steps.
