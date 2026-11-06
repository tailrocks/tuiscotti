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

The initial read-only recheck returned TCB main `7f385611fba0f8d4a77bb02e0a991ad660b48746`, TCB PR #2 `60b5fb2762d91a3549bd4ac63b1d77d534ee53d2` (open, dirty), and Jackin PR #1264 head `480ba8faab70e87d9c276da6ebd3985c8f92c258` (open, blocked). Later dated rehearsal evidence used Jackin head `27ec25ec0aa180af9bf075660af5cbf55dccefc3`, then `7198da57fa02666df3d14adfae665f03e0799913`. A fresh read-only recheck for this runbook update returned TCB main `7f385611fba0f8d4a77bb02e0a991ad660b48746`, TCB PR #2 `60b5fb2762d91a3549bd4ac63b1d77d534ee53d2` (open, dirty), and Jackin PR #1264 `7198da57fa02666df3d14adfae665f03e0799913` (open, blocked). These are dated observations, not a substitute for recording the full SHA returned by a new recheck before every run.

| Target | Baseline ownership and current pins at the checked head | Inventory and verified capture status |
|---|---|---|
| TCB main | Owns `tests/functional/baselines`. The dated checked head is `7f385611fba0f8d4a77bb02e0a991ad660b48746`; the exact-source rehearsal used Tuiscotti `59704edfff6a6676e8833f190dcc43dccc528329`. | Corrected inventory has 102 tracked producer IDs: 101 with one candidate and one (`functional/showcase_page_diff`) with two timestamp-different candidates; none missing. The formerly unexpected `showcase/buttons/held` is a tracked ID. All 117 producer tests passed across 12 normal-assertion groups. This is candidate-generation evidence, not visual approval; see evidence notes below. |
| TCB PR #2 | Owns active and frozen baseline trees. Dated checked head: `60b5fb2762d91a3549bd4ac63b1d77d534ee53d2`; approval wrapper is `scripts/accept-baseline.py`. | The earlier 105-fixture/93-test rehearsal and separate held-button check were later followed by a current-source integrated rehearsal: 106 temporary fixtures and 94 selected producer tests passed together, using local TCB preparation `6ee587e601c2a69f3540da3924720b2863d19d7b` and Tuiscotti `006c8f952080a51ed3b25a1535e6d412e944d052`. Five legacy ID disposition cases remain unresolved. These are rehearsal fixtures, not approved baselines. |
| Jackin PR #1264 | Owns no sample files. The latest live head checked for this update is `7198da57fa02666df3d14adfae665f03e0799913`; its identity suite reads `functional` baselines from TCB. | A local binary-migration preparation based on that head, ending at `41dd7372175f9720eb5eaac6c0538cdc84bea181`, passed 47 identity tests and five focused Muse account tests using Tuiscotti `006c8f9` and 55 unapproved binary fixture overlays. The published PR is not thereby migrated; this is not full Jackin validation or approval. |

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

In TCB main, apply it to `tests/functional/support.rs::App::visual`. In PR #2, apply it to both `App::visual` and `App::compare_frame`. Before the initial capture attempt, migrate the two fixtures that failed that run. For `g02_harness::harness_changed_baseline_fails`, save a valid temporary `Sample::state("functional/harness_changed_screen", locator_screen(), Observation::empty())` as `functional/harness_changed_screen.tui.pb`; require `MismatchReason::State` and changed fields `cols` and `rows`, never `Missing`. Compare the active baseline tree's per-path digest map before and after. For `g05_reference_session_receipt_records_identity`, derive the expected evidence root from `TUISCOTTI_CAPTURE_ROOT` when set and otherwise retain the existing `target/functional` default; continue asserting the evidence path is outside baselines. These fixture fixes passed their focused two-test check in a disposable copy. This statement concerns the earlier broad capture attempt only; the separate final-source TCB rehearsal later passed 117 producer tests in bounded validation groups, as summarized below. Keep the existing cursor-only fast path. Also add an opt-in `TUISCOTTI_CAPTURE_ROOT` override to the evidence-root helper. Main's `evidence_root()` returns `PathBuf::from(root)` when the variable is set. PR #2's `evidence_directory()` returns `PathBuf::from(root).join(format!("{name}-{}-{nanos}-{seq}", std::process::id()))` when set. Otherwise, each helper keeps its existing default. The exact path behavior was smoke-tested in the disposable copies described in the audit draft.

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

An earlier macOS capture reported 101 `CAPTURED` and one `UNEXPECTED` ID, but its tracked-ID scan omitted the nested showcase fixture and its source was not the final `59704ed` rehearsal source. Use the corrected 102-ID reconciliation and final-source producer validation in the evidence notes below for current counts. Repeat capture on every required supported OS and architecture. Preserve the test-ID grammar and the `state-and-pixels` platform suffix rules in [SNAPSHOTS.md](SNAPSHOTS.md).

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
root="$PWD/run-evidence/active-visual"; if [ -e "$root" ] || [ -L "$root" ]; then exit 2; fi; mkdir -p "$root"
set +e
TUISCOTTI_CAPTURE_ROOT="$root" MISE_CI=1 TUISCOTTI_CAPTURE_ONLY=1 timeout 230 mbx nextest run --offline --workspace --test functional --no-tests fail -E 'test(/^visual::/)' --no-capture > "$root/nextest.log" 2>&1
capture_status=$?
set -e
printf '%s\n' "$capture_status" > "$root/nextest.exit"
```

The earlier five partition runs completed showcase 59/59, holla 11/11, tablepro 11/11, jackin 10/10, and lifecycle 2/2. They yielded 104 candidates for 110 tracked active IDs: 109 functional IDs plus nested `showcase/buttons/held`. The subsequent 93-test integrated validation covered selected producer tests against a 105-fixture temporary composite; it omitted `visual::showcase_held_button_matches_frozen_tuiscotti_baseline`. A separate run then captured that held-button candidate and passed its focused assertion 1/1. Those older results were separate runs. They were superseded as a combined fixture/producer check by the current-source rehearsal documented below, which assembled all 106 temporary fixtures and passed 94 selected producers together. This does not resolve the five legacy ID disposition cases or approve candidates. Keep the bounded `visual::` partition above for future reruns and full-root reconciliation. Some captured IDs differ from old names; do not equate, rename, delete, or approve those entries automatically.

### Inventory and preserve candidates

For a single TCB main run, reconcile captured `sample.tui.pb` files against every recursively tracked fixture under the active baseline root, including nested paths such as `showcase/buttons/held`. Set `EVIDENCE_ROOT` to that run's capture root and `ARTIFACT_MARKER=artifacts`. Set `OUT` to a new path outside the active baseline root. The script copies only unambiguous tracked IDs and records missing, duplicate, or untracked IDs. It does not decode or approve samples. For PR #2, run all six partitions above and use the multi-root reconciliation below instead; that inventory excludes frozen `pre_refactor` records.

```sh
: "${EVIDENCE_ROOT:?set isolated capture root}"
: "${ARTIFACT_MARKER:?set to artifacts for main or check-artifacts for PR #2}"
export ACTIVE="${ACTIVE:-tests/functional/baselines}"
export OUT="${OUT:-$PWD/run-evidence/candidates}"
export EVIDENCE_ROOT ARTIFACT_MARKER
python3 - <<'PY'
import hashlib, os, shutil, subprocess
from pathlib import Path

active = Path(os.environ["ACTIVE"])
out = Path(os.environ["OUT"])
root = Path(os.environ["EVIDENCE_ROOT"])
marker = os.environ["ARTIFACT_MARKER"]
if out.exists() or out.is_symlink():
    raise SystemExit(f"candidate output already exists: {out}")
out.mkdir(parents=True, exist_ok=True)
tracked = subprocess.check_output(["git", "ls-files", "-z", "--", active.as_posix()]).split(b"\0")
expected = set()
for raw_path in tracked:
    if not raw_path:
        continue
    path = Path(os.fsdecode(raw_path))
    if path.suffixes[-2:] != [".tui", ".json"]:
        continue
    relative = path.relative_to(active)
    if "pre_refactor" in relative.parts:
        continue
    expected.add(relative.as_posix()[:-len(".tui.json")])
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

For PR #2, reconcile all six partition roots together from the PR checkout root. The script checks the binary magic and copies only one distinct digest for a tracked active sample ID; frozen `pre_refactor` fixtures are excluded. Raw candidates remain in the input roots. Set `OUT` to a new absent destination if you rerun it.

```sh
cd /path/to/fresh/tcb-pr-2
OUT="$PWD/run-evidence/active-candidates-reconciled"
export OUT
python3 - <<'PY'
import hashlib, os, shutil, subprocess
from pathlib import Path
active = Path('tests/functional/baselines')
roots = [Path('run-evidence') / f'active-{n}' for n in ('showcase', 'holla', 'tablepro', 'jackin', 'lifecycle', 'visual')]
out = Path(os.environ['OUT'])
if out.exists() or out.is_symlink():
    raise SystemExit(f'refusing existing output: {out}')
out.mkdir(parents=True)
tracked = subprocess.check_output(['git', 'ls-files', '-z', '--', active.as_posix()]).split(b'\0')
expected = set()
for raw_path in tracked:
    if not raw_path:
        continue
    path = Path(os.fsdecode(raw_path))
    if path.suffixes[-2:] != ['.tui', '.json']:
        continue
    relative = path.relative_to(active)
    if 'pre_refactor' in relative.parts:
        continue
    expected.add(relative.as_posix()[:-len('.tui.json')])
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

In the prior five-partition composite, these six tracked active IDs lacked candidates. At that point the nested showcase ID had only been captured separately. The later current-source integrated rehearsal assembled it with the other 105 fixtures and validated its producer in the same overall rehearsal. The five legacy ID changes remain unresolved and require disposition:

| Prior composite: tracked active ID without a candidate | Candidate ID in later evidence | Producer test |
|---|---|---|
| `functional/jackin_launch_failure` | `functional/jackin_launch_failed` | `jackin_cockpit_launch_failure_acknowledges` |
| `functional/jackin_manager_stop_dialog` | `functional/jackin_stop_instance_dialog` | `jackin_manager_stop_cancel_then_confirm` |
| `functional/jackin_settings_leave` | `functional/jackin_settings_dirty_leave` | `jackin_settings_unsaved_leave_cancel_then_discard` |
| `functional/showcase_keyboard_activation` | `functional/showcase_keyboard_run_task` | `showcase_keyboard_activates_focused_run_task` |
| `functional/tablepro_query_error_recovery` | `functional/tablepro_query_syntax_error` | `tablepro_query_select_results_and_error_recovery` |
| `showcase/buttons/held` | separately captured as `showcase/buttons/held.tui.pb` (not in the composite; review required) | `visual::showcase_held_button_matches_frozen_tuiscotti_baseline` |

Preserve all five legacy files and all new candidates while the human migration owner reviews them. The separately captured held-button candidate also remains unapproved. For TablePro, `functional/tablepro_query_syntax_error` captures the unrecovered error checkpoint and `functional/tablepro_query_recovered` remains a separate checkpoint after the corrected query; do not map the old error ID to the recovery sample. The captured states differ materially from the old states. The checked PR #2 dispositions list these as superseded, but that record is evidence for review, not authorization to rename or delete. The migration owner must decide whether to adopt the dispositions.

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

Run `"$TUISCOTTI_CLI" check` against each staged candidate and staged baseline root. Then create a second fresh disposable validation checkout for each target from the downstream migration candidate under review. In that copy, repeat the exact merged Tuiscotti dependency pin and lockfile update. Apply the binary-fixture, coverage-checker, identity-test, and generated-CI migrations under review before running tests. Install the approved candidates through acceptance at the baseline paths the tests read, or use a separately reviewed temporary root override pointing to the already accepted staging root. Verify the resulting bytes against the approved candidates. TCB functional tests and Jackin identity tests use hardcoded baseline roots; a staging directory alone does not make those suites read staged samples. Keep this validation copy separate from capture and approval work. Run the target's full functional, identity, coverage, and repository CI gates on each required platform. Mark the validation incomplete until those commands finish successfully. For PR #2, copy the exact `scripts/check-refactor-coverage.py` production invocation from the current workflow and provide the current inventory, Nextest list, dispositions, receipts, and staged baseline path. Review all additions, removals, and visual changes before moving staged files into a downstream review.

## Migrate Jackin's identity coverage

Jackin consumes TCB's selected `functional` baseline root. It does not own or receive a copy of those samples. Decide explicitly which TCB revision Jackin will consume. If Jackin keeps the checked `a901c275055091810ae0d0ac30ec5fc09ce595d9` source pin, generate and approve against that revision. If it adopts another approved TCB revision, update the pin in a separate downstream change and generate against that exact revision.

Update `crates/jackin-testkit/tests/identity/baseline.rs` and `coverage.rs` to recognize `.tui.pb`. Replace the raw-JSON color mutation with a valid temporary binary sample and preserve the changed-cell assertion. Keep the journey-to-visual-ID checks, approved-file inventory, and four no-file scenarios. Run the identity coverage test after the selected TCB baseline root has staged `.tui.pb` files. The latest local overlay rehearsal generated and matched scratch candidates for all 55 distinct IDs and passed 47 identity tests, but this does not validate the published JSON-based PR or approve samples. Do not report the downstream migration complete until reviewed binary files are installed through the approval path and the full required test gates pass.

## Promote and roll back

In a separate disposable migration checkout, install each individually approved candidate into the destination baseline root through `tuiscotti accept` or the repository approval wrapper. Use the candidate’s exact SHA-256 and `--expect-baseline missing` for a new binary path; if a binary baseline already exists, supply its exact current digest. Verify the installed bytes and digest against the approved candidate. Do not copy staged binaries directly into the destination baseline root. Only after every destination binary is installed and validated, remove the superseded active JSON files as an explicit reviewed repository change. Include the necessary downstream code, manifest, coverage, and generated CI updates in that review. Preserve JSON history and PR #2 `pre_refactor` records. Regenerate generated CI from its source configuration; do not hand-edit generated paths. Do not update the downstream pin to an unreleased Tuiscotti commit.

If a promoted downstream change needs rollback, revert that downstream commit so the prior baseline paths and exact dependency pin return together. Do not delete files or assume that replacing a baseline directory is atomic. Keep the previous Git revision and lockfile available until the new pin and baseline review pass.

## Evidence and limits

The dated evidence in this runbook is local rehearsal evidence from disposable worktrees and overlays. It did not change a downstream PR head, published dependency pin, or active baseline. Candidate generation, fixture comparison, and passing tests do not imply visual review, approval, or permission to promote any sample. All generated candidate sets remain unapproved.

The final-source TCB main run used Tuiscotti `59704edfff6a6676e8833f190dcc43dccc528329` and passed 117 distinct producer tests. Its corrected inventory is 102 tracked IDs (101 single-candidate IDs and one two-candidate ambiguity), with no missing IDs. The two `functional/showcase_page_diff` candidates differ only in capture timestamp according to the prior state/render comparison; because that comparator result comes from a separate rehearsal, treat equivalence as a caveat and retain both for review. The old source-mismatched rehearsal at Tuiscotti `d31900993b533ad4758848c261b5f4c5b2eb1cee` and earlier `818b2ebfd94cfea669a6603b79cb96532db9b282` rehearsal are not evidence for the final `59704ed` source.

The earlier PR #2 integrated checkpoint added a distinct TablePro syntax-error checkpoint while retaining recovery coverage. Its 93/93 validation used 105 unapproved temporary fixtures, with capture-only unset; six tracked active IDs were absent from that composite, including nested `showcase/buttons/held`. The later current-source rehearsal assembled 106 unapproved temporary fixtures and passed 94 selected producer tests together, including the held-button test. Five old IDs remain unmatched and require disposition. This selected producer run is not the full active producer set or the production coverage gate, and no candidate was approved. Jackin's former 47/47 rehearsal on PR head `27ec25e` was superseded by the later head `7198da57` rehearsal described below; neither is full Jackin validation or evidence of sample approval.

The held-button ID was initially captured in a separate isolated worktree at TCB checkpoint `6ee587e601c2a69f3540da3924720b2863d19d7b`, using Tuiscotti source `59704edfff6a6676e8833f190dcc43dccc528329`. The fresh producer emitted candidate `captured/functional/showcase/buttons/held.tui.pb` with SHA-256 `f75af1e12d8eddfe59126d504587105f354118615ea5d5c1c6c031de95cdcbbc`; the focused assertion passed 1/1. This original candidate and check remain historical evidence. The later current-source rehearsal assembled this held fixture with 105 others and passed all 94 selected producer assertions against the combined fixture root. Its evidence README is `/Users/donbeave/Projects/tailrocks/tuiscotti/.audit/rollout-review-index-20261106/pr2-final-source-006c8f9/README.md`; it records TCB preparation `6ee587e601c2a69f3540da3924720b2863d19d7b`, Tuiscotti `006c8f952080a51ed3b25a1535e6d412e944d052`, and unchanged hashes for 1,978 active baseline files. No acceptance or approval command ran.

The local TCB PR #2 preparation used here is based on PR head `60b5fb2762d91a3549bd4ac63b1d77d534ee53d2`; its Holla formatting checkpoint is commit `6ee587e601c2a69f3540da3924720b2863d19d7b`, local evidence and not the remote PR head. The earlier Jackin preparation based on `27ec25ec0aa180af9bf075660af5cbf55dccefc3` ended at `5b7f8232bef4ae372f915b26ea61a0f955bbfb26`; that result is superseded for latest-head evidence by the `7198da57` preparation below. These local identities do not transfer ownership of downstream branches or imply a push, merge, approval, or promotion.

The 117-test TCB result is a producer rehearsal on the recorded native arm64 environment, not validation of all required OS/architecture suffix lanes. Native Linux and other required platform lanes remain **not verified**. Full PR #2 production coverage, human review, and approval remain outstanding. See the local review index and evidence bundles for provenance; their temporary source paths and overlays are not portable inputs.

The current-source PR #2 report confirms 106 fixture files assembled into one isolated root and 94 distinct selected producer tests passing against that root, with capture bypasses unset. It does not cover all producers or resolve the five unmatched old IDs. The latest Jackin local preparation is based on live head `7198da57fa02666df3d14adfae665f03e0799913`, ending at `41dd7372175f9720eb5eaac6c0538cdc84bea181`, and passed 47 identity tests plus five focused Muse account tests using Tuiscotti `006c8f9` and 55 unapproved binary fixture overlays. The evidence report is `/Users/donbeave/Projects/tailrocks/tuiscotti/.audit/rollout-review-index-20261106/jackin-7198-identity/identity-refresh.report.txt`. This remains local overlay evidence: it does not complete full Jackin validation, approve fixtures, or show that the live PR contains the preparation. The reviewed local audit draft preserves earlier capture mechanics and failure analysis; its temporary paths are evidence references, not portable inputs or prerequisites for post-merge steps.
