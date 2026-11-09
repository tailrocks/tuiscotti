# Same-capture selected history

This experiment compares Git pack sizes for JSON and protobuf samples generated from the same upstream snapshots. Five captured state samples passed their revision's normal frozen check and the paired full-field comparison, including metadata. The five captured pairs cover an addition and update of a focused-control sample, an added browser-dialog sample, and a picker sample before and after an update. The reconstructed Git sequence also removes the browser-dialog path at a source-verified deletion; there is no sixth capture for that deletion.

The histories contain **three selected logical paths**, with matching stages and format-specific file suffixes. One repack produced a 16,870-byte JSON pack and a 15,134-byte protobuf pack, each with 22 objects. The result is a single selected-path measurement; it is not a timing comparison.

These are reconstructed histories built from fresh captures, not the full original commits or their original captured values. The picker seed uses a capture from the later parent revision: its committed baseline blob was unchanged from the initial revision, but that does not establish identical capture metadata. This experiment does not measure pixel or palette updates. Earlier shared-target captures were invalid and are excluded.

The [evidence archive](../../benchmark-evidence/tuiscotti-selected-history-evidence-20261109.tar.gz) contains the isolated capture pairs, comparison proofs and metadata negative controls, test logs, selected producer sources and locks, baseline records, helper provenance, measurement source and result, and both pack files. Executables and full source trees are omitted; executable hashes and source commit references are included. The effective environment and immutable build dependency closure were not fully recorded, so the archive does not establish a byte-identical rebuild. Its manifests preserve original absolute scratch paths; identify archived files by stage and hash rather than treating those paths as portable.

The archive SHA-256 is `a5b0e432f55d39fc2d74a7be6826826bcfa8d7ae23f2752516b10e825f1a6305`. From the repository root, verify and extract it into a new temporary directory:

```sh
archive=benchmark-evidence/tuiscotti-selected-history-evidence-20261109.tar.gz
printf '%s  %s\n' a5b0e432f55d39fc2d74a7be6826826bcfa8d7ae23f2752516b10e825f1a6305 "$archive" | shasum -a 256 -c
history_evidence_dir=$(mktemp -d)
tar -xzf "$archive" -C "$history_evidence_dir"
(cd "$history_evidence_dir/history-evidence" && shasum -a 256 -c SHA256SUMS)
```

The extracted `README.md` explains the evidence layout and limitations. The extracted `measurement/RUN_SUCCESS` binds `measurement/result.json`, which records the Git settings, selected stages, input hashes, and pack hashes. See also the earlier [mainline history experiment](real-tcb-mainline-history.md) and [paired field comparison](paired-field-comparison.md).
