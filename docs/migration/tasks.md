# Task board

Status values: `todo`, `active`, `review`, `done`. Owners: primary agent, upstream writer, downstream writer, code reviewer.

| Task | Subject | Owner | Status | Branch | Base SHA |
|---|---|---|---|---|---|
| P01 | Source, instructions, branches, base behavior | primary agent | done | `goal/tui-test-extension` | `a1ea27f757d75d07b05499276cd932e9f8e067ad` |
| P02 | Corpus, API record, task records | primary agent | active | `goal/tui-test-extension` | `a1ea27f757d75d07b05499276cd932e9f8e067ad` |
| U01 | Strict upstream snapshot policy | upstream writer | todo | `goal/u01-strict-snapshots` | `f2760592ed00e90436f8c3bae124dab712eaecf8` |
| U02 | Owned snapshot and atomic capture | upstream writer | todo | `goal/u02-owned-snapshot` | `f2760592ed00e90436f8c3bae124dab712eaecf8` |
| U03 | Public offline rendering | upstream writer | todo | `goal/u03-offline-rendering` | U02 head |
| U11 | Explicit underline color | upstream writer | todo | `goal/u11-underline-color` | U03 head |
| U04 | Pinned fonts and render evidence | upstream writer | todo | `goal/u04-pinned-fonts` | U03 head |
| U05 | Grapheme rendering | upstream writer | todo | `goal/u05-graphemes` | U04 head |
| U06 | Pure pixel comparison | upstream writer | todo | `goal/u06-pixel-comparison` | `f2760592ed00e90436f8c3bae124dab712eaecf8` |
| U07 | Checked mouse, paste, focus | upstream writer | todo | `goal/u07-checked-input` | `f2760592ed00e90436f8c3bae124dab712eaecf8` |
| U08 | Child environment, native arguments | upstream writer | todo | `goal/u08-launch-options` | `f2760592ed00e90436f8c3bae124dab712eaecf8` |
| U09 | Raw input bytes | upstream writer | todo | `goal/u09-raw-bytes` | `f2760592ed00e90436f8c3bae124dab712eaecf8` |
| U10 | Quiet intervals, request cancellation | upstream writer | todo | `goal/u10-request-waits` | U02 head |
| D01 | Reduced crates, Ratatui adapter | downstream writer | todo | `goal/tui-test-extension` | U02 head (patched) |
| D02 | Sample format, render profiles | downstream writer | todo | `goal/tui-test-extension` | D01 + U03/U04/U05/U11 heads |
| D03 | Comparison and candidate evidence | downstream writer | todo | `goal/tui-test-extension` | D02 + U06 head |
| D04 | Approval and frozen checking | downstream writer | todo | `goal/tui-test-extension` | D03 head |
| D05 | Offline CLI | downstream writer | todo | `goal/tui-test-extension` | D04 head |
| D06 | Test and example migration | downstream writer | todo | `goal/tui-test-extension` | D05 + U07/U08/U09/U10 heads |
| D07 | Code removal and documentation | downstream writer | todo | `goal/tui-test-extension` | D06 head |
| V01 | Full checks and last code review | code reviewer | todo | both repos | D07 head |
| R01 | Focused upstream PRs | primary agent | todo | one branch per U task | accepted heads |
| R02 | Released dependency adoption | primary agent | todo | `goal/tui-test-extension` | V01 + release |
| F01 | Last evidence and completion report | primary agent | todo | `goal/tui-test-extension` | all local tasks |

R01 starts for each accepted upstream task before the full migration ends. Dependent PRs stay in a recorded stack. U01 is not a prerequisite for the downstream approval engine.
