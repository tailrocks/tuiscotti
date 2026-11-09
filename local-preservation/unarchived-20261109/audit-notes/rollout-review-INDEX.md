# Rollout review candidate index — 2026-11-06

This is a local review index only. It does not approve, accept, promote, or publish any baseline and is not a review application. Evidence remains in its original `/private/tmp` locations; this index links to it without copying or modifying it.

## Reviewed Tuiscotti source

- Snapshot: `/private/tmp/tuiscotti-rollout-reviewed-20261106`
- `SHA256SUMS` SHA-256: `471e8b11f1ef7f4fd846f67517d47d9a8d68ad00a9b9d0e0ea04136601c0e9e2`
- All entries in that manifest verified successfully with `shasum -a 256 -c`.
- PR #2 evidence states its exact source was this snapshot. Its tested TCB revision was `60b5fb2762d91a3549bd4ac63b1d77d534ee53d2` (PR #2 rehearsal; dirty disposable checkout), not TCB main.

## Independent evidence sets

### Jackin identity capture

- [HANDOFF.md](/private/tmp/tcb-jackin-pin-a901c275-20261106/run-evidence/jackin-identity-20261106/HANDOFF.md) — SHA-256 `2552ff6ec6e98f24279e73af939028488db742e98e3fe1c77a4c64e7419c9738`
- Candidate ledger [jackin-identity-candidates.tsv](/private/tmp/tcb-jackin-pin-a901c275-20261106/run-evidence/jackin-identity-20261106/jackin-identity-candidates.tsv) — SHA-256 `d8e903ac46f9ce00efa1ba1ea16a2c2b54d2744cc5da9a3f253ddfe4b7a7d388`
- [nextest.log](/private/tmp/tcb-jackin-pin-a901c275-20261106/run-evidence/jackin-identity-20261106/nextest.log) — SHA-256 `59c82e15a921e1fdfb1e783c6ed2bef3bff626225aaa678fb8a772f84ddf0bca`
- Exact TCB source: `a901c275055091810ae0d0ac30ec5fc09ce595d9`; original Tuiscotti pin (temporarily overridden to the local binary implementation for rehearsal): `d8d54155947f6303a41dfd6079b53c0abc16f29a`. Consumer Jackin head was `845d128da0b839befab4dc69f24c0e748fc25069`.
- Handoff reports 55/55 IDs matched and 52/52 TCB functional tests passed; focused downstream identity tests passed 5/5. Captures are unreviewed and unapproved; full identity integration target was not run. Existing JSON baselines remain unchanged. No candidate is approved.
- Ambiguity: this is a distinct TCB pin and should not be treated as PR #2 or main evidence.

### TCB PR #2 rehearsal

- [README.md](/private/tmp/tcb-pr2-rollout-prep-20261106/run-evidence/README.md) — SHA-256 `1bddcde8766d135ff2f4c2fe864722880748a4156c9089a7cd9f23e9fd419fc2`
- v2 [inventory.tsv](/private/tmp/tcb-pr2-rollout-prep-20261106/run-evidence/active-candidates-reconciled-v2/inventory.tsv) — SHA-256 `58a1c0eb112be0a152d683f48b6758cc406c126bcff10b3fbe50d0e0c8eef5bb`
- [five-unmatched-id-review.tsv](/private/tmp/tcb-pr2-rollout-prep-20261106/run-evidence/five-unmatched-id-review.tsv) — SHA-256 `067e7c2c0b5bd92f91772220a0e974f461ea82c6ee24ab18d48a33cdf60c06b5`
- Candidate ledger [candidate-sha256.txt](/private/tmp/tcb-pr2-rollout-prep-20261106/run-evidence/candidate-sha256.txt) — SHA-256 `0ad2686dcb225f97a3c6187ef78f020619ebe356cae68e9d47ac11da6024a4f9`
- Exact tested source as reported: reviewed snapshot above (manifest digest matches). TCB revision `60b5fb2762d91a3549bd4ac63b1d77d534ee53d2`; TCB main at recheck `7f385611fba0f8d4a77bb02e0a991ad660b48746`.
- v2 counts: 104 `CAPTURED`, five `MISSING`, zero `AMBIGUOUS`, zero `UNEXPECTED`. Missing old IDs: `functional/jackin_launch_failure`, `functional/jackin_manager_stop_dialog`, `functional/jackin_settings_leave`, `functional/showcase_keyboard_activation`, `functional/tablepro_query_error_recovery`. The disposition ledger maps these to replacements but marks each `REVIEW_REQUIRED`; preserve old files and do not rename, delete, or approve based on this evidence.
- Rehearsal has capture/test partition failures and retries detailed in README. Existing production baselines remained unchanged; no sample was accepted. Candidate set is not approved.

### TCB main rehearsal — source mismatch

- [README.md](/private/tmp/tcb-main-rollout-20261106/run-evidence/README.md) — SHA-256 `354f2c80a5a4f8080ac4713af678045300f64dfdaad03dde4a7ceee1d174041d`
- Inventory [inventory.tsv](/private/tmp/tcb-main-rollout-20261106/run-evidence/candidates/inventory.tsv) — SHA-256 `04253e349de627853db255137464b4e794a7cd272ce4e85da2c4568b985362f6`
- TCB revision: `7f385611fba0f8d4a77bb02e0a991ad660b48746`.
- Exact source reported by this README: `/private/tmp/tuiscotti-continuation-snapshot`, SHA `d31900993b533ad4758848c261b5f4c5b2eb1cee`. This is not the reviewed snapshot above. The README describes a separate, earlier run and says no merged Tuiscotti SHA existed. Its results cannot establish testing against the reviewed final head.
- Reported counts: 101 active IDs captured, zero missing/ambiguous, one unexpected candidate `showcase/buttons/held`; no human review or approval. Full capture attempt hit its 230-second bound; tail completed. Validation is split across runs as described in README.
- Exact-source rerun against the reviewed final head was reported as still live when this index task was assigned; no result is represented here. Do not conflate this older-codec/source rehearsal with the final head.

## Provenance and review status

All linked artifacts are existing files in the specified evidence trees. Hashes above were recomputed locally for the linked evidence files; the snapshot manifest itself was fully checked. This index does not independently establish facts beyond the linked handoffs, logs, manifests, and inventories. No visual review or baseline approval is recorded here. No `accept` command, downstream application, PR, or shared-file edit was performed.

## Scope limits

The main TCB rerun is unresolved here. The PR #2 missing IDs and all Jackin captures still require human disposition/review. A successful capture or consumer check is not visual approval. No inference is made that evidence from different TCB commits, Tuiscotti source snapshots, or test inventories is interchangeable.

## Later exact-source TCB main rehearsal

The independently reviewed [exact-source rehearsal](/private/tmp/tcb-main-final-src-rehearsal-20261106/run-evidence/README.md) used Tuiscotti commit `818b2ebfd94cfea669a6603b79cb96532db9b282`, preceding later PR #26 changes. All 117 producer tests passed without capture bypass in bounded validation groups. It found 100 unique active IDs, one active ID with two candidates differing only in capture timestamp, no missing active IDs, and one extra showcase ID. Both variants remain available for explicit review; no candidate has been approved. The original TCB baselines are unchanged.

## Preserved candidate archive

The independently checked [candidate archive](../benchmark-evidence/rollout-candidates-20261106.tar) has SHA-256 `d19d460587aa0e886e8f159dbdd10512898622502d942da07514ee461cadfe4e`. It retains 103 TCB-main raw candidate containers (100 unique active, two variants of one ambiguous active ID, and one unexpected ID), 104 PR #2 candidates, 55 Jackin candidates, linked local report assets and manifests. Its source revisions and unresolved reviews are recorded in its README. These samples remain unapproved.

## Final-source Jackin validation evidence

The independently reviewed [Jackin final-source archive](../benchmark-evidence/jackin-full-final-source-20261106.tar) has SHA-256 `6be5be9f8f2f0cc2f102ccf08ed544c4cb28cd968b6ea33ab6aa2098d2a76a75`. It preserves 55 fresh candidates and test fixture copies generated against the `59704ed` implementation snapshot, plus logs for all 47 identity tests passing in four normal-assertion partitions. The producer log reports 52/52 passed, but its wrapper exit receipt is missing. No candidate is approved.

## Additional PR #2 candidates

The independently reviewed [PR #2 additional-candidate archive](../benchmark-evidence/pr2-new-candidates-613dbf6-20261106.tar) has SHA-256 `42af17f61b2eebba9f95a2b84a074e9f42811b8d40c74f105dd55812e3dfc892`. It contains two stable paused-frame Holla candidates, two lifecycle candidates, and one Jackin first-use candidate, with raw evidence and focused validation logs. All remain unapproved; the five prior unmatched IDs require disposition.

## Integrated PR #2 validation

The independently verified [PR #2 integrated evidence](../benchmark-evidence/pr2-integrated-93of93-25fd820-20261106.tar) has SHA-256 `3d9b63310fd8ff3d61eb4246dd0b3390998f8ae7b2df9caa2762f972953deb61`. Normal-assertion partitions pass 93 distinct visual tests against 104 unapproved temporary fixtures. Five superseded old IDs still require disposition; this is not approval or active-baseline replacement.

## Jackin live-head identity rerun

On PR #1264 head `27ec25ec0aa180af9bf075660af5cbf55dccefc3`, the detached worktree `/private/tmp/jackin-pr1264-livehead-27ec25e` ran `mise exec -- mbx +1.99.1 nextest run -p jackin-testkit --test identity --locked --offline` with the reviewed local Tuiscotti and TCB binary migration overlay. Nextest reported 47 passed, zero skipped; the exit receipt is zero. An independent review compared all 55 scratch fixtures byte-for-byte against the archived candidates and found zero mismatches. The unmodified published PR still uses JSON; these candidates remain unapproved, and this run is not the full E2E suite.

Evidence under that worktree's `.audit-local/`: `identity-pr1264-livehead-27ec25e.report.txt` SHA-256 `07fd7d3c5f2e7fa8162fd63ecce176c4b291628f4e0049c097fff3b500ec5f04`; raw log SHA-256 `33d46b8fceeff39fc60bcfa2ead07ed97c3314899ea6f24c40bdb14a9969de8c`; exit receipt SHA-256 `9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe2ab86aa`. The command provenance is recorded in the report; the raw log alone does not record the shell command.

## TCB main final-source validation

A native arm64 rehearsal at TCB main `7f385611fba0f8d4a77bb02e0a991ad660b48746` and Tuiscotti `59704edfff6a6676e8833f190dcc43dccc528329` passed all 117 distinct functional producer tests across 12 normal-assertion groups; both retained `showcase_page_diff` variants passed their focused assertions. Independent verification reconciled the 117 test IDs with all group filters and receipts and verified 202 functional candidate hashes across the two scratch roots. The existing unexpected `showcase/buttons/held` fixture is present only in the primary scratch root, outside that 202-file hash manifest. No baseline was approved; the ambiguity and extra capture remain for review.

[Final rehearsal report](/private/tmp/tcb-main-final-validation-20261106/evidence/FINAL.md) SHA-256: `f702503513735bbf7c77ff466762ca8a75c093d7223e87ed40dc93451205edf6`. Capture-only was unset in the recorded environment and omitted from the recorded commands; the test logs do not independently capture each process environment. The report relies on the prior rehearsal for the variants' state/render equivalence. The checked-in TCB baseline tree has no changes.

The [portable TCB main evidence bundle](../benchmark-evidence/tcb-main-final-validation-20261106/README.md) preserves the logs, receipts, both scratch fixture roots, and lock/toolchain inputs. Its compressed archive SHA-256 is `3cc355955fb66d37d3dff9bcffb1f11aa574f4f4ff9f075ae1f78e73884d0542`. It requires the recorded source commits and pinned tools for reproduction; the original Cargo patch path must be adjusted in a disposable checkout.

## Corrected TCB main fixture reconciliation

The earlier inventory's `UNEXPECTED` classification for `showcase/buttons/held` is superseded. Its expected-set scan omitted tracked fixtures outside `functional/`; the nested showcase fixture is tracked and its producer uses that exact test ID. The independently reproduced [corrected reconciliation](held-reconciliation/README.md) includes all tracked fixture paths: 102 IDs, 101 with one candidate and one with two candidates, zero missing IDs, and zero untracked candidate IDs. The 103 candidate hashes include the still-ambiguous `functional/showcase_page_diff` pair. `showcase/buttons/held` passed its normal producer assertion against its fresh staged candidate, which does not by itself prove equivalence to the prior JSON or approve the candidate.

The reconciliation checks candidate file paths, container magic, and hashes; it does not decode every binary payload or verify its embedded test ID. The pre-existing reports and archive are preserved unchanged.

The [decoded candidate audit](decoded-candidates/README.md) uses the final public `Sample::from_file` reader at Tuiscotti `59704ed` on all 103 distinct candidates. Each decoded test ID, file SHA-256, and byte count matches the corrected inventory. An independent rerun produced byte-identical results and independently checked candidate directory hashes. This validates decoding and identity, not visual approval; the two page-diff candidates remain ambiguous.

## PR #2 TablePro syntax-error coverage

An audit found the old TablePro fixture captured the syntax-error screen while its proposed replacement captured only recovery. A reviewed local preparation adds a distinct error-screen checkpoint and preserves the recovery checkpoint. [Evidence and the proposed patch](pr2-tablepro-checkpoint/README.md) include fresh `.tui.pb` candidates and a passing normal-assertion focused test with capture-only unset. Session metadata retains an older reference-manifest revision; the actual local build used Tuiscotti `59704ed`. This does not approve either candidate or change the PR head.

The TablePro checkpoint is preserved in local signed commit `f99172be849830e14ed80e69f5efa361022729c6`. The [integrated validation receipts](pr2-tablepro-checkpoint/final-93/) record 93/93 distinct visual producer tests passing against an unapproved 105-fixture composite, with capture-only unset. An initial Holla run using older candidates failed two age-cell comparisons; the rerun used the previously validated paused-frame candidates. An independent audit checked test coverage, receipts and baseline hashes; a later lineage record traces three other fixture replacements byte-for-byte to earlier captures. These checks do not establish a match against production baselines or approve the composite.

Additional [PR #2 repository checks](pr2-tablepro-checkpoint/repository-checks/repository-checks.txt) passed formatting, targeted functional Clippy, coverage-script fixture tests, and disposition validation. These checks used the repository's locked pre-migration Tuiscotti pin, not the local binary override used for the 93-test rehearsal. Formatting of Holla setup was corrected in local signed commit `6ee587e601c2a69f3540da3924720b2863d19d7b`. The full coverage gate remains failing with blocked rows and missing receipts and baselines; no candidate has been approved.

The prepared PR #2 branch also passed `mise exec -- mbx clippy --all-targets --locked --offline -- -D warnings` with a temporary local patch to Tuiscotti `59704ed`. [Binary-dependency lint evidence](pr2-tablepro-checkpoint/binary-clippy/) records the command, lock provenance and restored clean worktree; active-baseline hashes were unchanged.

## Jackin current-head local preparation

The local branch `codex/jackin-pr1264-binary-identity` is based on PR #1264 head `27ec25ec0aa180af9bf075660af5cbf55dccefc3` and ends at `5b7f8232bef4ae372f915b26ea61a0f955bbfb26`. It adapts identity fixture discovery and mutation to binary samples and preserves the observation field. Independent Sol review found the observation issue, which was fixed and rereviewed without further findings. The identity suite passed 47/47 with local dependency and unapproved fixture overlays; package formatting passed. Workspace formatting has a pre-existing unrelated diff in `crates/jackin-tui/src/launch.rs`. [Local run records](jackin-current-head-identity/) preserve the test evidence. The verified Git bundle `.audit/jackin-current-head-binary-preparation.bundle` has SHA-256 `429b21e2a62a1ab305379c810632e02320195bc75778aee9c0629c16697b1b68`. No PR head, durable pin, or active baseline was changed.

## PR #2 nested held-button capture

The corrected recursive scan found the earlier five producer partitions omitted `showcase/buttons/held`. A [separate native arm64 capture and normal-assertion check](pr2-held-capture/README.md) at local TCB preparation `6ee587e` and Tuiscotti source `59704ed` produced candidate SHA-256 `f75af1e12d8eddfe59126d504587105f354118615ea5d5c1c6c031de95cdcbbc`. The public CLI decoded its ID, and the producer passed 1/1 against a byte-identical disposable fixture with capture-only unset. Independent review verified baseline hashes unchanged. This is separate from the 105-fixture/93-test run; the candidate remains unapproved.

## Codec replay after allocation fixes

The [public-codec replay](../roundtrip-codec-47491d4-59704ed/README.md) decoded and re-encoded 104 preserved state-gated captures at each of `59704ed` and `47491d4`; every output matched its input bytes, decoded sample and state digest, and the per-row result files matched each other. Independent audit verified all input hashes, result rows and clean source heads. The corpus contains no pixel-gated samples, and the earlier-source binding relies on the documented harness path swap rather than source information embedded in the results. This is not baseline approval or a replacement for pixel-gated verification.

The [pixel-corpus replay](../roundtrip-pixel-codec-47491d4-59704ed/README.md) covers 20 frozen real captures, including ten pixel-gated samples, at `59704ed` and `47491d4`. Container bytes, decoded samples, state digests, public contract projections, embedded PNG hashes, and independently decoded RGBA hashes matched. Independent audit verified inputs, results and archive checksums. Source binding is procedural rather than self-attested in run logs; this does not re-render frames or test subsequent `13c56b5`.

The [current-head pixel replay](../roundtrip-pixel-current-head-2ac5b8c/REPORT.md) passed all 20 frozen captures at `2ac5b8c`, including PNG/RGBA identities for ten pixel-gated samples. Independent verification confirmed 140/140 frozen checks and per-row equality with prior runs. Its wrapper checked the exact clean Git commit and tree before and after; ignored files are outside Git's clean-status check. The local copy preserves the report, source, logs and results, while the original checksum manifest refers to the temporary compiled binary and original artifact paths. This does not establish fresh rendering or cross-platform determinism.

The [PNG-cap current-head replay](../roundtrip-pixel-current-head-99fa00f/REPORT.md) passed all 20 archived real captures at `99fa00f`, including ten pixel-gated samples; its 140 frozen checks and rows matched prior results. Independent audit verified source guard, evidence hashes and frozen records. It does not test fresh rendering or cross-platform determinism. The copied manifest retains original absolute paths and includes a temporary compiled binary not copied here; the external benchmark archive is not covered by that evidence-folder manifest.

## Final implementation review

An independent Sol medium review of Tuiscotti `99fa00f` against merge base `d8d5415` found no actionable correctness or security issues in container bounds, generated strict scanning, allocation accounting, mapping, digests, approval, binary consumers, or the latest PNG and geometry changes. Its independent workspace nextest run passed 193/193. The review did not rerun downstream regeneration, benchmarks, or fuzzing and is not a GitHub approving review.

## Synthetic acceptance transaction probe

The [acceptance probe](../accept-synthetic-probe-9e8eab4/REPORT.md) ran the real library `ApprovalStore::accept` twice per invocation using synthetic state-only samples and a disposable temporary baseline root. Five runs exited 0; maximum observed RSS was 6,389,760 bytes and the sum of wall times rounded to hundredths was 0.12 s. Independent audit verified source/binary hashes, test scope and raw resource measurements. This includes test setup, does not measure pixel acceptance or provide codec timing evidence, and does not approve any real candidate. Caps were compared after the runs, not enforced by the OS. The evidence was moved from the implementation worktree to this `.audit` directory after recording; any old absolute paths in its raw records retain their original provenance.

At Tuiscotti PR #26 head `9e8eab42e072936824186bbaa17919181410a6e9`, GitHub Actions run `42184060181` completed successfully, and its `Plan` job's `Check generated files` step specifically concluded `success`. That is CI evidence, not local arm64 execution. The local `mise run ci-deny` at the same head passed advisories, bans, licenses and sources, with non-fatal duplicate and unmatched-license-allowance warnings.

## PR #2 integrated current-source rehearsal

The [current-source rehearsal](pr2-final-source-006c8f9/README.md) ran 94 distinct selected producers against 106 unapproved temporary fixtures on native macOS arm64, with capture bypasses unset. It used TCB preparation `6ee587e` and Tuiscotti `006c8f9`; all six partitions passed. Independent audit confirmed 106 fixture hashes, all 94 distinct passing test names and exit receipts, and equal before/after hashes for all 1,978 active baseline files. This does not resolve the five old-ID dispositions, approve any candidate, or establish a match against active production baselines.

## Jackin e30c34df refresh

The local binary identity preparation was replayed on PR #1264 head `e30c34df8a49feb7d228ec09f8fcc5be7190611e` and ends at local commit `bed65e9318393ab2aa32e35b7c2a3c97bf1c9529`. The [identity rehearsal](jackin-e30c-identity/identity-pr1264-refresh.report.txt) passed 47/47 with local Tuiscotti `006c8f9` and 55 unapproved fixture overlays. An independent audit verified test receipts, fixture hashes and patch ancestry; capture-bypass environment is reported but not recorded in the raw test log. Package fmt reports an unchanged upstream `gateway.rs` issue. This is not full Jackin validation or candidate approval. Bundle `.audit/jackin-e30c-binary-preparation.bundle` SHA-256 `32916ada0af9958fd2fee1a6e9bd7d8a0d57443ef4ca53d73e01e2614fc75e9e`; no shared branch changed.

## Jackin 7198da57 refresh

At PR #1264 head `7198da57fa02666df3d14adfae665f03e0799913`, the local binary identity preparation ending at `41dd7372175f9720eb5eaac6c0538cdc84bea181` passed 47/47 identity tests against 55 unapproved binary fixture overlays and Tuiscotti `006c8f9`; five focused Muse tests also passed. Independent audit verified ancestry, receipts, fixture hashes and identity command environment. The Muse environment is reported but not independently recorded in its command log; these are focused tests, not full Jackin validation. Package fmt still reports unrelated unchanged `gateway.rs` formatting. [Evidence](jackin-7198-identity/identity-refresh.report.txt); verified Git bundle `.audit/jackin-7198-binary-preparation.bundle` SHA-256 `5c86944a1a08d05981320503e3fe83c64b4a5517190582f7f24cfe472dddda7d`. No shared branch or active baseline changed.

## PR #2 ledger and G05 preparation

Local TCB preparation now ends at `33d9a7772c9a05c80843d2e45942ffcc7b62d08b`. It recursively discovers nested `.tui.pb` baselines, and G05 reads active replacement samples as binary while retaining ID and dimension assertions. Three suspect JSON records were copied byte-for-byte into frozen `pre_refactor/functional` provenance; their statuses and original active files were not changed. The [isolated G05 rehearsal](pr2-g05-binary-overlay/VALIDATION.md) passed against 106 unapproved binary fixtures and local Tuiscotti `006c8f9`; independent review verified the exact source/root overrides and unchanged fixture hashes. Before approved promotion, the unmodified active root lacks the binary samples and the strict G05 check correctly fails. The local preparation bundle `.audit/pr2-binary-preparation-33d9a77.bundle` SHA-256 is `254b02b3762c56d1da33c47de6d061c2d6b844bf13fd38471947dc7b1e2bb931`. No shared branch or approval changed.

An independent Sol medium final-head review of Tuiscotti `2b85b23` (increment since previous full review at `99fa00f`) found no new actionable issues; workspace nextest passed 194/194 and library package verification passed. Standalone CLI package verification against the published library still fails because registry `tuiscotti 0.4.0` lacks `FrozenStore::check_owned`. CLI package tests previously passed only with a temporary local library patch; publication requires coordinated updated library/dependency and is outside this run.

## Jackin eb90acea refresh

A new local preparation at PR #1264 head `eb90acea891bd5936aa34b0dafb4ffdcaf57fb30` ends at `3106d0018edc1e2b23f56a1f52a10ce7484cfdc4`. Against 55 unapproved binary fixtures and local Tuiscotti `0fbcb06`, identity passed 47/47 and a focused account/launch/plan selection passed 12 tests with 225 skipped. Independent audit verified source/branch ancestry, capture-bypass environment receipts and fixture hashes; it is not full Jackin gate coverage. [Evidence](jackin-eb90-identity/identity-refresh.report.txt); verified bundle `.audit/jackin-eb90-binary-preparation.bundle` SHA-256 `d71ee562a271c4a582bf428cb75d0fcae4b1394f34355ea98e63cbb44fb73a13`. No candidate approved or shared PR head modified.

## CLI check resource probe

At Tuiscotti `0fbcb06`, the [CLI check benchmark](../tuiscotti-cli-check-bench-20261106/README.md) ran 530 passing checks across five passes over 106 real unapproved state-only samples against byte-identical disposable baselines. Median pass wall was 3.711 s, observed maximum child RSS 6,897,664 bytes. Independent review verified raw output/identity/hash receipts and corrected timing/load labels. This does not cover pixels or mismatches; limits were checked after child processes (600-second watchdog), not enforced as a hard memory limit. Host contention prevents relative timing claims. No acceptance or active baseline write occurred.

## Pixel-gated CLI check resource probe

At Tuiscotti `0fbcb06`, the [pixel CLI check probe](../tuiscotti-cli-pixel-check-20261106/README.md) ran 50 passing checks across five passes over ten real state-and-pixels samples against identical disposable baselines. Median pass wall was 0.943 s; maximum observed child RSS was 62,259,200 bytes. Independent review verified the 50 outputs, candidate/baseline digests, raw timing and RSS, and 164 evidence checksums. The probe covers only the equal-baseline success path on a contended native arm64 host; roots were verified unchanged by hashes, not permission-enforced read-only, and resource caps were assessed after each child with a watchdog. No real candidate was approved.

## PR #2 current-source rehearsal at 0fbcb06

The [rehearsal](pr2-final-source-0fb/README.md) used local TCB preparation `33d9a7772c9a05c80843d2e45942ffcc7b62d08b` and Tuiscotti `0fbcb0644de3b51e8ca73f59d257118edf16c12d` on macOS arm64. With capture bypasses unset and the baseline root redirected to 106 isolated, unapproved binary fixtures, 94 distinct selected producer tests and two separate G05 tests passed. Independent audit verified fixture provenance and hashes, and unchanged hashes for 1,978 active files and three archived records. This is not the full functional suite or a CI matrix; the path dependency and temporary root override do not prove a published dependency build. It neither approves candidates nor resolves the five legacy ID dispositions.

## Jackin c28e5c6 refresh

On PR #1264 head `c28e5c6c4ce7f339a833c6d68184040c75ab5030`, the isolated preparation ends at `0de1bac926a739dfa0a556732136732c448b8c14`. The [identity rehearsal](jackin-c28-identity/) passed 47/47 tests with capture bypass variables filtered out, local Tuiscotti `0fbcb064` and 55 unapproved binary fixture overlays. Independent review confirmed ancestry, receipts and unchanged fixture hashes. Local dependency patches and lock changes include six `windows-sys` references; this is not validation of the unmodified PR, its full suite, or other platforms. No candidate was approved and no shared PR head was modified.

## Five legacy PR #2 IDs: source and state review

A read-only source review checked the five rows in `../benchmark-evidence/rollout-candidates-20261106/pr2/run-evidence/five-unmatched-id-review.tsv` against `docs/testing/baseline-dispositions.json` in the local PR #2 preparation and `/private/tmp/tuiscotti-state-diff-20261106/run.log`. All five have recorded supersession intent, but none is identical to its named replacement: three Jackin samples show different stages or dialogs, the showcase sample differs in focus and action, and the old TablePro error differs from the recovered query. Current visual calls produce the replacements, not the old IDs. The dedicated TablePro syntax-error capture preserves coverage of the error state; it does not establish byte identity with the old sample. These findings support retaining the old active records until disposition and candidate approval, not treating a rename as equivalence.

## Exact-head local gates

At Tuiscotti `0fbcb0644de3b51e8ca73f59d257118edf16c12d`, the [native macOS arm64 gate run](tuiscotti-0fb-local-gates/SUMMARY.md) passed formatting, workspace all-target clippy, workspace nextest (194 passed), library doctests (2 passed), rustdoc with warnings denied, and `mise run ci-deny`. Cargo checks ran through the mise-managed mbx wrapper; the source checkout stayed clean. Cargo deny reported nonfatal duplicate dependency and unmatched ISC allowance warnings. These local results do not establish cross-platform determinism.

## Downstream codec consumer proof

The [consumer proof](tuiscotti-downstream-consumer-0fb/provenance.txt) ran `mise exec -- sh proof/downstream.sh` in a disposable copy of Tuiscotti `0fbcb064`, using the mise-managed mbx for its Cargo builds. It passed the combined pkg-config, legacy, thin, zstdmt and zdict_builder feature consumer and confirmed the tested non-Cargo and empty pkg-config cases fail closed. CMake was unavailable and its case was skipped; the run does not validate a published registry package or other platforms. Source hashes and raw logs are retained alongside the evidence.

The previously skipped CMake case was subsequently run successfully with CMake 4.3.6 installed in a disposable tool directory. The [fixture-root rerun](tuiscotti-downstream-consumer-0fb/cmake-rerun-fixture-root/downstream.log) exited 0 and source integrity verification found no mismatches. An earlier invocation from the wrong working directory is excluded from the evidence. This remains a native macOS arm64 consumer proof, not published-package or cross-platform verification.

## Independent final-head implementation review

An independent Sol medium review of clean Tuiscotti PR #26 head `cc4db85db920c7183997171c2ae7c444da33cdb0` found no new actionable issue. It examined the increment from `2b85b23`, strict decoder bounds and mapping, digest and approval seams, binary consumers, dependency evidence and downstream guidance. It confirmed the dependency replay snapshot's manifests, decoder and digest source match this head byte-for-byte. The replay covers 20 frozen captures and 140 checks on native macOS arm64, not fresh rendering or cross-platform proof. The CLI package still requires coordinated library publication/dependency update because published `tuiscotti 0.4.0` lacks `check_owned`; publication is outside this run. The review was read-only and did not rerun gates or approve candidates.

## Jackin fresh candidates against merged Tuiscotti

On PR #1264 head `c28e5c6c4ce7f339a833c6d68184040c75ab5030`, an isolated capture produced 55 candidates for 55 expected IDs using merged Git Tuiscotti `fec38841b10a593f64498f1e6daf2f4246a4d70f`; 42 journey tests passed, with five coverage-only tests excluded from capture. A separate normal-comparison worktree passed all 47 identity tests against those candidates. Independent Sol medium review verified exact Git source, decoded IDs and gates, manifests and clean comparison `app.rs`. The comparison worktree includes prepared changes to Cargo.toml, Cargo.lock, Cargo config and binary coverage tests; its disposable TCB overlay replaces 55 tracked JSON fixtures with fresh binary candidates. Absence of capture/bypass environment variables is receipt-only because the filter output was not retained; clean comparison code independently excludes the capture instrumentation. This is not approval, old-baseline equivalence, full workspace validation, or cross-platform evidence. [Capture provenance](/private/tmp/jackin-pr1264-capture-probe-20261106/capture-provenance.txt); [comparison result](/private/tmp/jackin-pr1264-fresh-compare-20261106/.audit-local/result.txt).

## Post-merge downstream capture and comparison

Tuiscotti PR #26 merged as `fec38841b10a593f64498f1e6daf2f4246a4d70f`; PR #25 subsequently merged as `6d01628170de8bed3f00f52713f3a510b782e41e`. At TCB main `f539d9a61b1fe1e9fab13f4e976c872b09b8fb9a`, [fresh capture and normal-check evidence](/private/tmp/tcb-main-capture-fec38841-evidence/README.md) covers 104 IDs and 106 strictly decoded binary payloads, with two variants each retained for `showcase_page_diff` and `tablepro_clean_picker`. All 119 distinct producer tests passed with capture bypass unset against 104 isolated selected files. Independent Sol review verified hashes, counts, clean baseline diffs and exact source; this used a local path override to clean merged-SHA source, and selections are not visual approval.

At PR #2 head `60b5fb2762d91a3549bd4ac63b1d77d534ee53d2`, [fresh capture and normal-check evidence](/private/tmp/tcb-pr2-reference-capture-fec3884-20261106/run-evidence/README.md) covers 104 active candidates, a separate TablePro replacement, five superseded IDs and one separate held candidate. All 94 selected normal producer tests passed; six of seven G05 checks passed, with the remaining receipt check failing on the legacy JSON digest. Independent Sol review verified logs, candidate hashes, and unchanged 1,981-file baseline ledger. Session metadata still contains old comparator and lockfile values from the reference manifest and is not valid provenance for this run; current acceptance receipts must be regenerated. Neither rehearsal approves candidates, establishes reference-binary equivalence, nor validates other platforms.

## Jackin PR #1264 head 79a948d refresh

The live head advanced to `79a948d4bf941f4dfd14ae557c112701c1c89ab8`. In isolated preparation above that head, using TCB `f539d9a` and Tuiscotti `fec38841`, fresh capture produced 57 IDs from 44 passing journey tests; all 57 decoded with matching IDs and State gates. The full normal identity suite passed 49/49 serially against those temporary samples, including the mutation rejection test. Independent Sol review verified raw logs, source ancestry and before/after hashes. A preceding parallel normal run passed 46/49, with three capsule tests timing out before visual comparison; serial success does not establish parallel reliability. The temporary preparation changes coverage tests and uses a local TCB overlay. No candidate was approved and no shared PR or active baseline root changed. [Evidence](/private/tmp/jackin-pr1264-binary-identity-79a948d-20261106/run-evidence/README.md).

## Release workflow security work

Repository Actions setting `can_approve_pull_request_reviews` was enabled with default permissions left at `read`, addressing the observed PR-creation 403; the release workflow was not rerun. A standalone bounded release-artifact validator exists uncommitted at `/tmp/velnor-secure-release-impl` and passed four focused tests and independent Sol review. It is not wired to a workflow or publisher, and does not establish semantic correctness, safe filesystem use or remote freshness. Semver checking remains absent from the shipped release workflow; a credential-free analysis and trusted publisher split is not yet implemented.

## PR #2 provenance generator correction

In `/private/tmp/tcb-pr2-merged-pin-20261106`, the prepared reference manifest binds the merged comparator SHA and current Cargo.lock digest while retaining the reference app identity and old receipts. Independent review found the generator had a truncated lock hash; it now computes SHA-256 from `Cargo.lock` and has a test against the manifest. The focused session test and two Python tests passed, and independent re-review found no remaining provenance issue. This remains uncommitted preparation; the historical acceptance-receipt check still fails until valid new receipts are produced through reviewed acceptance.

## Release artifact materialization prototype

The uncommitted `/tmp/velnor-secure-release-impl` prototype now includes bounded producer/validator and atomic no-replace materialization with descriptor-anchored paths and owned-stage cleanup. Scoped tests and independent reviews cover hostile Git filters, symlinks, base digest changes, concurrent collisions and cleanup. The parent directory and independent Git object store require trusted provenance; no atomic snapshot, semantic release validation, clean producer checkout, publisher, workflow wiring or Ubuntu 26.04 qualification is established. These components do not yet fix the shipped release workflow or its semver warning.

## Tokenless release-plz update experiment

A disposable credential-free release-plz 0.4.3 update at Tuiscotti main `6d01628170de8bed3f00f52713f3a510b782e41e`, with cargo-semver-checks 0.51.0, selected library 0.5.0 (breaking API) and CLI 0.4.1. It changed both manifests and Cargo.lock and created two changelogs. Fixture before/after files are in `/tmp/velnor-secure-release-impl/crates/velnor-actions-contract/tests/fixtures/tuiscotti-release-6d016281/`; source checkout is `/tmp/velnor-tuiscotti-tokenless/source`. This is an unshipped experiment, not a release or a complete semantic validation. Independent review verified the structural creation logic on fixture bytes but could not independently verify the fixture provenance SHA in its workspace. No credentialed release-pr rerun or package publication occurred.

## Release workspace-manifest semantic prototype

An uncommitted standalone workspace-manifest gate in `/tmp/velnor-secure-release-impl` validates the captured real Tuiscotti version and local dependency requirement changes against independently supplied authorized versions. It rejects unrelated/source changes and checks required dependency updates across unchanged manifests. Eight focused tests and independent review/mutation checks passed on macOS. It is not wired into a workflow; caller-provided source/membership/version authority, lockfile and changelog semantic checks, and the trusted publisher remain unresolved. Unsupported workspace patterns fail closed.

## Release lockfile semantic prototype

An uncommitted standalone lockfile gate in `/tmp/velnor-secure-release-impl` accepts the captured real Tuiscotti update against an independently supplied workspace version map, and rejects unauthorized external identity, checksum and graph changes and ambiguous local rebindings. Seven focused tests passed on macOS; independent review prompted and verified rejection of unknown package fields. The validator is not wired into a release workflow, and trusted version authority, changelog semantics, credential separation, publisher and Ubuntu 26.04 qualification remain outstanding.

## PR #2 merged-pin preparation bundle

The local preparation now ends at signed commit `f08195c07e9c3dff56559995825a41eef81f0489`, above `6ca1e23ef713382ada84a098a718e6646272fbfc`. It includes the merged Tuiscotti pin and reviewed reference-manifest/generator correction, without changing baselines, receipts or frozen history. The verified bundle `.audit/pr2-merged-pin-f08195c.bundle` has SHA-256 `b81333aff78b4c04f11ec2eec652dd37eb2ebf48e49f6b21181acb22515f2096`; fresh-clone ancestry and clean checkout were checked. It has not been pushed to the owner's PR.

## Static candidate review material

The [candidate review bundle](static-candidate-review-fec38841/INDEX.md) contains 269 exact captured payloads with per-candidate IDs, SHA-256 digests, and CLI check reports. It has 264 candidate renders; five fail with typed `MISSING_GLYPH` for U+26B7, which the current pinned font contract explicitly requires when no selected font covers a glyph. Independent review verified candidate bytes, all 1,366 checksum entries, corrected held-button legacy path and PR #2 prep provenance. Legacy JSON comparisons are not provided; the current binary-only CLI rejects JSON. The bundle does not approve or accept candidates.

## Missing-glyph review evidence

A read-only search found no existing terminal screenshots or other faithful raster/vector images for the five U+26B7 candidates in the bundle or associated capture roots. Their binary files, exact IDs and hashes are preserved, but missing-baseline HTML reports are not screen renderings. The five renders remain unavailable under the current pinned-font missing-glyph contract; no converter, host font or substituted glyph was introduced.

## Legacy-to-fresh read-only comparison

The [dual-decoder comparison](legacy-vs-fresh-fec38841/README.md) reads legacy JSON with the pre-change `d319009` public decoder and fresh Protobuf with merged `fec38841`; it does not convert or accept samples. TCB main: all 106 candidate payloads match old state digests and decoded Debug domain after excluding snapshot metadata. PR #2: 103 of 105 paired candidates match state digest; `holla_first_use_idle` and `holla_help` differ by a single footer age digit (`3` to `2`), and the syntax-error candidate has no old JSON. Jackin: 57/57 paired state digests match. Independent review confirmed the PR #2 and Jackin full decoded Debug domains agree for the other 160 pairs after removing only snapshot metadata, and checked candidate/legacy hashes and no-write evidence. Debug projections are implementation-formatted evidence, not byte-identical sample files; capture metadata differs and no candidate is approved.

## Proposed exact candidate selection

The [review-only selection](candidate-selection-fec38841-20261107/README.md) binds 269 payloads and five unresolved superseded records to exact hashes and paths. It proposes the TCB main variants used in the 119-test rehearsal, retains both alternatives, and leaves PR #2 replacement/held/dispositions pending. Independent Sol review verified hashes, counts, uniqueness and mappings; the Jackin 79a948d→5076ca2 comparison was independently checked via GitHub API and contains only research/plan/evidence documents. This is not an approval or an acceptance instruction.

## Preserved Velnor release prototype

The uncommitted `/tmp/velnor-secure-release-impl` prototype at base `8bf44dd649` is preserved as [tracked patch](velnor-release-prototype-8bf44dd/tracked.patch) (SHA-256 `81dcccba1f13cd9f5fa10956325215827c1b5dc7ee931cac4d6f006de376e496`) and [untracked archive](velnor-release-prototype-8bf44dd/untracked.tar.gz) (SHA-256 `947fce647d0114eed0cee1f8a744d2382f8440789f44c15958377ee0d48ff256`). Restoring both into a fresh detached worktree reproduced all 51 changed files byte-for-byte and passed `git diff --check`. This is an incomplete, unwired prototype; the pinned published Velnor binary lacks its operations, and producer/publisher workflow isolation, artifact handoff and Ubuntu 26.04 qualification remain outstanding. No package release or PR was created.

## Velnor source split after branch-prefix review

The later uncommitted release-split source at base `8bf44dd64995ce7cb364c8c7b6b5592dc72c2460` is preserved in [tracked patch](velnor-release-split-final-8bf44dd/tracked.patch) (SHA-256 `7e2f77ea3c29d31bb7fe0454f6f1874816226b6c10fbf668e1be084917488745`) and [untracked archive](velnor-release-split-final-8bf44dd/untracked.tar.gz) (SHA-256 `b2c0ac29a7fe7a9eac9d31c222c0e7f7fbf615d0d22c7fed9cabe8cf1d7acfed`). Fresh restoration reproduced 78 changed files byte-for-byte and passed diff-check. Independent review verified the new `release-plz-velnor-` branch prefix against an actual 0.4.3 localhost dry-run; earlier independent review found no confirmed split-boundary blocker. This is not shipped: the current published Velnor binary lacks its operations, true Ubuntu 26.04 qualification is missing, and full local gates fail due to unavailable genuine rust-build-guard and stale dogfood goldens. No package release or additional PR was made.

## Missing-glyph cell evidence

The [static cell evidence](missing-glyph-cell-evidence-20261107/README.md) exports the five candidates that cannot render with the pinned fonts as text rows and complete decoded cell/style/palette/cursor data, using the merged `fec38841` public decoder in a disposable probe. Independent Sol review re-decoded all five, verified their hashes and exact 120×40/4,800-cell mappings, and prompted correction of a duplicate JSON key; final outputs and checksums pass. This is not a pixel render, visual equivalence claim or candidate approval. Sample round-trip hashes demonstrate sample decode/encode identity, not reconstruction from exported JSON.
