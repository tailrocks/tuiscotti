// @generated enum mapping for Gate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Gate {
    State,
    StateAndPixels,
}
impl Gate {
    pub(super) fn from_wire(value: i32) -> Option<Self> {
        match value {
            1 => Some(Self::State),
            2 => Some(Self::StateAndPixels),
            _ => None,
        }
    }
}
impl Gate {
    pub(super) fn to_wire(self) -> i32 {
        match self {
            Self::State => 1,
            Self::StateAndPixels => 2,
        }
    }
}
