# API record

P02 API record for the migration. Proposed names follow GOAL Section 8.2. No name conflict exists at the base commit, so no replacement mapping was necessary before dependent changes.

## View palette (fixed)

Source: in-memory `AlacrittyEmu::new(80, 24, &Profile::default())`, read through `Emulator::colors()`. No child, PTY, or session was started. Probe: `/tmp/palette-probe` (throwaway, not committed). Corroborated by `Colors::default()` in `crates/tui-test/src/profile.rs`.

- Foreground: `#c0c0c0` (192, 192, 192)
- Background: `#000000` (0, 0, 0)
- Cursor: `#c0c0c0` (192, 192, 192)
- Palette: classic VGA/xterm 256 entries (16 ANSI slots, 216 color cube, 24 gray ramp), all recorded:
  - `0-7: #000000 #800000 #008000 #808000 #000080 #800080 #008080 #c0c0c0`
  - `8-15: #808080 #ff0000 #00ff00 #ffff00 #0000ff #ff00ff #00ffff #ffffff`
  - `16-23: #000000 #00005f #000087 #0000af #0000d7 #0000ff #005f00 #005f5f`
  - `24-31: #005f87 #005faf #005fd7 #005fff #008700 #00875f #008787 #0087af`
  - `32-39: #0087d7 #0087ff #00af00 #00af5f #00af87 #00afaf #00afd7 #00afff`
  - `40-47: #00d700 #00d75f #00d787 #00d7af #00d7d7 #00d7ff #00ff00 #00ff5f`
  - `48-55: #00ff87 #00ffaf #00ffd7 #00ffff #5f0000 #5f005f #5f0087 #5f00af`
  - `56-63: #5f00d7 #5f00ff #5f5f00 #5f5f5f #5f5f87 #5f5faf #5f5fd7 #5f5fff`
  - `64-71: #5f8700 #5f875f #5f8787 #5f87af #5f87d7 #5f87ff #5faf00 #5faf5f`
  - `72-79: #5faf87 #5fafaf #5fafd7 #5fafff #5fd700 #5fd75f #5fd787 #5fd7af`
  - `80-87: #5fd7d7 #5fd7ff #5fff00 #5fff5f #5fff87 #5fffaf #5fffd7 #5fffff`
  - `88-95: #870000 #87005f #870087 #8700af #8700d7 #8700ff #875f00 #875f5f`
  - `96-103: #875f87 #875faf #875fd7 #875fff #878700 #87875f #878787 #8787af`
  - `104-111: #8787d7 #8787ff #87af00 #87af5f #87af87 #87afaf #87afd7 #87afff`
  - `112-119: #87d700 #87d75f #87d787 #87d7af #87d7d7 #87d7ff #87ff00 #87ff5f`
  - `120-127: #87ff87 #87ffaf #87ffd7 #87ffff #af0000 #af005f #af0087 #af00af`
  - `128-135: #af00d7 #af00ff #af5f00 #af5f5f #af5f87 #af5faf #af5fd7 #af5fff`
  - `136-143: #af8700 #af875f #af8787 #af87af #af87d7 #af87ff #afaf00 #afaf5f`
  - `144-151: #afaf87 #afafaf #afafd7 #afafff #afd700 #afd75f #afd787 #afd7af`
  - `152-159: #afd7d7 #afd7ff #afff00 #afff5f #afff87 #afffaf #afffd7 #afffff`
  - `160-167: #d70000 #d7005f #d70087 #d700af #d700d7 #d700ff #d75f00 #d75f5f`
  - `168-175: #d75f87 #d75faf #d75fd7 #d75fff #d78700 #d7875f #d78787 #d787af`
  - `176-183: #d787d7 #d787ff #d7af00 #d7af5f #d7af87 #d7afaf #d7afd7 #d7afff`
  - `184-191: #d7d700 #d7d75f #d7d787 #d7d7af #d7d7d7 #d7d7ff #d7ff00 #d7ff5f`
  - `192-199: #d7ff87 #d7ffaf #d7ffd7 #d7ffff #ff0000 #ff005f #ff0087 #ff00af`
  - `200-207: #ff00d7 #ff00ff #ff5f00 #ff5f5f #ff5f87 #ff5faf #ff5fd7 #ff5fff`
  - `208-215: #ff8700 #ff875f #ff8787 #ff87af #ff87d7 #ff87ff #ffaf00 #ffaf5f`
  - `216-223: #ffaf87 #ffafaf #ffafd7 #ffafff #ffd700 #ffd75f #ffd787 #ffd7af`
  - `224-231: #ffd7d7 #ffd7ff #ffff00 #ffff5f #ffff87 #ffffaf #ffffd7 #ffffff`
  - `232-239: #080808 #121212 #1c1c1c #262626 #303030 #3a3a3a #444444 #4e4e4e`
  - `240-247: #585858 #626262 #6c6c6c #767676 #808080 #8a8a8a #949494 #9e9e9e`
  - `248-255: #a8a8a8 #b2b2b2 #bcbcbc #c6c6c6 #d0d0d0 #dadada #e4e4e4 #eeeeee`.
- Canonical digest: `92757b147e00dec394b10b7be2fa42f782f0d30325a5922570080f91d60f641e` over sorted-key JSON `{"background","cursor","foreground","palette"}` with `#rrggbb` strings.

Do not recalculate this palette from a later dependency version. Copy these values into the snapshot for the built-in view profile.

## Proposed upstream API vs base-commit equivalents

| Task | Proposed API | Equivalent at `f276059` | Decision |
|---|---|---|---|
| U01 | `compare_with_policy`, `CheckOnly`/`CreateMissing`/`Replace`, `Equal`/`Missing`/`Different`/`Created`/`Replaced` | `compare()` with `update: bool`, `SnapshotStatus::{Passed,Written,Updated,Failed}` | Add new API; keep `compare()` as a compatibility wrapper |
| U02 | `TerminalSnapshot::try_new`, `Session::capture_snapshot`, capture scopes | `Engine::frame()` returns `LiveFrame` (monitor only, unowned, no validation) | Add owned snapshot with validation; keep `frame()` behavior |
| U03 | `render_snapshot_svg`, `render_snapshot_png`, `SnapshotRenderOptions` | `render/svg.rs` and `render/raster.rs` used by live screenshot and recording paths | Add offline functions over the shared renderer; no second renderer |
| U04 | `PinnedFontSet`, pinned policy in `SnapshotRenderOptions` | System-font lookup in `render/raster/font.rs`; bundled JetBrains Mono bytes are `pub(crate)` (`REGULAR_FONTS`, `STYLED_FONTS`) | Add pinned policy; expose only what U04 needs (bytes stay caller-supplied if no public accessor is added) |
| U05 | mandatory grapheme shaping | `unsupported_grapheme()` rejection in `render/raster/draw.rs` | Use `rustybuzz` if it meets MSRV/license/behavior; record version, license, MSRV |
| U06 | `compare_rgba` | none (only baseline-file snapshot compare) | Add pure function; no filesystem effects |
| U07 | `Session::mouse_checked`, `Session::paste`, `Session::focus` | raw `write_monitor_input_raw`, backend-aware key encoders | Add checked methods; keep raw methods unchanged |
| U08 | `Session::spawn_with`, `LaunchOptions` (OsString env/args, ordered ops) | `RunOptions`/`OpenOptions` string-based launch | Add new types; do not add fields to `RunOptions`/`OpenOptions` |
| U09 | `Session::write_bytes(&[u8])` | text input methods | Add additive method; keep text methods unchanged |
| U10 | `Session::wait_quiet` plus a session method with request cancellation | `Operation::WaitIdle`, `Engine::execute_with_context_cancellable()` | Add new options type; keep `WaitIdle` behavior. The cancellation method has no proposed name in Section 8.2; the U10 writer selects it and records the mapping before dependent changes. |
| U11 | underline color in shared paint data | underline style recorded; raster geometry shared | Keep captured underline color through paint conversion |

## Snapshot contract fields (Section 8.3) and resource limits (Section 8.4)

Snapshot fields: logical columns and rows; row-major cells and continuation cells; grapheme strings and cell widths; default, indexed, and RGB color references; all 256 resolved palette entries and foreground, background, and cursor defaults; recorded cell attributes and underline details; hyperlink URI and available hyperlink identity; cursor coordinates, visibility, shape, color, and available blink intent; window title; capture scope and viewport origin. Observation metadata (capture time, revision, backend identity, session identity, source kind) is not compared. Scopes: viewport, active-screen, full-history. Limits: encoded sample file 64 MiB; one sample 1,000,000 cells; one cell string 4,096 UTF-8 bytes; decoded RGBA 256 MiB; one image dimension 32,768 pixels; test ID 240 ASCII bytes; one test ID segment 64 ASCII bytes.

## Downstream entry points (names; record changes here before dependents use them)

`capture_view`, `snapshot_from_buffer`, `Sample::state`, `Sample::with_pixels`, `FrozenStore::check`, `ApprovalStore::accept`, `render_sample`, `write_report_index`. No renames recorded yet.

## Font record

All faces below are single-face files; the face index is 0 in every row (checked from file headers: `00 01 00 00` TTF and `OTTO` OTF magic, no TTC container).

| File | Source URL | Version | Face index | SHA-256 | License |
|---|---|---|---|---|---|
| Upstream `crates/tui-test/assets/jetbrains-mono/JetBrainsMono-Regular.ttf` (+ Bold, Italic, BoldItalic under `-styles`) | [JetBrains Mono](https://www.jetbrains.com/lp/mono/) | 2.305 (nameID 5) | 0 | (pinned by upstream release; bytes are `pub(crate)`) | OFL 1.1 (upstream tree) |
| Downstream `assets/fonts/JetBrainsMonoNerdFontMono-Regular.ttf` (+ Bold, Italic, BoldItalic) | [JetBrains Mono](https://www.jetbrains.com/lp/mono/) patched by [Nerd Fonts Mono](https://github.com/ryanoasis/nerd-fonts) | 2.304 + Nerd Fonts 3.5.1 (nameID 5) | 0 | Regular `f2a5ea6cfab397445ffab00c0370927b66d61e560a05db5db271b42006381c1a`; Bold `bfcf9a917276ffc058867d87cbc8a5b2f1ab0f4b710e9170dc02763ccb80bd4b`; Italic `31efd6ead98746f5b0afa1ee6dba60267ad48db36428360bee327bec10621f97`; BoldItalic `9dba502e00e35209f6ed2a151c7376c051657b067cdebbc6e52d06cb9002cf31` | `assets/fonts/LICENSE-JetBrainsMono.txt` (OFL 1.1) |
| Downstream `assets/fonts/NotoSansSymbols-subset.ttf` | [google/fonts](https://github.com/google/fonts) at `8b0a1d0f`, `ofl/notosanssymbols/NotoSansSymbols[wght].ttf` instantiated at wght 400, pyftsubset ranges per `assets/fonts/FONTS.md` | variable-font instance (name table subset) | 0 | `6f9cc93e71f8676361c5db286368be046e75d42c0b841afbf3f50da6bb0a2b8a` | OFL (Noto; see `assets/fonts/FONTS.md`) |
| Downstream `assets/fonts/NotoSansSymbols2-subset.ttf` | [google/fonts](https://github.com/google/fonts) at `7b6724ac`, `ofl/notosanssymbols2/NotoSansSymbols2-Regular.ttf` | subset (name table subset) | 0 | `e1d177a40af910100eceb0e825331e55f0cfd005bc0f26087fd4e58fbe60e6c5` | OFL (Noto; see `assets/fonts/FONTS.md`) |
| Downstream `assets/fonts/NotoSansCJKjp-subset.otf` | [noto-cjk](https://github.com/notofonts/noto-cjk) at `165c01b4`, `Sans/OTF/Japanese/NotoSansCJKjp-Regular.otf` | subset (name table subset) | 0 | `777bee41f0c6076c00ad919384359a6e396b8822cf9056041fca8fcf2759d897` (678988 bytes) | `assets/fonts/LICENSE-Noto.txt` |

U04 decides whether upstream exposes bundled bytes publicly; otherwise downstream keeps the licensed assets above under version control. Do not fetch fonts during a test, render, or approval.

- Upstream bundled symbol font: used per D02 fallback order position 3.
- Mandatory unsupported-glyph case: `U+1F980` (crab), expected missing from pinned fonts.
- Fallback order: styled JetBrains Mono face, JetBrains Mono regular, upstream bundled symbol font, pinned CJK subset. Private Use Area icons select the symbol font first.

## Previous behavior disposition

| Area | Recorded tests | Disposition | Requirement |
|---|---|---|---|
| Pure-view draw closure and buffer capture (`ratatui_views.rs`, `cells.rs`) | KEEP (shape changes to upstream snapshot) | C01, C04, C05 |
| Snapshot lifecycle, safety, approval (`snapshot*.rs`, CLI accept/render/report) | REPLACE with sample format, frozen checking, explicit digests | C09-C12, C17 |
| Live PTY sessions, locators, waits, mouse/key encoders (`tui*.rs`, `runner`, `bound_locator`, `waits`, `keys`, protocol, MCP) | REMOVE after D06 migration | C02, C13-C16 via upstream |
| Recorders, video, piped capture, shell execution | REMOVE | 3.3 |
| Local rasterizer, glyph shaping, font lookup, image renderer | REMOVE after U03-U06, U11 pass | 3.3 |
| `Store`, `GroupedStore`, generation tags, ANSI baselines, artifact import, frozen-tree import/export | REMOVE | 3.3 |
| Semantic providers, manual clocks, Sixel/Kitty inspection, second approval engine (Insta integration) | REMOVE | 3.3 |
| Insta example | REPLACE with ordinary Rust call if useful; no shipping adapter | D06 |

Fixtures: controlled Rust fixture programs for PTY tests; absolute fixture paths when `PATH` is cleared; no shell startup files, interactive terminals, or network services.
