// @generated enum mapping for Source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Source {
    Live,
    Offline,
}
impl Source {
    pub(super) fn from_wire(value: i32) -> Option<Self> {
        match value {
            1 => Some(Self::Live),
            2 => Some(Self::Offline),
            _ => None,
        }
    }
}
impl Source {
    pub(super) fn to_wire(self) -> i32 {
        match self {
            Self::Live => 1,
            Self::Offline => 2,
        }
    }
}
