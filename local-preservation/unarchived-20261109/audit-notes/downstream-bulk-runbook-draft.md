# Downstream compressed baseline regeneration runbook

Updated 2026-11-06 from `.audit/downstream-runbook-draft.md`; local candidate snapshot verified read-only. This is an execution recipe, not evidence of full regeneration. No downstream ref, active baseline, published pin, PR head or remote was changed.

## Read-only inputs and live heads

- Local Tuiscotti candidate source: `/private/tmp/tuiscotti-continuation-snapshot` (immutable snapshot of local candidate `d31900993b533ad4758848c261b5f4c5b2eb1cee`). Its `SHA256SUMS` manifest SHA-256 is `f431296cbbc15d8c83ffd44eeaf2d68bbd6c6df116dfdca56d58fb3983860805`; `shasum -a 256 -c SHA256SUMS` passes all listed files.
- Current live heads rechecked read-only with `gh`: terminal-components-baseline `main` `7f385611fba0f8d4a77bb02e0a991ad660b48746`; PR #2 `60b5fb2762d91a3549bd4ac63b1d77d534ee53d2` (OPEN, DIRTY); Jackin PR #1264 `480ba8faab70e87d9c276da6ebd3985c8f92c258` (OPEN, BLOCKED).
- `mise --version` is `2026.11.1`, `mbx --version` is `1.21.1`, and `mise exec -- cargo --version` reports the active wrapped Cargo `1.99.1`. Use `MISE_CI=1 mbx ...` for every Cargo command in the rehearsal copies.
- The earlier bounded macOS evidence remains useful as a mechanics smoke: one fresh candidate per target, candidate bytes preserved in isolated staging, local disposable acceptance, post-accept digest equality, and CLI check pass. Those assistant-run disposable accepts are not the required explicit human workstation approval and did not exercise bulk coverage.

## Capture-only behavior required for a real bulk run

In each disposable TCB copy, retain other Cargo source patches and add or replace only this source override (verify the effective `.cargo/config.toml` before building):

```toml
[patch."https://github.com/tailrocks/tuiscotti"]
tuiscotti = { path = "/private/tmp/tuiscotti-continuation-snapshot/crates/tuiscotti" }
```

The two terminal-components-baseline revisions write candidate evidence to different locations. Main's default `evidence_root()` is `parent(CARGO_TARGET_TMPDIR)/functional`, and its `FrozenStore` artifacts are under `<evidence-root>/artifacts/<test-id>/<candidate-sha>/sample.tui.pb`. PR #2's default `evidence_directory()` hardcodes `<CARGO_MANIFEST_DIR>/target/functional`; its `FrozenStore` artifacts are under each launch directory's `check-artifacts/<test-id>/<candidate-sha>/sample.tui.pb`. MBX can direct Cargo output to its managed target cache, so the bulk driver adds an opt-in `TUISCOTTI_CAPTURE_ROOT` override to the disposable evidence-root helper in each revision. This gives each run a new, explicit evidence root independent of MBX's target path. In both, an absent `.tui.pb` makes the old JSON baseline invisible; `FrozenStore::check` records a missing-baseline failure and saves the candidate bytes.

A test normally panics at the first failed visual assertion, which prevents later checkpoints in that same journey from being captured. On a disposable copy only, replace each visual mismatch assertion with this opt-in branch:

```rust
if !outcome.is_pass() && std::env::var_os("TUISCOTTI_CAPTURE_ONLY").is_some() {
    self.log(format!("capture-only {test_id}: {outcome:?}"));
    return;
}
assert!(outcome.is_pass(), "visual mismatch for {test_id}: {outcome:?}");
```

Main has one assertion in `tests/functional/support.rs::App::visual`. PR #2 has two, in `App::visual` and `App::compare_frame` (the latter also serves `visual_while`); replace both. The existing cursor-only fast path remains as-is because it already returns after `FrozenStore::check` writes candidate evidence. In main, make `evidence_root()` first return `PathBuf::from(root)` when `std::env::var_os("TUISCOTTI_CAPTURE_ROOT")` is present, then retain its current default. In PR #2, make `evidence_directory()` first return `PathBuf::from(root).join(format!("{name}-{}-{nanos}-{seq}", std::process::id()))` when that variable is present, then retain its current default. This changes only visual mismatch assertions under an explicit opt-in and lets the copy keep its evidence separate from MBX's shared target cache; PTY startup, interactions, semantic assertions, sample serialization, and failure-evidence writes still execute. Do not commit this patch or run it in the approved checkout. Do not set either capture variable outside the disposable capture copy. Generated candidates remain captured test artifacts, never approved baselines.

### Bulk capture command: terminal-components-baseline main

Start from a disposable copy of live main and patch only its `.cargo/config.toml` to replace the Tuiscotti git source with the immutable local snapshot. Keep the copy's existing baseline roots unchanged. Save output and status outside the checkout:

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

This is the narrowed command proven to select the actual main producer target; the previous `--workspace --test functional` attempt failed target selection because `jackin-console` requires feature `demo`. With capture-only assertions enabled, a missing-baseline result alone no longer fails the test; Nextest may exit zero. Nonzero can still report unrelated semantic assertions, PTY failures, or timeout. Do not infer coverage from exit status; inspect the log and preserve `run-evidence/capture`.

### Bulk capture command: terminal-components-baseline PR #2

Use a separate fresh disposable copy of the live PR head, patch only its Tuiscotti source override and the opt-in assertions above, keep `tests/functional/baselines` and historical `pre_refactor` untouched, and ensure the hardcoded capture directory is empty before the run:

```sh
mkdir -p run-evidence
if [ -e run-evidence/capture ] || [ -L run-evidence/capture ]; then
  printf '%s\n' 'capture root already exists; use a fresh disposable copy' >&2
  exit 1
fi
export TUISCOTTI_CAPTURE_ROOT="$PWD/run-evidence/capture"
set +e
TUISCOTTI_CAPTURE_ONLY=1 MISE_CI=1 timeout 230 mbx nextest run --offline \
  --workspace --test functional --no-tests fail --no-capture \
  >run-evidence/functional-capture.log 2>&1
capture_status=$?
set -e
printf '%s\n' "$capture_status" >run-evidence/functional-capture.exit
```

The unmodified PR #2 producer command was verified on one real sample. The full command with capture-only was later attempted in the disposable PR #2 copy and hit its 230-second cap; details and logs are recorded below. `timeout` is a bound, not a suite runtime estimate. `--workspace --test functional` was the selected target command. Capture-only suppresses the visual mismatch assertion, so missing baselines alone may yield exit zero. Inspect all logs and retain partial artifacts after timeout/failure.

### Candidate inventory and preservation

After the bounded or bulk command (regardless of Nextest exit status), enumerate every captured sample and compare it to the target's active functional JSON path inventory. Do not include showcase, coverage sentinels, historical `pre_refactor`, or arbitrary JSON as candidate IDs. For PR #2 the frozen `pre_refactor` records are never regeneration candidates.

Run the inventory after each target-specific capture with a fresh `OUT` outside the active baseline root. For both TCB revisions set `EVIDENCE_ROOT="$PWD/run-evidence/capture"`; use `ARTIFACT_MARKER=artifacts` for main and `ARTIFACT_MARKER=check-artifacts` for PR #2. The explicit conditional guard aborts if the capture directory exists, while the script itself aborts if `OUT` already exists. These guards give each run new artifacts while leaving MBX's target cache reusable. Keep the complete evidence root and command logs as raw evidence. The script records missing, duplicate, and out-of-scope IDs; it does not decode or approve them.

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
            status = "CAPTURED"
            dest = out / (test_id + ".tui.pb")
            dest.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, dest)
            if hashlib.sha256(dest.read_bytes()).hexdigest() != digest:
                raise SystemExit(f"copy digest mismatch: {test_id}")
            f.write(f"{status}\t{test_id}\t{digest}\t{source}\n")
PY
```

Inspect `MISSING`, `AMBIGUOUS`, and `UNEXPECTED` rows explicitly before any later visual review. The full raw artifact tree retains duplicate candidates by test run and hash. This inventory does not claim a complete bulk rehearsal until every active ID has one unambiguous captured candidate and all expected suite tests have completed. A partial capture-only rehearsal and its incomplete inventories are recorded below; the full PR #2 functional suite did not complete.

Before later PR #2 approval, refresh and validate its reference manifest against the actual capture source binary, current lockfile, comparator, and Tuiscotti candidate revision. The existing wrapper writes receipts from that manifest; leaving its historical values would create misleading provenance. Keep each receipt outside the baseline root. `scripts/check-refactor-coverage.py::baseline_files` currently walks all files with `rglob("*")` and strips `.tui.json` / `.json` in `strip_baseline_suffix`. In a later downstream PR, add `.tui.pb` handling to that suffix helper, keep receipt and metadata JSON processing separate, and add a focused coverage test showing binary IDs count while historical records and metadata stay distinct. Do not claim the checker covers binary candidate IDs until that migration and its checks pass.

## Jackin PR #1264 capture boundary

Jackin has no owned sample files; its identity suite reads the separately pinned terminal-components-baseline `functional` root and asserts expected approved-file inventory. Its current live PR #1264 pins that source at `a901c275055091810ae0d0ac30ec5fc09ce595d9`, distinct from current TCB main and PR #2 heads. If Jackin keeps this source revision, regenerate at a disposable copy of exactly `a901c275…`; if it adopts an approved newer TCB revision, update the pin explicitly in a later authorized downstream change and generate/review against that exact revision. Do not imply that current TCB main or PR #2 captures automatically supply Jackin's baseline. Jackin's work is a consumer/coverage migration after its selected baseline source has freshly generated and explicitly human-approved `.tui.pb` files. Its test path assertions and changed-cell mutation fixture must move from `.tui.json` to `.tui.pb`; keep those changes in a separate Jackin disposable copy and run the relevant identity coverage test after the candidate baselines are staged. Do not bulk-copy candidates into Jackin. Its identity helper writes artifacts to `PathBuf::from(CARGO_TARGET_TMPDIR)/identity/<test-id>/<candidate-sha>/sample.tui.pb`; there is no `check-artifacts` marker in that layout.

The already recorded representative Jackin `functional/jackin_launch_reconnect` capture proves only one ID and the local acceptance/check path. Full Jackin identity coverage with staged `.tui.pb` baselines, full TCB bulk candidate coverage, complete target CI, Linux, and other required architecture suffixes remain NOT VERIFIED.

## Approval stays a later, human workstation step

No command above calls `accept`. After a complete capture, a human reviewer checks every candidate PNG/state and intent against the old approved sample and product behavior, then explicitly approves each candidate on a workstation with `CI`, `TUISCOTTI_CAPTURE_ONLY`, and `TUISCOTTI_CAPTURE_ROOT` unset into a separate empty staging root. For a first `.tui.pb` write into that empty root, use `--expect-baseline missing` even when a prior `.tui.json` was used as review provenance; record that JSON digest separately. For later replacements within the staging root, pass the exact current `.tui.pb` digest. Before PR #2 wrapper approvals, refresh its reference manifest as described above. Hash each input candidate and hash the installed `.tui.pb` after approval; require exact equality. Run `tuiscotti check` on staging and the target's coverage checks before proposing downstream changes. Never bless a whole candidate directory automatically.

## Verification record and known limits

- Read-only: all three live heads queried and rechecked 2026-11-06; snapshot checksum manifest validated; mise Cargo wrapper and MBX versions identified.
- Existing representative macOS mechanics smoke: TCB main, PR #2, and Jackin PR #1264 each captured one fresh sample, recorded candidate SHA, had sample-level semantic/render comparison, local disposable acceptance, post-install digest equality, and offline check pass. A PR #2 second candidate had a real semantic mismatch and was deliberately left unapproved. These assistant-run disposable accepts do not satisfy the required explicit human workstation approval.
- Artifact-root smoke (main): the unmodified helper wrote to MBX's managed target cache, confirming that assuming an in-checkout `target/` is wrong. With the disposable `TUISCOTTI_CAPTURE_ROOT` helper override, the filtered main producer produced the expected missing-baseline failure and saved `functional/showcase_buttons_truecolor` at `/private/tmp/tuiscotti-main-capture-root-proof-20261106/artifacts/functional/showcase_buttons_truecolor/4bc61c42943bd03d9ffa9e46cebc7662159499142a9feaa8ee3cf16279e1f71e/sample.tui.pb` (SHA-256 `4bc61c42943bd03d9ffa9e46cebc7662159499142a9feaa8ee3cf16279e1f71e`). Test exit 100 was expected because this path smoke kept the visual assertion enabled. Log SHA-256: `838631790941c265070a96fb499df5102c246812e2d09086233ad8be4b4f0613`.
- Artifact-root/capture-only smoke (PR #2): `tablepro_editors_types_accept_and_reject_each_family` ran in a disposable copy with capture-only and the isolated root override. It passed 1 test with 1,095 skipped (13.363 seconds). It wrote both candidates under `/private/tmp/tuiscotti-pr2-capture-root-proof-20261106/<launch>/check-artifacts/...`: `pre_refactor/t02_editors/t02_editors_types_120x40` SHA-256 `c7c2dae1f2b37f9c08c559e1bc0b004cf478fca56b98fd2a5b21a6ad610478a9` and `pre_refactor/t02_editors/t02_editors_types_narrow` SHA-256 `00cc1b6bbdd18da504969299409af472a2b24cf5433d8ae117a75dfd7a00d348`. Log SHA-256: `ab9c442fb59783c17b7bf3b417dc8181c7b082afcf7495ccb5c87be4f08e0a42`; exit was zero, demonstrating why missing-baseline output alone cannot be inferred from suite status.
- The documented inventory script passed a parser smoke against both isolated roots: it classified main's captured `functional/showcase_buttons_truecolor` as `CAPTURED` with the matching digest, and PR #2's two historical `pre_refactor/*` checkpoints as `UNEXPECTED` without copying them. Both disposable support helpers were restored byte-for-byte after the runs. No active baseline tree was written.
- Not done: full PR #2 capture-only suite, PR #2 coverage checker, Jackin `.pb` coverage migration, any complete CI matrix or multi-platform runs. The bulk command can be budget/runtime limited and some semantic assertions, PTY setup, app failures, suffix requirements or unsupported platform behavior may still block IDs; record each observed missing/ambiguous ID and preserve artifacts.
- No baseline was modified in a downstream checkout. No published pin or PR head changed, no sample was converted, and no merge SHA is available.
- No baseline approval was performed in this task. All sample review and approval must be done explicitly by a human on a workstation after fresh bulk capture.

## Capture-only rehearsal record — 2026-11-06

These are disposable captures made with the verified local candidate snapshot at `/private/tmp/tuiscotti-goal-current-20261106`; its `SHA256SUMS` digest is `2d1e3c0cdb34294c13573f18165158d0e55aa89f20fa822ee7eaa84cff454765`, with all 684 entries verified. Disposable support patches only add the opt-in visual-assertion suppression and evidence-root override described above. They do not approve candidates. Raw evidence and partial outputs remain in the named disposable worktrees; no candidate was copied into an active baseline.

- Live refs rechecked read-only: TCB `main` `7f385611fba0f8d4a77bb02e0a991ad660b48746`; TCB PR #2 `60b5fb2762d91a3549bd4ac63b1d77d534ee53d2` (OPEN, DIRTY); Jackin PR #1264 `480ba8faab70e87d9c276da6ebd3985c8f92c258` (OPEN, BLOCKED). No branch, published pin, or PR was changed.
- TCB main was captured in `/private/tmp/tuiscotti-tcb-main-clone-20261106` at the main SHA above. Its narrowed producer command completed 117/117 tests in 202.277 seconds with capture-only enabled. Inventory at `/private/tmp/tuiscotti-tcb-main-clone-20261106/run-evidence/candidates/inventory.tsv` reports 101 `CAPTURED`, 1 `UNEXPECTED` (`showcase/buttons/held`), 0 `MISSING`, and 0 `AMBIGUOUS`; the 101 candidate files are fresh binary samples. Main log SHA-256 `48b3d66cfc077f9011ad5677a1da99d1e9f40633c3a835bae7761f214aabe2a3`; inventory SHA-256 `3ea9ec6823c0e6b55ed48ff47cbef592b6a8648b7f5ee133d55a7d12cb369f91`; raw sample hash ledger SHA-256 `e126febf91d82076f1397337a6dd326d79ff4b2ea0e54392458a763a62fe9a21`; candidate hash ledger SHA-256 `478ca056edef1a3b4912a31d7f259b2d46a1ebca4683236491bf029a8dbda19b`. Main's active JSON baseline inventory had 101 IDs. This producer-target run passed, but does not cover other operating systems or architecture suffixes.
- PR #2 was exercised in a fresh detached worktree `/private/tmp/tuiscotti-bulk-pr2-20261106` at the exact PR head. The complete functional command was capped at 230 seconds and exited 124: 93 of 1,096 tests started, 90 passed, `cases::g02_harness::harness_changed_baseline_fails` and `cases::g05_reference::g05_reference_session_receipt_records_identity` failed, and `cases::h05_previews::h05_previews_preview` received SIGTERM at the cap. Its log is `run-evidence/functional-capture.log`. A broader app-name selector separately hit its 230-second cap after 65/152 started (59 passed, five failed, one SIGTERM); its artifacts and log were retained under `run-evidence/active-capture`.
- To prioritize producer journeys after that broad timeout, fresh per-module capture-only partitions ran `showcase` 59/59 (107.721s), `holla` 11/11 (39.745s), `tablepro` 11/11 (83.703s), and `jackin` 10/10 (37.693s). A source audit found three current producer IDs under `lifecycle.rs`, excluded by those module filters; a fresh `lifecycle` partition then passed 2/2 in 5.161s. Each partition had its own previously absent capture root and its own `timeout 230` command. Logs and raw artifacts are under `run-evidence/active-{showcase,holla,tablepro,jackin,lifecycle}`.
- Exact PR #2 partition commands (run separately from the disposable worktree above; preserve all existing evidence and choose a fresh root for any rerun):

  ```sh
  cd /private/tmp/tuiscotti-bulk-pr2-20261106
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

  Each run's command was capped independently. A nonzero exit or timeout is preserved; do not delete prior roots, reduce the selected tests, approve samples, or treat capture-only exit zero as baseline approval.
- Reconciliation used exactly the five roots above, marker `check-artifacts`, the 109 active PR #2 JSON paths under `tests/functional/baselines/functional`, and a new output path `run-evidence/active-candidates-reconciled`. The command below records duplicates by distinct digest, checks binary magic, copies only unambiguous expected active IDs, and leaves all raw samples untouched. To reproduce, set `OUT` to a new absent destination and keep the five evidence roots unchanged:

  ```sh
  cd /private/tmp/tuiscotti-bulk-pr2-20261106
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

- Ledger digest clarification: the pre-run ledger file SHA-256 is `779618450122eef3b0e7b466d01998e77ae3695ce5de69c2692be82d1e5e4c60`; the post-partition ledger file SHA-256 is `c854fbff0b105a5ac9937c640b8add40a6b40b98ab5e88b37e3c6eb455621185`. These file hashes differ because the ledgers were written in different orders. A per-path mapping comparison of the 1,978 entries found 0 changed paths or digests. Do not report the file hashes as equal.
- Reconciled inventory `/private/tmp/tuiscotti-bulk-pr2-20261106/run-evidence/active-candidates-reconciled/inventory.tsv` reports 104 `CAPTURED`, 5 `MISSING`, 0 `UNEXPECTED`, 0 `AMBIGUOUS`; all 104 raw samples have `TUISCPB\0` magic and were copied byte-for-byte to the sibling candidate tree. Inventory SHA-256: `99075cc4864a537308bbc7c8888f691a317d0c20868848f25312f0dffc2ce650`. The five IDs still listed as missing are `functional/jackin_launch_failure`, `functional/jackin_manager_stop_dialog`, `functional/jackin_settings_leave`, `functional/showcase_keyboard_activation`, and `functional/tablepro_query_error_recovery`. Source inspection and passing targeted tests show different current visual IDs: `jackin_launch_failed`, `jackin_stop_instance_dialog`, `jackin_settings_dirty_leave`, `showcase_keyboard_run_task`, and `tablepro_query_recovered`, respectively. These name pairs are observations only; no automatic equivalence, renaming, or baseline deletion is authorized. Human product and provenance review must decide whether an old sample is obsolete or a producer was lost.
- PR #2 baseline immutability was checked against all 1,978 .tui.json files: the pre-run ledger file SHA-256 is 779618450122eef3b0e7b466d01998e77ae3695ce5de69c2692be82d1e5e4c60; the post-partition ledger file SHA-256 is c854fbff0b105a5ac9937c640b8add40a6b40b98ab5e88b37e3c6eb455621185. The ledger bytes differ because their lines were written in different orders, but per-path mapping comparison found zero changed paths or digests. The complete-suite log, timed-out broad-filter log/artifacts, each per-module raw root/log/exit code, and both pre/post-reconciliation inventories remain preserved in that worktree.
- These results are not a full PR #2 bulk rehearsal: the full suite timed out, five active JSON IDs did not receive candidates, and the PR #2 coverage checker has not been migrated to count `.tui.pb` IDs. No conversion, sample approval, CI auto-bless, human workstation approval, Jackin pin upgrade, branch update, or PR action was performed. The separate Jackin sample smoke is not Jackin bulk coverage.

## PR #2 capture failure and missing-ID diagnosis

The saved full-run log is `/private/tmp/tuiscotti-bulk-pr2-20261106/run-evidence/functional-capture.log` (SHA-256 `28800e58692fb3be94476e464d25380d7a551e07ff1f5c966f175daf7a785445`). The candidate source manifest `/private/tmp/tuiscotti-goal-current-20261106/SHA256SUMS` was rechecked at SHA-256 `2d1e3c0cdb34294c13573f18165158d0e55aa89f20fa822ee7eaa84cff454765`; all 684 entries pass. Live refs were rechecked: TCB main `7f385611fba0f8d4a77bb02e0a991ad660b48746`; TCB PR #2 `60b5fb2762d91a3549bd4ac63b1d77d534ee53d2` (OPEN, DIRTY); Jackin PR #1264 `480ba8faab70e87d9c276da6ebd3985c8f92c258` (OPEN, BLOCKED).

### `g02_harness::harness_changed_baseline_fails`

The full-run log shows `assert_ne!(failure.reason(), MismatchReason::Missing)` failed with `left: Missing`, `right: Missing` at `tests/functional/cases/g02_harness.rs:235`. The fixture reads `showcase_popover_resize.tui.json`, changes its embedded test ID, writes `harness_changed_screen.tui.json` in a temporary root, then checks the renamed ID. Binary-only `FrozenStore::check` looks for `.tui.pb`; it correctly ignores the JSON fixture and reports `Missing`. The test's expected state/dimensions comparison cannot be reached. This is a migration test-fixture defect, not a Tuiscotti decoder/comparator failure. Failure evidence is under `/private/tmp/tuiscotti-bulk-pr2-20261106/run-evidence/capture/harness-changed-baseline-60222-1793900130868360000-0`.

Proposed downstream test migration, with semantics preserved and no JSON fallback: stop editing raw JSON. Build a valid temporary binary baseline with `Sample::state("functional/harness_changed_screen", locator_screen(), Observation::empty())`, serialize with `to_bytes()` (or `save()`), and write `functional/harness_changed_screen.tui.pb` under the temporary root. `locator_screen()` is deliberately 8×2 while the live screen is 120×40, so assert `MismatchReason::State` and require `changed_fields()` to contain both `cols` and `rows`; state-grid dimension changes are reported as `State`, while `Dimensions` is reserved for image/render evidence. Also assert the reason is never `Missing`. Preserve the no-write guarantee for active baselines by hashing the whole active baseline tree before/after the check; remove the JSON-specific `production_baseline()`/string-replacement dependency. This exercises the binary-only check path without converting or copying an approved sample.

Relevant source SHA-256: `g02_harness.rs` `f666bb668d050a1f4dd0f53043d1d369951e72a73f60bcef12af113ed9537f12`; `support.rs` `f2c72781efe3c5fd0280142501212a14738ae766de78fd7fcd4f6a07afb5da35`.

### `g05_reference::g05_reference_session_receipt_records_identity`

The full-run log shows the assertion at `tests/functional/cases/g05_reference.rs:411` failed because `evidence` was not under `CARGO_MANIFEST_DIR/target/functional`. The disposable capture driver intentionally makes `evidence_directory()` use the configured `TUISCOTTI_CAPTURE_ROOT`; the evidence path was under `run-evidence/capture/g05-reference-session-...`. The test fixture asserted the helper's default path despite the explicit bulk-capture override. This is an assertion/fixture mismatch, not a Tuiscotti failure and not a reason to remove the root override.

Proposed downstream test migration: derive the expected root from `TUISCOTTI_CAPTURE_ROOT` when set, otherwise retain the current `CARGO_MANIFEST_DIR/target/functional` default. Assert the evidence path starts under that selected root and continue asserting it is outside the active baseline root. Thus ordinary test runs still enforce the default location, while the opt-in capture run validates its isolated root. No production migration script or Tuiscotti library change is indicated. Failure evidence is under `/private/tmp/tuiscotti-bulk-pr2-20261106/run-evidence/capture/g05-reference-session-68445-1793900154333299000-0`.

Relevant source SHA-256: `g05_reference.rs` `4212750e3be2f768d78020460d675f0bd931d2d03158d46a37ebdf21ba61538b`; the same `support.rs` hash above contains the temporary root override.

### Five active JSON IDs without captured output

In the 109-ID PR #2 active JSON inventory, the five missing IDs have no matching Rust visual-call literals in this head. Current tests emit these different IDs instead:

| Active JSON ID with no captured candidate | Current source visual ID | Producer |
| --- | --- | --- |
| `functional/jackin_launch_failure` | `functional/jackin_launch_failed` | `jackin_cockpit_launch_failure_acknowledges` |
| `functional/jackin_manager_stop_dialog` | `functional/jackin_stop_instance_dialog` | `jackin_manager_stop_cancel_then_confirm` |
| `functional/jackin_settings_leave` | `functional/jackin_settings_dirty_leave` | `jackin_settings_unsaved_leave_cancel_then_discard` |
| `functional/showcase_keyboard_activation` | `functional/showcase_keyboard_run_task` | `showcase_keyboard_activates_focused_run_task` |
| `functional/tablepro_query_error_recovery` | `functional/tablepro_query_recovered` | `tablepro_query_select_results_and_error_recovery` |

The corresponding current producer tests passed in the per-module capture partitions, but they produced the current IDs listed in the middle column. That evidence does not establish that the older JSON IDs are equivalent or disposable. Preserve both names and all JSON files pending human product/provenance review; do not rename, delete, or approve any of them automatically. The three other IDs initially absent from the 91-test module filter—`showcase_help_80x24`, `showcase_help_resize`, and `showcase_popover_resize`—were produced by `lifecycle.rs` after its separate 2-test capture. The reconciled result is therefore 104/109, not 101/109.

The five existing JSON file SHA-256s at the PR #2 head are: launch failure `2d7a7bcd54e06f180e1e6de4350db3882f6ff606826a6d900fc02e50f7a5f0f1`; manager stop `7d776bc3242ac7d0396832090e65dcdca405ca779ab63c4ece08821c7b460294`; settings leave `685e8d9fe61890363eabe6694b444de29ceb1eac8094ddee59fda554db35a146`; keyboard activation `2ec7f4cb28b1cd541d5a408a248e6c389ec4079c7573fb9bf1fbbbe55e43ec12`; query error recovery `a87328fd7174a30f0f0406b5c23289e6e9336a25739c22c9c3cc0b31a022684f`.

These findings require downstream test-fixture migration and an explicit human decision on the five ID pairs. They do not justify modifying current Tuiscotti or any automated baseline conversion/approval.

## Follow-up validation and state comparison — 2026-11-06

No current downstream source or active baseline was edited. The following validation uses a new detached copy at /private/tmp/tuiscotti-pr2-failure-fixcheck-20261106, HEAD 60b5fb2762d91a3549bd4ac63b1d77d534ee53d2, with only a temporary local candidate Cargo patch, the capture-only support patch, and two test-fixture edits. No approval API was invoked.

- The binary-safe G02 fixture was changed in that copy to construct and save a valid temporary functional/harness_changed_screen.tui.pb from Sample::state(..., locator_screen(), Observation::empty()); it asserts MismatchReason::State plus cols and rows in changed_fields(). Both the missing-baseline and changed-baseline tests now compare a digest ledger of the active baseline tree instead of requiring a JSON file path. The G05 fixture chooses TUISCOTTI_CAPTURE_ROOT when set and the original target/functional default otherwise, while retaining the assertion that evidence is outside baselines.
- MISE_CI=1 mbx nextest list --offline --workspace --test functional -E 'test(/harness_changed_baseline_fails/) | test(/g05_reference_session_receipt_records_identity/)' selected exactly those two tests. Run command: TUISCOTTI_CAPTURE_ROOT="$PWD/run-evidence/fix-validation/captures" TUISCOTTI_CAPTURE_ONLY=1 MISE_CI=1 timeout 230 mbx nextest run --offline --workspace --test functional --no-tests fail -E 'test(/harness_changed_baseline_fails/) | test(/g05_reference_session_receipt_records_identity/)' --no-capture. Result: 2/2 passed, 1,094 skipped, 17.515 seconds, exit 0. Log SHA-256 64b17c84f1625e83723ffd33b9ce0316e41e23e0396f04e2345c384d996815de. Baseline before/after ledgers each have 1,978 paths and each file SHA-256 c854fbff0b105a5ac9937c640b8add40a6b40b98ab5e88b37e3c6eb455621185; cmp returned 0. The generated G02 candidate is under run-evidence/fix-validation/captures/.../check-artifacts/functional/harness_changed_screen/; its result.json reports reason state, changed_fields [cols, rows, scope, palette, default_foreground, default_cursor, cursor.col, cursor.row, cursor.visible, cursor.shape, cursor.color], candidate SHA-256 bddfc66df0fe07a038ae9a9483bcbe971162a833a74eea0b979c96ae79604473, and temporary binary baseline SHA-256 fa1a88895231f334d75e469cc8163ab019e4b2e630f775ea3bc08ce3473b1a85. G05 wrote its session.json under the isolated capture root. Neither test calls accept.
- The focused fixture diff in the disposable copy is preserved at run-evidence/fix-validation/fixture-fix.patch, SHA-256 aed2e0ca0f599c4377fe5219499a2d1502354bca5789944fe12942bf09f6001f; post-edit source hashes: g02_harness.rs 1c15b0ff0bce49d2f50e69408ec1bacf6572903748d3abef3199f1df08065351, g05_reference.rs 40cc76938c42e5022024a78025f4cd07f432002bbf1d7e81a34ea8c715eb611d. These are validation changes only and are not committed.
- For the five unmatched old IDs, an ad hoc read-only mbx run tool decoded the .tui.pb candidates with Sample::from_file, compared their serialized TerminalSnapshot values to the old JSON snapshot objects, and did not write, rename, approve, or convert samples. Both samples in every pair are gate state, 120×40, and have no embedded PNG/render contract. The comparison excludes sample IDs and capture metadata, and reports grid/cursor/cell differences only; it is not an approval or assertion that differently named IDs are interchangeable. Utility source is /private/tmp/tuiscotti-state-diff-20261106/src/main.rs (SHA-256 be5474b29037f9aca1c3599790cd38c60ab70cd11470186ad5626cd0e39b73eb); mbx run output is /private/tmp/tuiscotti-state-diff-20261106/run.log (SHA-256 fbc4833fbe072c8f1fb1cda4442c5e2a52366f92ff29a74f0175b1db4a49577c). Counts below are changed cells and nonexclusive per-cell property counts:

  | Existing JSON state → captured binary state | Differing text rows | Differing cells | Cell property differences | Representative visible difference |
  | --- | ---: | ---: | --- | --- |
  | jackin_launch_failure → jackin_launch_failed | 31 | 4,734 | text 1,112; fg 4,423; bg 1,574; bold 185 | Progress row says 2/11 stages in old state, 6/11 in captured failed-network state. |
  | jackin_manager_stop_dialog → jackin_stop_instance_dialog | 26 | 4,750 | text 1,122; fg 4,653; bg 701; bold 116; underline 2 | Old state shows the generic workspace manager; captured state shows selected workspace/instance details. |
  | jackin_settings_leave → jackin_settings_dirty_leave | 38 | 4,758 | text 1,287; fg 4,669; bg 2,993; bold 113; underline 3 | Old state is the workspace manager; captured state is Settings with one unsaved change and the leave choice. |
  | showcase_keyboard_activation → showcase_keyboard_run_task | 2 | 26 | text 10; fg 11; bg 18; bold 7 | Old focus/status is Cancel / Cancel ✓; captured focus/status is Run task / Run task ✓. |
  | tablepro_query_error_recovery → tablepro_query_recovered | 23 | 1,383 | text 1,135; fg 1,005; bg 674; bold 13; underline_color 8 | Old state contains SELEC bogus; and the syntax error; captured state contains corrected SELECT * FROM orders; with no error. |

- docs/testing/baseline-dispositions.json at the checked PR #2 head (SHA-256 775e370b851082ebc999179ca839c79da2d14a2039678b215df3308871be2d8c) already labels these five JSON entries superseded and lists the current IDs as replacement_id, with reasons matching the observations above. This existing disposition is relevant historical evidence, not permission to execute a rename, delete a file, or approve a candidate in this rehearsal. The human migration owner must decide whether to adopt those recorded dispositions. No file or baseline action was taken here.
