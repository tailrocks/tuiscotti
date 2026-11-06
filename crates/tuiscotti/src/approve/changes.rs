use crate::check::{MismatchReason, PresentComparison};

pub(super) struct ChangeSet {
    pub(super) reason: Option<MismatchReason>,
    pub(super) fields: Vec<String>,
}

impl ChangeSet {
    pub(super) fn empty() -> Self {
        Self {
            reason: None,
            fields: Vec::new(),
        }
    }

    pub(super) fn missing() -> Self {
        Self {
            reason: Some(MismatchReason::Missing),
            fields: vec!["baseline".to_string()],
        }
    }

    pub(super) fn from_present(comparison: PresentComparison) -> Self {
        match comparison {
            PresentComparison::Pass => Self::empty(),
            PresentComparison::Failure {
                reason,
                changed_fields,
            } => Self {
                reason: Some(reason),
                fields: changed_fields,
            },
        }
    }
}
