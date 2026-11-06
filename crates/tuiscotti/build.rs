//! Enforce the pinned Zstandard decoder context bound.

use std::{env, io, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rustc-check-cfg=cfg(fuzzing)");
    println!("cargo:rerun-if-env-changed=ZSTD_SYS_USE_PKG_CONFIG");
    println!("cargo:rerun-if-changed=zstd_context_bound.c");
    assert!(
        env::var_os("ZSTD_SYS_USE_PKG_CONFIG").is_none(),
        "ZSTD_SYS_USE_PKG_CONFIG is unsupported, even when empty"
    );
    let include = env::var_os("DEP_ZSTD_INCLUDE")
        .map(PathBuf::from)
        .ok_or_else(|| {
            io::Error::other(
                "zstd-sys did not provide DEP_ZSTD_INCLUDE; non-cargo and links overrides are unsupported",
            )
        })?;
    assert!(
        include
            .join("decompress/zstd_decompress_internal.h")
            .is_file(),
        "zstd-sys source headers are missing from DEP_ZSTD_INCLUDE: {}",
        include.display()
    );
    let compiler = cc::Build::new().get_compiler();
    let mut command = compiler.to_command();
    command.arg("-I").arg(&include);
    if compiler.is_like_msvc() {
        command.arg("/Zs");
    } else {
        command.arg("-fsyntax-only");
    }
    let status = command.arg("zstd_context_bound.c").status()?;
    if !status.success() {
        return Err(io::Error::other("zstd target layout assertion failed").into());
    }
    Ok(())
}
