# Real historical snapshot pixel render candidate

This page records a deterministic rendering comparison of a real Tuiscotti logical update. The rendered snapshots are historical captures from source revisions `ea8cf188583be2a390ae65fd9f74d3aeb30492aa` and `bdc44737842c4f2b6a4ba7b4814b8fff9f734de2`. The later revision changed 1,902 of 4,800 cells in `functional/showcase_page_pickers`; it did not change the snapshot's 256-entry palette table.

The original captures, old JSON, paired protocol buffers, and successful state comparator evidence are in the [selected-history evidence archive](../../benchmark-evidence/tuiscotti-selected-history-evidence-20261109.tar.gz) (SHA-256 `a5b0e432f55d39fc2d74a7be6826826bcfa8d7ae23f2752516b10e825f1a6305`). The [render supplement](../../benchmark-evidence/tuiscotti-reconstructed-pixel-render-20261109.tar.gz) (SHA-256 `beba0945840b2c66fe5d6f502becb020c88284659db3dbc98ea9f6cd2c1ebd7a`) contains only the renderer helper, rendered PNGs and repeated outputs, result records, and pinned tool/wrapper evidence. It references inputs by archive-relative path and SHA-256, so it does not duplicate the legacy capture files.

Both snapshots were rendered with the same pinned profile (`tuiscotti-pinned-v1`, renderer `0.2.1`) at 1200 × 800. The pair differs in 158,537 of 960,000 pixels (16.514%); the summed absolute RGBA channel delta is 27,148,953. The `ea8` and `bdc` PNG hashes are `e7410ddd35f68ee91f1f87dce641ec68d828787fccba6a99de4d820e906ebdd4` and `f17563ddf68549b1d441b8a37a91cd659b22cfc7be9c088938013dc5a153b0ec`. The repeated run reproduced the PNG bytes and comparison results exactly. The package records the RGBA hashes, exact input hashes, commands, and run output.

## Disposable checkout replay

Run from the repository root on macOS arm64 with Git, mise, and network access. The replay creates a fresh temporary directory, extracts and verifies both archives, fetches the pinned renderer source, installs the archived helper into that disposable checkout, and writes all render outputs under the temporary directory. It removes the mise trust entry on exit.

```sh
set -eu
work="$(mktemp -d "${TMPDIR:-/tmp}/tuiscotti-pixel-replay.XXXXXX")"
base_archive=benchmark-evidence/tuiscotti-selected-history-evidence-20261109.tar.gz
render_archive=benchmark-evidence/tuiscotti-reconstructed-pixel-render-20261109.tar.gz
printf '%s  %s\n' a5b0e432f55d39fc2d74a7be6826826bcfa8d7ae23f2752516b10e825f1a6305 "$base_archive" | shasum -a 256 -c
printf '%s  %s\n' beba0945840b2c66fe5d6f502becb020c88284659db3dbc98ea9f6cd2c1ebd7a "$render_archive" | shasum -a 256 -c
mkdir -p "$work/base" "$work/render"
tar -xzf "$base_archive" -C "$work/base"
tar -xzf "$render_archive" -C "$work/render"
(cd "$work/base/history-evidence" && shasum -a 256 -c SHA256SUMS)
(cd "$work/render/tuiscotti-reconstructed-pixel-render-20261109" && shasum -a 256 -c SHA256SUMS)
git clone --filter=blob:none https://github.com/tailrocks/tuiscotti.git "$work/source"
git -C "$work/source" checkout --quiet --detach 6d01628170de8bed3f00f52713f3a510b782e41e
mkdir -p "$work/source/crates/tuiscotti/examples" "$work/output"
install -m 0644 "$work/render/tuiscotti-reconstructed-pixel-render-20261109/render-code/real_pixel_render_pair.rs" "$work/source/crates/tuiscotti/examples/real_pixel_render_pair.rs"
left="$work/base/history-evidence/captures/ea8"
right="$work/base/history-evidence/captures/bdc"
left_snapshot="$left/snapshots/process-91390-000000000000000018e64b6983b50810-0000/functional/showcase_page_pickers.capture-0000000000000000.snapshot.json"
left_json="$left/old-json/process-91390-000000000000000018e64b6983b50810-0000/functional/showcase_page_pickers.capture-0000000000000000.tui.json"
right_snapshot="$right/snapshots/process-2123-000000000000000018e64b794463c2e0-0000/functional/showcase_page_pickers.capture-0000000000000000.snapshot.json"
right_json="$right/old-json/process-2123-000000000000000018e64b794463c2e0-0000/functional/showcase_page_pickers.capture-0000000000000000.tui.json"
mise trust "$work/source/mise.toml"
trap 'mise trust --untrust "$work/source/mise.toml" >/dev/null 2>&1 || true' EXIT
(cd "$work/source" && CARGO_TARGET_DIR="$work/target" mise exec -- mbx +1.99.1 run --release --locked --example real_pixel_render_pair -p tuiscotti -- \
  ea8cf188583be2a390ae65fd9f74d3aeb30492aa "$left_snapshot" "$left_json" "$work/output/ea8.png" \
  bdc44737842c4f2b6a4ba7b4814b8fff9f734de2 "$right_snapshot" "$right_json" "$work/output/bdc.png" \
  > "$work/output/result.json")
printf 'Replay output: %s\n' "$work/output"
```

Compare `diff` and, for each side, `width`, `height`, `png_sha256`, `rgba_sha256`, and `render_contract_debug` in the generated `result.json` with the supplement record. Compare the input snapshot and old-JSON hashes as well. Ignore `snapshot_path`, `old_json_path`, and `png_path`: those absolute paths are expected to differ in a new temporary checkout. The command rebuilds the renderer example in release mode; the supplement retains the actual isolated tool and wrapper logs for the reported run.

## Evidence boundary

This is a reconstructed render of historical captured snapshots, not a historical stored pixel-gate capture. Both input source JSON records say the gate was `state`; no pixel-gated sample was generated or accepted, and no baseline was written. The evidence demonstrates a real pixel difference when the two historical snapshots are rendered under one pinned profile. It does not demonstrate a palette-table update, and it does not close the historical pixel-gate coverage gap.
