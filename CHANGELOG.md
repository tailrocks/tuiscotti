# Changelog

All notable changes to the `tuiscotti` and `tuiscotti-cli` crates are
documented here. Versions follow Semantic Versioning.

## 0.4.0 (pending publication)

> On publication the crates will be at [crates.io](https://crates.io/crates/tuiscotti)
> and [docs.rs](https://docs.rs/tuiscotti). Install with
> `cargo add tuiscotti@0.4`, or grab the binary with
> `cargo install tuiscotti-cli@0.4.0` (see
> [Install from the registry](#install-from-the-registry)).

First public release. Two crates, one offline binary:

- `tuiscotti`: pure-view capture (`view`), versioned samples
  (`sample`), frozen checks (`check`), explicit digest-arbitrated
  approval (`approve`), offline rendering (`render`), static reports
  (`report`), one pinned render profile (`render_profile`), one
  typed error (`error`). Never spawns a process, never opens a
  terminal device, never touches the network.
- `tuiscotti-cli`: offline `tuiscotti` binary with exactly four
  commands — `check`, `accept`, `render`, `report`. Approval refuses
  before any write whenever the `CI` variable is set, in any form.

### Registry blocker (resolved: `tuiscotti-tui-test 0.2.1` adopted)

Both crates build against snapshot/render/pixel APIs that upstream
`microsoft/tui-test` (`tui-test-rs 0.1.1`) does not carry. The
maintained fork is now published as the renamed registry crate
`tuiscotti-tui-test 0.2.1` (lib name still `tui_test`), and the
workspace `[patch.crates-io]` git pin is removed: both crates depend
on `tuiscotti-tui-test 0.2.1` from crates.io like any other registry
dependency, so downstream consumers resolve the same release with no
extra pin. Remaining follow-up (R02) is switching the dependency name
back to upstream `tui-test-rs` once its registry release carries the
required APIs; `use tui_test::...` imports are unaffected either way.

### Install from the registry

Depend on the published crate:

```toml
[dependencies]
tuiscotti = "0.4"
```

or add it on the command line:

```sh
cargo add tuiscotti@0.4
```

Install the CLI binary from the registry:

```sh
cargo install tuiscotti-cli@0.4.0 --locked
```
