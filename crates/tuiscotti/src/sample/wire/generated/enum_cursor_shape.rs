// @generated enum mapping for CursorShape.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CursorShape {
    Block,
    Underline,
    Bar,
}
impl CursorShape {
    pub(super) fn from_wire(value: i32) -> Option<Self> {
        match value {
            1 => Some(Self::Block),
            2 => Some(Self::Underline),
            3 => Some(Self::Bar),
            _ => None,
        }
    }
}
impl CursorShape {
    pub(super) fn to_wire(self) -> i32 {
        match self {
            Self::Block => 1,
            Self::Underline => 2,
            Self::Bar => 3,
        }
    }
}
