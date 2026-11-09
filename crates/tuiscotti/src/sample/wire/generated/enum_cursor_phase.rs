// @generated enum mapping for CursorPhase.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CursorPhase {
    Shown,
    Hidden,
}
impl CursorPhase {
    pub(super) fn from_wire(value: i32) -> Option<Self> {
        match value {
            1 => Some(Self::Shown),
            2 => Some(Self::Hidden),
            _ => None,
        }
    }
}
impl CursorPhase {
    pub(super) fn to_wire(self) -> i32 {
        match self {
            Self::Shown => 1,
            Self::Hidden => 2,
        }
    }
}
