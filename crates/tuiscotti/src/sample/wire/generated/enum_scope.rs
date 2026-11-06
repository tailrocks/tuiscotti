// @generated enum mapping for Scope.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Scope {
    Viewport,
    ActiveScreen,
    FullHistory,
}
impl Scope {
    pub(super) fn from_wire(value: i32) -> Option<Self> {
        match value {
            1 => Some(Self::Viewport),
            2 => Some(Self::ActiveScreen),
            3 => Some(Self::FullHistory),
            _ => None,
        }
    }
}
impl Scope {
    pub(super) fn to_wire(self) -> i32 {
        match self {
            Self::Viewport => 1,
            Self::ActiveScreen => 2,
            Self::FullHistory => 3,
        }
    }
}
