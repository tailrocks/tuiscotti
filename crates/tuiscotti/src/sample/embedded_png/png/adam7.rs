//! Utility functions related to handling of
//! [the Adam7 algorithm](https://en.wikipedia.org/wiki/Adam7_algorithm).
use core::ops::RangeTo;

#[cfg(doc)]
use crate::sample::embedded_png::png::decoder::Reader;

/// Describes which stage of
/// [the Adam7 algorithm](https://en.wikipedia.org/wiki/Adam7_algorithm)
/// applies to a decoded row.
///
/// See also [`Reader::next_interlaced_row`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Adam7Info {
    /// The Adam7 pass number, 1..7.
    pub(crate) pass: u8,
    /// The index of the line within this pass.
    pub(crate) line: u32,
    /// The original pixel count.
    pub(crate) width: u32,
    /// How many Adam7 samples there are.
    pub(crate) samples: u32,
}

/// The index of a bit in the image buffer.
///
/// We do not use a pure `usize` to avoid overflows on 32-bit targets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BitPostion {
    byte: usize,
    /// [0..8)
    bit: u8,
}

impl Adam7Info {
    /// Creates a new `Adam7Info`.  May panic if the arguments are out of range (e.g. if `pass` is
    /// 0 or greater than 8).
    ///
    /// * `pass` corresponds to a pass of the
    ///   [the Adam7 algorithm](https://en.wikipedia.org/wiki/Adam7_algorithm)
    /// * `line` is the number of a line within a pass (starting with 0).  For example,
    ///   in an image of height 8, `line` can be beteween `0..4` in the 7th `pass`
    ///   (those 4 interlaced rows correspond to 2nd, 4th, 6th, and 8th row of the full image).
    /// * `width` describes how many pixels are in a full row of the image. The bytes in each
    ///   passline of the Adam7 are calculated from this number.
    ///
    /// Note that in typical usage, `Adam7Info`s are returned by [`Reader::next_interlaced_row`]
    /// and there is no need to create them by calling `Adam7Info::new`.  `Adam7Info::new` is
    /// nevertheless exposed as a public API, because it helps to provide self-contained example
    /// usage of [`expand_interlaced_row`](crate::sample::embedded_png::png::expand_interlaced_row).
    pub(crate) fn new(pass: u8, line: u32, width: u32) -> Self {
        assert!((1..=7).contains(&pass));
        assert!(width > 0);

        let info = PassConstants::PASSES[pass as usize - 1];
        let samples = info.count_samples(width);

        Self {
            pass,
            line,
            width,
            samples,
        }
    }

    fn pass_constants(&self) -> PassConstants {
        PassConstants::PASSES[self.pass as usize - 1]
    }

    /// How often to repeat a pixel.
    fn splat_pixel_repeat(self, idx: usize) -> u8 {
        let pass = self.pass_constants();
        let x_pixel = idx as u32 * u32::from(pass.x_sampling) + u32::from(pass.x_offset);
        (self.width - x_pixel).min(pass.splat_x_repeat().into()) as u8
    }

    fn splat_line_repeat(self, height: u32) -> u8 {
        let pass = self.pass_constants();
        let y_line = self.line * u32::from(pass.y_sampling) + u32::from(pass.y_offset);
        (height - y_line).min(pass.splat_y_repeat().into()) as u8
    }
}

#[derive(Clone, Copy)]
pub(crate) struct PassConstants {
    x_sampling: u8,
    x_offset: u8,
    y_sampling: u8,
    y_offset: u8,
}

impl PassConstants {
    const fn splat_x_repeat(self) -> u8 {
        self.x_sampling - self.x_offset
    }

    const fn splat_y_repeat(self) -> u8 {
        self.y_sampling - self.y_offset
    }

    pub(crate) fn count_samples(self, width: u32) -> u32 {
        width
            .saturating_sub(u32::from(self.x_offset))
            .div_ceil(u32::from(self.x_sampling))
    }

    pub(crate) fn count_lines(self, height: u32) -> u32 {
        height
            .saturating_sub(u32::from(self.y_offset))
            .div_ceil(u32::from(self.y_sampling))
    }

    /// The constants associated with each of the 7 passes. Note that it is 0-indexed while the
    /// pass number (as per specification) is 1-indexed.
    pub(crate) const PASSES: [Self; 7] = {
        // Shortens the constructor for readability, retains clear argument order below.
        const fn new(x_sampling: u8, x_offset: u8, y_sampling: u8, y_offset: u8) -> PassConstants {
            PassConstants {
                x_sampling,
                x_offset,
                y_sampling,
                y_offset,
            }
        }

        [
            new(8, 0, 8, 0),
            new(8, 4, 8, 0),
            new(4, 0, 8, 4),
            new(4, 2, 4, 0),
            new(2, 0, 4, 2),
            new(2, 1, 2, 0),
            new(1, 0, 2, 1),
        ]
    };
}

/// This iterator iterates over the different passes of an image Adam7 encoded
/// PNG image
/// The pattern is:
///     16462646
///     77777777
///     56565656
///     77777777
///     36463646
///     77777777
///     56565656
///     77777777
///
#[derive(Clone)]
pub(crate) struct Adam7Iterator {
    line: u32,
    lines: u32,
    line_width: u32,
    current_pass: u8,
    width: u32,
    height: u32,
}

impl Adam7Iterator {
    pub(crate) fn new(width: u32, height: u32) -> Adam7Iterator {
        let mut this = Adam7Iterator {
            line: 0,
            lines: 0,
            line_width: 0,
            current_pass: 1,
            width,
            height,
        };
        this.init_pass();
        this
    }

    /// Calculates the bounds of the current pass
    fn init_pass(&mut self) {
        let info = PassConstants::PASSES[self.current_pass as usize - 1];
        self.line_width = info.count_samples(self.width);
        self.lines = info.count_lines(self.height);
        self.line = 0;
    }
}

/// Iterates over `Adam7Info`s.
impl Iterator for Adam7Iterator {
    type Item = Adam7Info;
    fn next(&mut self) -> Option<Self::Item> {
        if self.line < self.lines && self.line_width > 0 {
            let this_line = self.line;
            self.line += 1;
            Some(Adam7Info {
                pass: self.current_pass,
                line: this_line,
                width: self.width,
                samples: self.line_width,
            })
        } else if self.current_pass < 7 {
            self.current_pass += 1;
            self.init_pass();
            self.next()
        } else {
            None
        }
    }
}

/// The algorithm to use when progressively filling pixel data from Adam7 interlaced passes.
///
/// Adam7 interlacing is a technique optionally used in PNG by which only a sub-sample of pixel
/// data is encoded in the beginning of the image data chunks, followed by progressively larger
/// subsets of the data in subsequent passes. Therefore a 'rough image' is available after ust a
/// very tiny fraction of the data has been read which can be advantageous for loading an image
/// from a slow IO medium while optimizing time-to-first-meaningful-paint and then replacing the
/// presented data as it is streamed in.
///
/// There are trade-offs to make here. The strictly necessary requirement for an implementation is
/// that the exact image is recovered after all passes are applied. However the intermediate states
/// of the output are left to the implementation, as long as it follows the restriction of
/// resulting in the intended image when all passes have been applied.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub(crate) enum Adam7Variant {
    /// This is the adam7 de-interlace we do by default. Only pixels related to the pass are
    /// written. The output buffer should not be directly used for presentation until all passes
    /// are complete. At least the invalid pixels in the buffer should be masked. However, this
    /// performs the least amount of writes and is optimal when you're only reading full frames.
    ///
    /// This corresponds to [`expand_interlaced_row`].
    ///
    /// [`expand_interlaced_row`]: crate::sample::embedded_png::png::expand_interlaced_row.
    #[default]
    Sparse,
    /// A variant of the Adam7 de-interlace that ensures that all pixels are initialized after each
    /// pass, and are progressively refined towards the final image. Performs more writes than the
    /// other variant as some pixels are touched repeatedly, but ensures the buffer can be used as
    /// directly as possible for presentation.
    ///
    /// This corresponds to [`splat_interlaced_row`].
    ///
    /// [`splat_interlaced_row`]: crate::sample::embedded_png::png::splat_interlaced_row
    Splat,
}

mod expand;
pub(crate) use expand::expand_pass;
#[cfg(test)]
pub(crate) use expand::expand_pass_splat;

#[cfg(test)]
#[path = "adam7_tests.rs"]
mod tests;
