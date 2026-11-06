use super::*;

/// An already `unfilter`-ed, previous row.
///
/// The data excludes the `RowFilter` byte - it only includes the actual pixel data.
pub(super) enum PrevRow {
    /// No unfiltered row.
    ///
    /// `None` is the value of `UnfilteringBuffer::prev_row` before any row has
    /// been unfiltered (or at the start of a new interlace pass).
    None,

    /// Offset of `UnfilteringBuffer::data_stream` where the unfiltered row
    /// starts.
    ///
    /// `UnfilteringBuffer::InPlace(_)` is used by `unfilter_curr_row_in_place`
    /// when setting `UnfilteringBuffer::prev_row`.
    InPlace(usize),

    /// Separate scratch buffer containing the unfiltered row data.
    ///
    /// `UnfilteringBuffer::Scratch(_)` is used by
    /// `unfilter_curr_row_using_scratch_buffer`
    /// when setting `UnfilteringBuffer::prev_row`.
    Scratch(Vec<u8>),
}

impl PrevRow {
    /// Returns the previous unfiltered row as a slice of bytes.
    ///
    /// `buf` should refer to the `..current_start` portion of
    /// `UnfilteringBuffer::data_stream`.
    pub(super) fn as_slice<'a>(&'a self, buf: &'a [u8]) -> &'a [u8] {
        match self {
            PrevRow::None => &[],
            PrevRow::InPlace(prev_start) => &buf[*prev_start..],
            PrevRow::Scratch(scratch) => scratch.as_slice(),
        }
    }
}

pub(super) fn checked_next_multiple_of(val: usize, factor: usize) -> Option<usize> {
    if factor == 0 {
        return None;
    }

    let remainder = val % factor;

    if remainder > 0 {
        val.checked_add(factor - remainder)
    } else {
        Some(val)
    }
}

#[test]
fn next_multiple_of_backport_testsuite() {
    assert_eq!(checked_next_multiple_of(1, 0), None);
    assert_eq!(checked_next_multiple_of(2, 0), None);
    assert_eq!(checked_next_multiple_of(1, 2), Some(2));
    assert_eq!(checked_next_multiple_of(2, 2), Some(2));
    assert_eq!(checked_next_multiple_of(2, 5), Some(5));
    assert_eq!(checked_next_multiple_of(1, usize::MAX), Some(usize::MAX));
    assert_eq!(checked_next_multiple_of(usize::MAX, 2), None);
}
