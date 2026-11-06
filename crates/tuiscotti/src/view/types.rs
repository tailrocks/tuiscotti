use std::fmt::{Display, Formatter};

use tui_test::profile::Rgb;
use tui_test::{SnapshotCursorShape, SnapshotError};

#[doc = "Result of a pure-view capture or conversion."]
pub type ViewResult<T> = Result<T, ViewError>;

#[derive(Debug)]
#[non_exhaustive]
#[doc = "Every failure a pure-view capture or conversion can report, with sources retained."]
pub enum ViewError {
    #[doc = "The requested grid has a zero column or row count."]
    EmptyDimensions,
    #[doc = "A buffer area coordinate plus its offset does not fit a 16-bit position."]
    AreaOverflow,
    #[doc = "A grid position inside the buffer area has no cell."]
    CellOutOfBounds {
        #[doc = "Buffer-local column of the missing cell."]
        x: u32,
        #[doc = "Buffer-local row of the missing cell."]
        y: u32,
    },
    #[doc = "A column covered by a wide leading cell carries its own grapheme instead of continuing that cell."]
    CoveredCellOccupied {
        #[doc = "Buffer-local column of the occupying cell."]
        x: u32,
        #[doc = "Buffer-local row of the occupying cell."]
        y: u32,
    },
    #[doc = "A cell outside any wide-cell coverage has zero width, so it cannot be a continuation."]
    UncoveredZeroWidthCell {
        #[doc = "Buffer-local column of the zero-width cell."]
        x: u32,
        #[doc = "Buffer-local row of the zero-width cell."]
        y: u32,
    },
    #[doc = "A wide leading cell is wider than the columns left in its row, so its continuation would fall outside the grid."]
    LeadingCellOverhang {
        #[doc = "Buffer-local column of the overhanging leading cell."]
        x: u32,
        #[doc = "Buffer-local row of the overhanging leading cell."]
        y: u32,
        #[doc = "Terminal columns the leading cell claims."]
        width: u16,
    },
    #[doc = "The upstream validated constructor rejected the converted snapshot."]
    Snapshot(SnapshotError),
}

impl Display for ViewError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyDimensions => f.write_str("view grid dimensions must be nonzero"),
            Self::AreaOverflow => {
                f.write_str("buffer area coordinate does not fit a 16-bit position")
            }
            Self::CellOutOfBounds { x, y } => {
                write!(f, "buffer has no cell at ({x}, {y})")
            }
            Self::CoveredCellOccupied { x, y } => {
                write!(
                    f,
                    "covered cell at ({x}, {y}) carries its own grapheme instead of continuing the wide cell"
                )
            }
            Self::UncoveredZeroWidthCell { x, y } => {
                write!(
                    f,
                    "cell at ({x}, {y}) has zero width without a covering wide cell"
                )
            }
            Self::LeadingCellOverhang { x, y, width } => {
                write!(
                    f,
                    "wide leading cell at ({x}, {y}) claims {width} columns past the row end"
                )
            }
            Self::Snapshot(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for ViewError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Snapshot(error) => Some(error),
            _ => None,
        }
    }
}

impl From<SnapshotError> for ViewError {
    fn from(error: SnapshotError) -> Self {
        Self::Snapshot(error)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Explicit cursor state for a pure-view capture."]
pub struct ViewCursor {
    #[doc = "Buffer-local cursor column."]
    col: u32,
    #[doc = "Buffer-local cursor row."]
    row: u32,
    #[doc = "Whether the terminal draws the cursor."]
    visible: bool,
    #[doc = "The shape the terminal draws the cursor as."]
    shape: SnapshotCursorShape,
    #[doc = "The resolved cursor color."]
    color: Rgb,
}

impl ViewCursor {
    #[doc = "Capture explicit cursor state."]
    #[must_use]
    pub const fn new(
        col: u32,
        row: u32,
        visible: bool,
        shape: SnapshotCursorShape,
        color: Rgb,
    ) -> Self {
        Self {
            col,
            row,
            visible,
            shape,
            color,
        }
    }

    #[doc = "Buffer-local cursor column."]
    #[must_use]
    pub const fn col(self) -> u32 {
        self.col
    }

    #[doc = "Buffer-local cursor row."]
    #[must_use]
    pub const fn row(self) -> u32 {
        self.row
    }

    #[doc = "Whether the terminal draws the cursor."]
    #[must_use]
    pub const fn visible(self) -> bool {
        self.visible
    }

    #[doc = "The shape the terminal draws the cursor as."]
    #[must_use]
    pub const fn shape(self) -> SnapshotCursorShape {
        self.shape
    }

    #[doc = "The resolved cursor color."]
    #[must_use]
    pub const fn color(self) -> Rgb {
        self.color
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Caller-supplied view settings for a pure-view capture."]
pub struct ViewSettings {
    #[doc = "Every resolved palette entry, indices 0 through 255."]
    pub(super) palette: [Rgb; 256],
    #[doc = "The resolved default foreground."]
    pub(super) default_foreground: Rgb,
    #[doc = "The resolved default background."]
    pub(super) default_background: Rgb,
    #[doc = "The explicit cursor state."]
    cursor: ViewCursor,
}

impl ViewSettings {
    #[doc = "Capture caller-supplied view settings with explicit cursor state."]
    #[must_use]
    pub const fn new(
        palette: [Rgb; 256],
        default_foreground: Rgb,
        default_background: Rgb,
        cursor: ViewCursor,
    ) -> Self {
        Self {
            palette,
            default_foreground,
            default_background,
            cursor,
        }
    }

    #[doc = "The built-in view profile: the fixed view palette with a hidden block cursor at the origin."]
    #[must_use]
    pub const fn builtin() -> Self {
        Self {
            palette: super::palette::build_view_palette(),
            default_foreground: Rgb::new(192, 192, 192),
            default_background: Rgb::new(0, 0, 0),
            cursor: ViewCursor::new(
                0,
                0,
                false,
                SnapshotCursorShape::Block,
                Rgb::new(192, 192, 192),
            ),
        }
    }

    #[doc = "Replace the resolved palette."]
    #[must_use]
    pub const fn with_palette(mut self, palette: [Rgb; 256]) -> Self {
        self.palette = palette;
        self
    }

    #[doc = "Replace the resolved default foreground and background."]
    #[must_use]
    pub const fn with_defaults(mut self, default_foreground: Rgb, default_background: Rgb) -> Self {
        self.default_foreground = default_foreground;
        self.default_background = default_background;
        self
    }

    #[doc = "Replace the explicit cursor state."]
    #[must_use]
    pub const fn with_cursor(mut self, cursor: ViewCursor) -> Self {
        self.cursor = cursor;
        self
    }

    #[doc = "Every resolved palette entry, indices 0 through 255."]
    #[must_use]
    pub const fn palette(&self) -> &[Rgb; 256] {
        &self.palette
    }

    #[doc = "The resolved default foreground."]
    #[must_use]
    pub const fn default_foreground(&self) -> Rgb {
        self.default_foreground
    }

    #[doc = "The resolved default background."]
    #[must_use]
    pub const fn default_background(&self) -> Rgb {
        self.default_background
    }

    #[doc = "The explicit cursor state."]
    #[must_use]
    pub const fn cursor(&self) -> ViewCursor {
        self.cursor
    }
}
