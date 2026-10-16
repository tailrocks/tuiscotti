# Limitations

Honest gaps at head `75ff479`. Labels: **tested** (ran green),
**compiles-only**, **not run**, **unsupported** (explicitly out of
scope, fails closed where applicable).

## Platform matrix

| Platform | Status | Evidence |
|---|---|---|
| macOS (aarch64) | tested | Full nextest 519/519 green (~21 s) at head `75ff479`, 2026-09-29. See PERFORMANCE.md. |
| Linux (x86_64) | tested in CI | CI lanes run `ubuntu-24.04` (`platform = "linux-x64"` in `.github/ci/project.toml`); latest `CI / PR` green incl. nextest + doctests. |
| Windows (ConPTY) | compiles-only, not run | `cargo check --target x86_64-pc-windows-gnu --tests`: 0 errors. `termpane` =0.1.0 (via its `portable-pty` transport) ships a ConPTY backend, but no Windows test process has ever executed and no Windows CI lane exists. Windows is never green until a CI lane runs it. |

Unix-only code that degrades or compiles out on Windows (paths at
head; tree is moving):

- `tui/` — `Signal::number` (`termpane::process::SIG*`) is `#[cfg(unix)]`.
- `tui_shell/` — unix guardian/process-group machinery is
  `#[cfg(unix)]`; `#[cfg(not(unix))]` stubs return `None` /
  `Containment::Unsupported` explicitly, not silently.
- `command/` — non-unix `classify` maps to `Termination::Exit`,
  never invents a signal.
- `proto/` — unix / non-unix session paths.
- `tests/piped.rs` — signal-death test is `#[cfg(unix)]`.
- `tests/cli.rs` — owner-only (0o700) runtime-dir and chmod blocks
  are `#[cfg(unix)]`; skipped, not asserted, on Windows.

For a truthful Windows claim, CI needs a `windows-latest` lane
running the same unit commands. Until then: "compiles for
`x86_64-pc-windows-gnu`; ConPTY path unverified".

## Render fidelity

- Terminal-like, measured fidelity — NOT pixel-identity with any
  terminal emulator; cell data stays authoritative for styles.
- Codepoints no vendored face covers (color emoji, Hangul, JIS
  level-2 kanji) render as deterministic tofu with correct advance
  AND are reported in `<name>.png.fidelity.json` next to every PNG.
  Covered: box drawing, blocks, Braille, Nerd icons, combining
  marks, Geometric Shapes / Misc Symbols / Dingbats subsets, kana +
  JIS X 0208 level-1 kanji + fullwidth forms. See
  `assets/fonts/FONTS.md`.
- Blink phase is frozen visible; slow/rapid rates stay combined.
  Overline is dropped (O17). Raw-ANSI replay cannot observe cursor
  appearance (block/steady cursor) — use PTY captures for cursor
  shape/blink assertions, or `render::ansi_dump_with_cursor` +
  `formats::ansi::frame_from_ansi` for normalized `.ansi` captures with
  a canonical VT cursor trailer.
- **Normalized `.ansi` cross-cell grapheme clusters
  (`formats::ansi::frame_from_ansi`)**: `render::ansi_dump` inserts an
  `ESC[0m` reset (followed by the active cell SGR) whenever two adjacent
  cells' symbols on a row would otherwise join into a single UAX #29
  extended grapheme cluster (e.g. adjacent regional-indicator flag
  cells, `'e'` followed by a cell containing `U+0301`, an emoji ZWJ
  sequence split across single-grapheme cells, or adjacent Hangul L+V
  jamo cells), so `frame_from_ansi` decodes each cell without joining.
  In addition, `GroupedStore::accept` in `ArtifactSet::AnsiCursor` mode
  enforces lossless round-trip (`ansi_dump_with_cursor(&decoded) == text`)
  and rejects any `.ansi` candidate that does not round-trip identically.
- No scrollback reflow on resize (emulator constraint).

## Emulator honesty gaps

- Unknown escape sequences are silently ignored; there is no
  unhandled-sequence reporting surface.
- The adapter owns input encoding (keys/mouse/paste,
  `tui/encode.rs`/`encode_key.rs`), qualified by
  `tool_qualification` + PTY matrix tests; query replies ride the
  emulator's `drain_passthrough` (`tui/capture.rs`).
- Per-cell blink is tracked by the emulator (O16 closed at the
  swap); cursor blink still readable via cursor style.
- Backend gaps inherited from `termpane` =0.1.0 (IDs from
  TERMPANE-SWAP-PLAN.md §2): O5 — DEC modes 4 (IRM), 6 (DECOM),
  20 (LNM) are absorbed untracked, never in `TermState.modes`;
  O6 — OSC 4 palette overrides are dropped, entries resolve to
  nominal defaults (live + replay pinned); O7 — OSC 10/11 set
  forms are dropped, default-color overrides never observed;
  O13 — CSI 14/16/18t (text-area size) are never answered.

## Mouse DEC mode requirements

Mouse input fails closed with `TuiError::ModeNotEnabled` (or
`ActionError::Session(TuiError::ModeNotEnabled(...))` on
`BoundLocator::click`) unless the application under test has enabled
the required tracking mode on the live PTY surface:

| Method | Required DEC tracking mode | Encoding | Error when mode is off |
|---|---|---|---|
| `Session::click` / `BoundLocator::click` / `mouse_down` / `mouse_up` / `mouse_wheel` | `DECSET 1000` (`\x1b[?1000h`, normal press/release), `1002` (`\x1b[?1002h`, button-event drag), or `1003` (`\x1b[?1003h`, any-event motion) | SGR `DECSET 1006` (`\x1b[?1006h`, recommended for coords > 223 and distinct press/release), UTF-8 `1005`, or X10 | `TuiError::ModeNotEnabled("mouse reporting (DEC 1000/1002/1003) not enabled by the application")` |
| `Session::mouse_drag` | `DECSET 1002` or `1003` (`1000` alone is rejected) | SGR `1006` (recommended), UTF-8 `1005`, or X10 | `TuiError::ModeNotEnabled("mouse drag reporting (DEC 1002/1003) not enabled by the application")` |
| `Session::mouse_move` (hover without buttons) | `DECSET 1003` (`\x1b[?1003h`; `1000` or `1002` alone is rejected) | SGR `1006` (recommended), UTF-8 `1005`, or X10 | `TuiError::ModeNotEnabled("mouse motion reporting (DEC 1003) not enabled by the application")` |

## Clock and Time Control in PTY Subprocesses

`tui::Tui` spawns a real OS child process attached to a kernel PTY.
Because the child executes in its own address space and schedules its
own timers and syscalls (`clock_gettime`, `epoll`/`kqueue`), the test
harness cannot inject a virtual Rust clock into an arbitrary child
binary over a PTY, and `Session::wait_stable_quiet` measures real
wall-clock PTY byte silence on the reader thread.

For deterministic visual baselines of applications with animations,
blinking cursors, spinners, or wall-clock status bars, use a two-part
pattern:

1. **App-side deterministic clock / tick env convention**: read an
   environment variable in the binary under test — passed via
   `Tui::new(cmd).env("TUISCOTTI_FIXED_TIME", "2026-01-01T00:00:00Z")`
   or `Tui::new(cmd).env("APP_TICK_MS", "0")` (or a key-driven step
   mode) — so wall-clock timestamps, spinners, and periodic ticks
   freeze or advance only on explicit input during visual baseline
   runs. Note: on Unix, the underlying PTY spawner (`portable-pty` via
   `termpane`) populates `SHELL` from the user's `passwd` entry when
   `SHELL` is unset in the child environment, so callers that render or
   branch on `$SHELL` should pass `.env("SHELL", "/bin/sh")` alongside
   `.env_clear(true)`.
2. **Wait-side semantic readiness + short quiet window**: wait for an
   explicit application readiness marker first
   (`session.get_by_text("Ready").any().expect_visible()?` or
   `wait_predicate`), then optionally call a short
   `session.wait_stable_quiet(deadline, Duration::from_millis(50), &cancel)?`
   to let trailing frame bytes flush — never use a large
   `wait_stable_quiet(400ms)` alone as a substitute for semantic
   readiness.

## Scope

- Foreign-language SDKs (Python/JS/TS clients) are out of scope:
  the transport is Rust-only (`tuiscotti` + `tuiscotti machine`).

## Security posture

- This is a research project: unsafe, breaking changes expected,
  never production-ready.
- Hidden text is conceal, not redaction — canonical JSON retains
  the original symbol. Never capture real secrets
  (`redact_frame`/`redact_screen` exist for scrubbing, but the
  safe default is to not capture secrets at all).
- `inspect`/`trace`/`import`/`review`/`report` never execute
  directory contents; `capture`/`record`/`session start` execute
  only the argv you pass.
- Session runtime dirs are owner-only (unix); `TUISCOTTI_RUNTIME_DIR`
  overrides the location.
