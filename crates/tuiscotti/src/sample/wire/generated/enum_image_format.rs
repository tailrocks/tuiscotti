// @generated enum mapping for ImageFormat.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ImageFormat {
    Png,
}
impl ImageFormat {
    pub(super) fn from_wire(value: i32) -> Option<Self> {
        match value {
            1 => Some(Self::Png),
            _ => None,
        }
    }
}
impl ImageFormat {
    pub(super) fn to_wire(self) -> i32 {
        match self {
            Self::Png => 1,
        }
    }
}
