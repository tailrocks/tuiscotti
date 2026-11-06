use std::path::{Path, PathBuf};

use crate::check::MismatchReason;
use crate::sample::Gate;

#[derive(Debug, Clone)]
#[doc = "One completed approval transaction: the reviewed candidate is now the approved baseline."]
pub struct AcceptOutcome {
    #[doc = "The approved test ID."]
    pub(super) test_id: String,
    #[doc = "The gate both the candidate and the new baseline carry."]
    pub(super) gate: Gate,
    #[doc = "SHA-256 over the approved candidate bytes."]
    pub(super) candidate_sha256: String,
    #[doc = "The previous baseline digest, or `missing` for a first approval."]
    pub(super) previous_baseline_sha256: String,
    #[doc = "SHA-256 over the written baseline bytes, always the candidate digest."]
    pub(super) baseline_sha256: String,
    #[doc = "The upserted baseline file."]
    pub(super) baseline_path: PathBuf,
    #[doc = "The structured change reason for this approval."]
    pub(super) reason: Option<MismatchReason>,
    #[doc = "Changed fields recorded for this approval, in comparison order."]
    pub(super) changed_fields: Vec<String>,
}

impl AcceptOutcome {
    #[must_use]
    #[doc = "The approved test ID."]
    pub fn test_id(&self) -> &str {
        &self.test_id
    }

    #[must_use]
    #[doc = "The gate both the candidate and the new baseline carry."]
    pub const fn gate(&self) -> Gate {
        self.gate
    }

    #[must_use]
    #[doc = "SHA-256 over the approved candidate bytes."]
    pub fn candidate_sha256(&self) -> &str {
        &self.candidate_sha256
    }

    #[must_use]
    #[doc = "The previous baseline digest, or `missing` for a first approval."]
    pub fn previous_baseline_sha256(&self) -> &str {
        &self.previous_baseline_sha256
    }

    #[must_use]
    #[doc = "SHA-256 over the written baseline bytes, always the candidate digest."]
    pub fn baseline_sha256(&self) -> &str {
        &self.baseline_sha256
    }

    #[must_use]
    #[doc = "The upserted baseline file."]
    pub fn baseline_path(&self) -> &Path {
        &self.baseline_path
    }

    #[must_use]
    #[doc = "The structured change reason for this approval."]
    pub const fn reason(&self) -> Option<MismatchReason> {
        self.reason
    }

    #[must_use]
    #[doc = "Changed fields recorded for this approval, in comparison order."]
    pub fn changed_fields(&self) -> &[String] {
        &self.changed_fields
    }
}
