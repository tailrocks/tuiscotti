# tuiscotti

Offline visual-regression library for Ratatui views: render a view
in-process, freeze it as a versioned sample, compare it against an
approved baseline, and approve new baselines through an explicit
digest-arbitrated transaction.

This crate never spawns a process, never opens a terminal device, and
never touches the network. Live terminal work (sessions, input,
locators, waits, live capture, rendering, pixel comparison) belongs to
upstream `tui-test-rs`; application test code calls
`tui_test::Session` directly. `tuiscotti` only converts Ratatui views
into the shared snapshot model and runs the offline
compare-and-approve workflow on the result.

## Dependencies and imports

Add the library plus `ratatui` to your crate:

```toml
[dependencies]
tuiscotti = "0.4"
ratatui = "0.30"
```

The modules map one-to-one to the workflow stages:

```rust
use tuiscotti::approve::ApprovalStore; // explicit baseline approval
use tuiscotti::check::FrozenStore; // frozen sample-vs-baseline compare
use tuiscotti::render_profile::{PROFILE_NAME, platform_key, resolve_profile};
use tuiscotti::report::{render_report, write_report_index};
use tuiscotti::sample::{Observation, Sample}; // versioned sample files
use tuiscotti::view::{ViewSettings, capture_view}; // in-process capture
```

## Minimal working example

Render a view, bind it as a `state-and-pixels` sample, and run a
frozen check against an empty baseline root (the check reports
`missing`, it never writes a baseline):

```rust
use ratatui::widgets::Paragraph;
use tuiscotti::check::FrozenStore;
use tuiscotti::render_profile::{PROFILE_NAME, platform_key, resolve_profile};
use tuiscotti::sample::{Observation, Sample};
use tuiscotti::view::{ViewSettings, capture_view};

fn main() -> tuiscotti::Result<()> {
    let snapshot = capture_view(80, 24, &ViewSettings::builtin(), |frame| {
        frame.render_widget(Paragraph::new("hello"), frame.area());
    })?;
    let profile = resolve_profile(PROFILE_NAME)?;
    let sample = Sample::with_pixels(
        &format!("views/hello/pixels-{}", platform_key()),
        &snapshot,
        Observation::empty(),
        profile,
    )?;
    let dir = tempfile::tempdir()?;
    let store = FrozenStore::new(
        &dir.path().join("baselines"),
        &dir.path().join("artifacts"),
    )?;
    let outcome = store.check(&sample.to_bytes()?)?;
    assert!(!outcome.is_pass());
    Ok(())
}
```

(`tempfile` is a dev-dependency of this crate; add it to your own
`[dev-dependencies]` to run the snippet as a test.)

## Contracts

- **Two gates.** `state` compares canonical snapshot state only;
  `state-and-pixels` additionally compares the render contract and the
  decoded RGBA pixels. Pixels are only examined after state matches.
- **Test IDs** are ASCII paths (`views/hello/pixels-linux-x86_64`):
  non-empty, at most 240 bytes, no backslashes or drive prefixes, no
  empty segments, no `.`/`..`, no Windows device names. A
  `state-and-pixels` ID must end in `/pixels-<os>-<arch>`; a `state`
  ID must not end in a `pixels-` segment.
- **Digests.** The candidate digest is the SHA-256 of the sample file
  bytes. A baseline digest is the SHA-256 of the compared baseline
  bytes, or the literal `missing` when no baseline exists. Compared
  state excludes test IDs, file bytes, and observation notes.
- **Schemas.** Samples are `tuiscotti.sample.v1` JSON files; check
  evidence records are `tuiscotti.check.v1`; CLI results are
  `tuiscotti.cli.v1`. Unknown fields are rejected.
- **Limits (before allocation).** Sample files 64 MiB; one sample
  1,000,000 cells; one cell string 4,096 bytes; decoded RGBA 256 MiB;
  one image dimension 32,768 px; test ID 240 bytes (64 per segment).
- **Frozen checks never write baselines.** `FrozenStore::check` only
  writes candidate evidence under the artifact root on mismatch.
- **Approval is explicit and digest-arbitrated.**
  `ApprovalStore::accept` requires the candidate SHA-256 and the
  expected previous digest (`missing` for a first approval), re-renders
  image samples through the pinned upstream profile, and installs the
  baseline with one atomic rename. It refuses before any write when
  the `CI` variable is set, in any form; there is no bless flag.
- **One pinned render profile.** Only `tuiscotti-pinned-v1` resolves
  (16 px font, 10x20 cells, pinned fonts, upstream
  `RASTER_POLICY_PINNED_V1` raster policy).
- **Conceal is not redaction.** Hidden terminal text stays in the
  snapshot as a compared state field; it is never stripped.

## Reference links

- [API surface](../../docs/API.md)
- [Architecture and dependency pins](../../docs/ARCHITECTURE.md)
- [Snapshot, check, and approval semantics](../../docs/SNAPSHOTS.md)
- [CLI reference](../../docs/CLI.md)
- [Testing (functional suite)](../../docs/TESTING.md)
- [Limitations](../../docs/LIMITATIONS.md)
- [Migration history](../../docs/MIGRATION.md)
