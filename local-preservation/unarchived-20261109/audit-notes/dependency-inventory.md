# Direct dependency and tool inventory

Observed 2026-11-05. Latest stable, non-yanked package versions are from the official crates.io sparse index. Tool versions are from the maintainers' GitHub release APIs. Recheck at final revision.

| Package | Old lock | Latest stable | Source |
| --- | --- | --- | --- |
| base64 | 0.23.1 | 0.23.1 | https://index.crates.io/ba/se/base64 |
| clap | 4.6.7 | 4.6.7 | https://index.crates.io/cl/ap/clap |
| fs2 | 0.4.3 | 0.4.3 | https://index.crates.io/3/f/fs2 |
| image | 0.25.10 | 0.25.10 | https://index.crates.io/im/ag/image |
| serde | 1.0.229 | 1.0.229 | https://index.crates.io/se/rd/serde |
| serde_json | 1.0.154 | 1.0.154 | https://index.crates.io/se/rd/serde_json |
| sha2 | 0.11.1 | 0.11.1 | https://index.crates.io/sh/a2/sha2 |
| ratatui | 0.30.2 | 0.30.2 | https://index.crates.io/ra/ta/ratatui |
| tempfile | 3.27.0 | 3.27.0 | https://index.crates.io/te/mp/tempfile |
| toml | 0.9.12+spec-1.1.0 | 1.2.0+spec-1.1.0 | https://index.crates.io/to/ml/toml |
| tuiscotti-tui-test | 0.2.1 | 0.2.1 | https://index.crates.io/tu/is/tuiscotti-tui-test |

| Tool | Old pin | Latest stable | Source |
| --- | --- | --- | --- |
| Rust | 1.99.1 | 1.99.1 | https://github.com/rust-lang/rust/releases/tag/1.99.1 |
| mr-boxington | 1.21.1 | 1.29.2 | https://github.com/jdx/mr-boxington/releases/tag/v1.29.2 |
| cargo-nextest | 0.9.148 | 0.9.151 | https://github.com/nextest-rs/nextest/releases/tag/cargo-nextest-0.9.151 |
| alint | 0.15.2 | 0.21.0 | https://github.com/asamarts/alint/releases/tag/v0.21.0 |
| cargo-deny | 0.20.2 | 0.20.2 | https://github.com/EmbarkStudios/cargo-deny/releases/tag/0.20.2 |
| Velnor Actions | 0.1.58 | 0.1.73 | https://github.com/tailrocks/velnor-new/releases/tag/v0.1.73 |

The Rust 1.99.1 declared floor remains current. TOML 0.9 remains a transitive dependency of tuiscotti-tui-test; the workspace direct dependency now uses 1.2. The updated alint reports six file-length violations in existing source and test files. Velnor upgrade and regeneration are pending.
