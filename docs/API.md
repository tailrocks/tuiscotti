# Public Rust API

Two shipping crates. Every production module owns one
responsibility in the capture-to-approval workflow; nothing else
ships. Status labels: **implemented** (shipped, tested),
**partial** (works with documented gaps), **unsupported**
(explicitly rejected, fails closed), **future** (planned, absent).

Live terminal APIs (session, input, locators, waits, capture,
rendering, pixel comparison) come from upstream `tui-test` and are
called directly from application test code. They are not listed here
because tuiscotti does not wrap or re-export them. The split is
summarized in [COMPARISON.md](COMPARISON.md).

## `tuiscotti`: Ratatui view capture (`view`)

| Item | Status | Notes |
|---|---|---|
| `view::capture_view` | implemented | Production draw closure + `ViewSettings` → upstream `TerminalSnapshot` through a Ratatui `TestBackend`. Starts no shell, child, PTY, daemon, or session. |
| `view::snapshot_from_buffer` | implemented | Caller-supplied Ratatui buffer + explicit `ViewCursor` → `TerminalSnapshot`. |
| `view::{ViewSettings, ViewCursor, ViewError, ViewResult}` | implemented | Typed inputs and failures; upstream validation errors are retained, not stringified. |

## `tuiscotti`: visual samples (`sample`)

| Item | Status | Notes |
|---|---|---|
| `sample::Sample` (`tuiscotti.sample.v1`) | implemented | Versioned domain value binding the test ID, gate, upstream snapshot, render contract, PNG image, and observation. `to_bytes` and `from_bytes` use the binary sample container. `with_pixels` builds `state-and-pixels`; `state` builds the `state` gate. |
| `sample::{Gate, RenderContract, SampleImage, Observation}` | implemented | `state` compares canonical state only; `state-and-pixels` also compares render contract and decoded RGBA. |
| `sample::{SampleError, Sample::candidate_id}` | implemented | Binary reads enforce the container, message, and resource limits. The decoder rejects JSON, unknown fields, unsupported versions, and malformed data. |

## `tuiscotti`: frozen checks (`check`)

| Item | Status | Notes |
|---|---|---|
| `check::FrozenStore::{new, check, check_owned, check_file}` | implemented | Read-only baselines; candidates and evidence go to the artifact root only. `check` borrows bytes, `check_owned` accounts for the owned input allocation, and `check_file` uses the shared bounded read. Outcomes: `pass`, `missing`, `different` with typed `MismatchReason`. |
| `check::{CheckError, CheckFailure, CheckSuccess, MISSING_BASELINE, validate_test_id}` | implemented | Failures carry evidence paths; test IDs are validated ASCII segments. |
| Ambient bless (`BLESS=1`, auto-accept) | unsupported | Removed; explicit `accept` with both digests is the only approval path. |

## `tuiscotti`: approval (`approve`)

| Item | Status | Notes |
|---|---|---|
| `approve::ApprovalStore::accept` | implemented | Digest-arbitrated transaction: takes the candidate path, candidate SHA-256, and expected baseline (`missing` for first approval). Validates the candidate test ID unconditionally, re-renders image samples through the upstream renderer with render-identity verification (recorded font/renderer/geometry/policy identity must match the re-render; a foreign platform key alone never rejects reproducible pixels), and commits the baseline with one atomic rename. Rejects inconsistent identity or pixels, unknown profiles, stale digests, and malformed input with typed codes. Full semantics: [SNAPSHOTS.md](SNAPSHOTS.md). |
| `approve::{ApproveError, AcceptOutcome, sample_code}` | implemented | Machine-readable codes and the in-memory change summary shared with the CLI; CI refusal is `approval is forbidden while CI is set`. |
| Approval under `CI` | unsupported | Any set `CI` value refuses before any write. No override exists. See [SNAPSHOTS.md](SNAPSHOTS.md). |

## `tuiscotti`: reports (`report`)

| Item | Status | Notes |
|---|---|---|
| `report::{render_report, escape_html}` | implemented | Static HTML report for one comparison. Writes nothing but the report file. |
| `report::{ReportInput, ReportedCell, PixelSummary, ReportError}` | implemented | Typed inputs and failures for offline rendering. |
| `report::{write_report_index, IndexOutcome, IndexError}` | implemented | Static HTML index over an artifact root; links resolve from the index parent. Code is always `IO`. |

## `tuiscotti`: offline rendering (`render`)

| Item | Status | Notes |
|---|---|---|
| `render::{render_sample, RenderOutcome, RenderError}` | implemented | Offline `.png`/`.html` rendering of saved sample bytes with typed codes; the `render` CLI command delegates to it. |

## `tuiscotti`: render profile (`render_profile`)

| Item | Status | Notes |
|---|---|---|
| `render_profile::{PROFILE_NAME, resolve_profile, platform_key}` | implemented | One logical profile, `tuiscotti-pinned-v1`. Every other name is `UnknownProfile`. No custom font paths, no network loading. |
| `render_profile::{Profile, ProfileError, PinnedFace, ...}` | implemented | Pinned upstream raster policy, licensed downstream font bytes (JetBrains Mono Nerd Font Mono, DejaVu symbol, CJK subset; 16 px font, 10×20 cells, scale 1). |

## `tuiscotti`: errors (`error`)

| Item | Status | Notes |
|---|---|---|
| `error::{Error, Result}` | implemented | One typed error over `view` / `sample` / `check` / `report` / `render_profile` / `approve` / I/O, with sources retained. Live failures surface through upstream `tui-test` error types, never through this facade. |

## `tuiscotti-cli`: offline binary (`main.rs` and its ops)

| Item | Status | Notes |
|---|---|---|
| `check`, `accept`, `render`, `report` | implemented | The only four commands; all offline. See [CLI.md](CLI.md). |
| `input`, `json_out`, `cli` | implemented | Bounded binary sample reads, one JSON result shape (`tuiscotti.cli.v1`), native argument parsing. Result JSON remains separate from sample storage. |

## Unsupported (removed — no API, no shim)

Termpane/PTY transport, workers, reader threads, reaping, terminal
parsing, key/mouse encoders, live locators, wait loops, daemon,
session registry, op protocol, MCP server, recorders, video, piped
capture, shell execution, local rasterizer, local renderer, old
`Store`/`GroupedStore`/generation tags, ANSI baselines, artifact
import, old snapshot schemas, aliases, frozen-tree import/export,
semantic-provider and manual-clock frameworks, sixel/Kitty
inspection, and the second approval engine are all **unsupported**:
naming any of them fails to compile, and there is no compatibility
re-export.
