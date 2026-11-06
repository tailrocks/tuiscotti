use core::convert::TryInto;

use crate::sample::embedded_png::png::{Compression, common::BytesPerPixel};

mod filter;
mod paeth;
mod unfilter;
pub(crate) use filter::filter;
#[cfg(test)]
pub(crate) use filter::sum_buffer;
pub(crate) use unfilter::unfilter;

#[cfg(any())]
mod simd;

/// The byte level filter applied to scanlines to prepare them for compression.
///
/// Compression in general benefits from repetitive data. The filter is a content-aware method of
/// compressing the range of occurring byte values to help the compression algorithm. Note that
/// this does not operate on pixels but on raw bytes of a scanline.
///
/// Details on how each filter works can be found in the [PNG Book](http://www.libpng.org/pub(crate)/png/book/chapter09.html).
///
/// The default filter is `Adaptive`, which uses heuristics to select the best filter for every row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub(crate) enum Filter {
    NoFilter,
    Sub,
    Up,
    Avg,
    Paeth,
    Adaptive,
    MinEntropy,
}

impl Default for Filter {
    fn default() -> Self {
        Filter::Adaptive
    }
}

impl From<RowFilter> for Filter {
    fn from(value: RowFilter) -> Self {
        match value {
            RowFilter::NoFilter => Filter::NoFilter,
            RowFilter::Sub => Filter::Sub,
            RowFilter::Up => Filter::Up,
            RowFilter::Avg => Filter::Avg,
            RowFilter::Paeth => Filter::Paeth,
        }
    }
}

impl Filter {
    pub(crate) fn from_simple(compression: Compression) -> Self {
        match compression {
            Compression::NoCompression => Filter::NoFilter, // with no DEFLATE filtering would only waste time
            Compression::Fastest => Filter::Up, // pairs well with FdeflateUltraFast, producing much smaller files while being very fast
            Compression::Fast => Filter::Adaptive,
            Compression::Balanced => Filter::Adaptive,
            Compression::High => Filter::Adaptive,
        }
    }
}

/// Unlike the public [Filter], does not include the "Adaptive" option
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum RowFilter {
    NoFilter = 0,
    Sub = 1,
    Up = 2,
    Avg = 3,
    Paeth = 4,
}

impl Default for RowFilter {
    fn default() -> Self {
        RowFilter::Up
    }
}

impl RowFilter {
    pub(crate) fn from_u8(n: u8) -> Option<Self> {
        match n {
            0 => Some(Self::NoFilter),
            1 => Some(Self::Sub),
            2 => Some(Self::Up),
            3 => Some(Self::Avg),
            4 => Some(Self::Paeth),
            _ => None,
        }
    }

    pub(crate) fn from_method(strat: Filter) -> Option<Self> {
        match strat {
            Filter::NoFilter => Some(Self::NoFilter),
            Filter::Sub => Some(Self::Sub),
            Filter::Up => Some(Self::Up),
            Filter::Avg => Some(Self::Avg),
            Filter::Paeth => Some(Self::Paeth),
            Filter::Adaptive | Filter::MinEntropy => None,
        }
    }
}

mod test {
    use super::*;
    use core::iter;

    #[test]
    fn roundtrip() {
        // A multiple of 8, 6, 4, 3, 2, 1
        const LEN: u8 = 240;
        let previous: Vec<_> = iter::repeat(1).take(LEN.into()).collect();
        let current: Vec<_> = (0..LEN).collect();
        let expected = current.clone();

        let roundtrip = |kind: RowFilter, bpp: BytesPerPixel| {
            let mut output = vec![0; LEN.into()];
            filter(kind.into(), bpp, &previous, &current, &mut output);
            unfilter(kind, bpp, &previous, &mut output);
            assert_eq!(
                output, expected,
                "Filtering {:?} with {:?} does not roundtrip",
                bpp, kind
            );
        };

        let filters = [
            RowFilter::NoFilter,
            RowFilter::Sub,
            RowFilter::Up,
            RowFilter::Avg,
            RowFilter::Paeth,
        ];

        let bpps = [
            BytesPerPixel::One,
            BytesPerPixel::Two,
            BytesPerPixel::Three,
            BytesPerPixel::Four,
            BytesPerPixel::Six,
            BytesPerPixel::Eight,
        ];

        for &filter in filters.iter() {
            for &bpp in bpps.iter() {
                roundtrip(filter, bpp);
            }
        }
    }

    #[test]
    fn roundtrip_ascending_previous_line() {
        // A multiple of 8, 6, 4, 3, 2, 1
        const LEN: u8 = 240;
        let previous: Vec<_> = (0..LEN).collect();
        let current: Vec<_> = (0..LEN).collect();
        let expected = current.clone();

        let roundtrip = |kind: RowFilter, bpp: BytesPerPixel| {
            let mut output = vec![0; LEN.into()];
            filter(kind.into(), bpp, &previous, &current, &mut output);
            unfilter(kind, bpp, &previous, &mut output);
            assert_eq!(
                output, expected,
                "Filtering {:?} with {:?} does not roundtrip",
                bpp, kind
            );
        };

        let filters = [
            RowFilter::NoFilter,
            RowFilter::Sub,
            RowFilter::Up,
            RowFilter::Avg,
            RowFilter::Paeth,
        ];

        let bpps = [
            BytesPerPixel::One,
            BytesPerPixel::Two,
            BytesPerPixel::Three,
            BytesPerPixel::Four,
            BytesPerPixel::Six,
            BytesPerPixel::Eight,
        ];

        for &filter in filters.iter() {
            for &bpp in bpps.iter() {
                roundtrip(filter, bpp);
            }
        }
    }

    #[test]
    // This tests that converting u8 to i8 doesn't overflow when taking the
    // absolute value for adaptive filtering: -128_i8.abs() will panic in debug
    // or produce garbage in release mode. The sum of 0..=255u8 should equal the
    // sum of the absolute values of -128_i8..=127, or abs(-128..=0) + 1..=127.
    fn sum_buffer_test() {
        let sum = (0..=128).sum::<u64>() + (1..=127).sum::<u64>();
        let buf: Vec<u8> = (0_u8..=255).collect();

        assert_eq!(
            sum,
            crate::sample::embedded_png::png::filter::sum_buffer(&buf)
        );
    }
}
