use super::*;

/// The type and strength of applied compression.
///
/// This is a simple, high-level interface that will automatically choose
/// the appropriate DEFLATE compression mode and PNG filter.
///
/// If you need more control over the encoding parameters,
/// you can set the [`DeflateCompression`] and [`Filter`] manually.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub(crate) enum Compression {
    /// No compression whatsoever. Fastest, but results in large files.
    NoCompression,
    /// Extremely fast but light compression.
    ///
    /// Note: When used in streaming mode, this compression level can actually result in files
    /// *larger* than would be produced by `NoCompression` on incompressible data because
    /// it doesn't do any buffering of the output stream to detect whether the data is being compressed or not.
    Fastest,
    /// Extremely fast compression with a decent compression ratio.
    ///
    /// Significantly outperforms libpng and other popular encoders by using a [specialized DEFLATE
    /// implementation tuned for PNG](https://crates.io/crates/fdeflate), while still providing
    /// better compression ratio than the fastest modes of other encoders.
    ///
    /// Like [`Compression::Fastest`] this can currently produce files larger than `NoCompression` in
    /// streaming mode when given incompressible data. This may change in the future.
    Fast,
    /// Balances encoding speed and compression ratio
    Balanced,
    /// Spend much more time to produce a slightly smaller file than with `Balanced`.
    High,
}

impl Default for Compression {
    fn default() -> Self {
        Self::Balanced
    }
}

/// Advanced compression settings with more customization options than [`Compression`].
///
/// Note that this setting only affects DEFLATE compression.
/// Another setting that influences the compression ratio and lets you choose
/// between encoding speed and compression ratio is the [`Filter`].
///
/// ### Stability guarantees
///
/// The implementation details of DEFLATE compression may evolve over time,
/// even without a semver-breaking change to the version of `png` crate.
///
/// If a certain compression setting is superseded by other options,
/// it may be marked deprecated and remapped to a different option.
/// You will see a deprecation notice when compiling code relying on such options.
#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub(crate) enum DeflateCompression {
    /// Do not compress the data at all.
    ///
    /// Useful for incompressible images, or when speed is paramount and you don't care about size
    /// at all.
    ///
    /// This mode also disables filters, forcing [`Filter::NoFilter`].
    NoCompression,

    /// Excellent for creating lightly compressed PNG images very quickly.
    ///
    /// Uses the [fdeflate](https://crates.io/crates/fdeflate) crate under the hood to achieve
    /// speeds far exceeding what libpng is capable of while still providing a decent compression
    /// ratio.
    ///
    /// Note: When used in streaming mode, this compression level can actually result in files
    /// *larger* than would be produced by `NoCompression` because it doesn't do any buffering of
    /// the output stream to detect whether the data is being compressed or not.
    FdeflateUltraFast,

    /// Compression level between 1 and 9, where higher values mean better compression at the cost of
    /// speed.
    ///
    /// This is currently implemented via [flate2](https://crates.io/crates/flate2) crate
    /// by passing through the [compression level](flate2::Compression::new).
    ///
    /// The implementation details and the exact meaning of each level may change in the future,
    /// including in semver-compatible releases.
    Level(u8),
    // Other variants can be added in the future
}

impl Default for DeflateCompression {
    fn default() -> Self {
        Self::from_simple(Compression::Balanced)
    }
}

impl DeflateCompression {
    pub(crate) fn from_simple(value: Compression) -> Self {
        match value {
            Compression::NoCompression => Self::NoCompression,
            Compression::Fastest => Self::FdeflateUltraFast,
            Compression::Fast => Self::FdeflateUltraFast,
            Compression::Balanced => Self::Level(flate2::Compression::default().level() as u8),
            Compression::High => Self::Level(flate2::Compression::best().level() as u8),
        }
    }
}
