# Task board

Status values: `todo`, `active`, `review`, `done`. Owners: primary agent, upstream writer, downstream writer, code reviewer.

| Task | Subject | Owner | Status | Branch | Base SHA |
|---|---|---|---|---|---|
| P01 | Source, instructions, branches, base behavior | primary agent | done | `goal/tui-test-extension` | `a1ea27f757d75d07b05499276cd932e9f8e067ad` |
| P02 | Corpus, API record, task records | primary agent | active | `goal/tui-test-extension` | `a1ea27f757d75d07b05499276cd932e9f8e067ad` |
| U01 | Strict upstream snapshot policy | upstream writer | done ([draft PR 276](https://github.com/microsoft/tui-test/pull/276)) | `goal/u01-strict-snapshots` | `f2760592ed00e90436f8c3bae124dab712eaecf8` |
| U02 | Owned snapshot and atomic capture | upstream writer | done ([draft PR 278](https://github.com/microsoft/tui-test/pull/278)) | `goal/u02-owned-snapshot` | `f2760592ed00e90436f8c3bae124dab712eaecf8` |
| U03 | Public offline rendering | upstream writer | done ([draft PR 280](https://github.com/microsoft/tui-test/pull/280)) | `goal/u03-offline-rendering` | U02 head (`364719a`, parent PR #278) |
| U11 | Explicit underline color | upstream writer | done ([draft PR 283](https://github.com/microsoft/tui-test/pull/283)) | `goal/u11-underline-color` | U03 head (`6a3c88f`, parent PR #280) |
| U04 | Pinned fonts and render evidence | upstream writer | done ([draft PR 284](https://github.com/microsoft/tui-test/pull/284)) | `goal/u04-pinned-fonts` | U03 head (`6a3c88f`, parent PR #280) |
| U05 | Grapheme rendering | upstream writer | done ([draft PR 285](https://github.com/microsoft/tui-test/pull/285)) | `goal/u05-graphemes` | U04 head (`3c37c6a`, parent PR #284) |
| U06 | Pure pixel comparison | upstream writer | done ([draft PR 277](https://github.com/microsoft/tui-test/pull/277)) | `goal/u06-pixel-comparison` | `f2760592ed00e90436f8c3bae124dab712eaecf8` |
| U07 | Checked mouse, paste, focus | upstream writer | done ([draft PR 282](https://github.com/microsoft/tui-test/pull/282)) | `goal/u07-checked-input` | `f2760592ed00e90436f8c3bae124dab712eaecf8` |
| U08 | Child environment, native arguments | upstream writer | done ([draft PR 286](https://github.com/microsoft/tui-test/pull/286)) | `goal/u08-launch-options` | `f2760592ed00e90436f8c3bae124dab712eaecf8` |
| U09 | Raw input bytes | upstream writer | done ([draft PR 279](https://github.com/microsoft/tui-test/pull/279)) | `goal/u09-raw-bytes` | `f2760592ed00e90436f8c3bae124dab712eaecf8` |
| U10 | Quiet intervals, request cancellation | upstream writer | done ([draft PR 281](https://github.com/microsoft/tui-test/pull/281)) | `goal/u10-request-waits` | U02 head (`364719a`, parent PR #278) |
| D01 | Reduced crates, Ratatui adapter | downstream writer | done (pure-view conversion accepted; CoveredCellOccupied/LeadingCellOverhang follow-up reviewed, suites green since) | `goal/tui-test-extension` | U02 head (patched) |
| D02 | Sample format, render profiles | downstream writer | done | `goal/tui-test-extension` | D01 + U03/U04/U05/U11 heads |
| D03 | Comparison and candidate evidence | downstream writer | done | `goal/tui-test-extension` | D02 + qual-stack `fb72d00` |
| D04 | Approval and frozen checking | downstream writer | done | `goal/tui-test-extension` | D03 head |
| D05 | Offline CLI | downstream writer | done | `goal/tui-test-extension` | D04 head |
| D06 | Test and example migration | downstream writer | done | `goal/tui-test-extension` | D05 + U07/U08/U09/U10 heads |
| D07 | Code removal and documentation | downstream writer | done | `goal/tui-test-extension` | D06 head |
| V01 | Full checks and last code review | code reviewer | done (re-review accepted `18d57ad`: guides match two-crate tree, qual-stack record corrected, 167/167 nextest green) | both repos | D07 head |
| R01 | Focused upstream PRs | primary agent | active (U01 #276, U06 #277, U02 #278, U09 #279, U03 #280, U10 #281, U07 #282, U11 #283, U04 #284, U05 #285, U08 #286) | one branch per U task | accepted heads |
| R02 | Released dependency adoption | primary agent | todo | `goal/tui-test-extension` | V01 + release |
| F01 | Last evidence and completion report | primary agent | todo | `goal/tui-test-extension` | all local tasks |

R01 starts for each accepted upstream task before the full migration ends. Dependent PRs stay in a recorded stack. U01 is not a prerequisite for the downstream approval engine.
