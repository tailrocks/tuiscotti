// @generated enum mapping for Underline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Underline {
    None,
    Single,
    Double,
    Curly,
    Dotted,
    Dashed,
}
impl Underline {
    pub(super) fn from_wire(value: i32) -> Option<Self> {
        match value {
            1 => Some(Self::None),
            2 => Some(Self::Single),
            3 => Some(Self::Double),
            4 => Some(Self::Curly),
            5 => Some(Self::Dotted),
            6 => Some(Self::Dashed),
            _ => None,
        }
    }
}
impl Underline {
    pub(super) fn to_wire(self) -> i32 {
        match self {
            Self::None => 1,
            Self::Single => 2,
            Self::Double => 3,
            Self::Curly => 4,
            Self::Dotted => 5,
            Self::Dashed => 6,
        }
    }
}
