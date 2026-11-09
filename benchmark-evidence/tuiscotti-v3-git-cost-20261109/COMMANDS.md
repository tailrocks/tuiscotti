# Exact command outline; all paths are disposable /private/tmp repositories.

Git: git version 2.54.0 (Apple Git-157).

Both bare repos were initialized independently and received identical local settings: core.compression=6, pack.compression=6, pack.window=0, pack.depth=0, repack.writeBitmaps=false. The empty database was measured with `git count-objects -v` before object creation.

Packed database: `git hash-object -w -t blob <v3-archive>`; `git repack -a -d -f --window=0 --depth=0 --keep-unreachable`; `git count-objects -v`; `git verify-pack -v objects/pack/*.idx`. `--keep-unreachable` was needed because no commit/tree/ref was added: the object database contains only the one archive blob.

The archive file SHA-256 and the SHA-256 of `git cat-file blob <oid>` were both f1c1f947cf60228cba5592f92ac67aec626cfebad823829089f48962805d05b6. See the raw count/config/verify-pack files in this directory and `GIT-COST-SUMMARY.json`.
