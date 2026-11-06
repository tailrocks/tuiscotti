use std::cell::Cell;
use std::rc::Rc;

use super::decode_error::DecodeError;

const MIN_LIMIT: u64 = 32 * 1024 * 1024 * 1024;
const BASE: u64 = 19_301_445_335;
const RESOURCE: &str = "decoder requested allocations";
const MAX_CHARGES: u8 = 16;
const MAX_MESSAGE: u64 = 64 * 1024 * 1024;
const ERROR_FIXED: u64 = 65_536;
const ERROR_PER_MESSAGE_BYTE: u64 = 32;

// Mirrors the pinned Rust 1.99.1 RcInner<()> allocation layout; re-audit on toolchain changes.
#[repr(C, align(2))]
struct IdentityLayout {
    strong: Cell<usize>,
    weak: Cell<usize>,
    value: (),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Phase {
    Input,
    Codec,
    Scan,
    Mapping,
    Image,
}

#[derive(Debug)]
#[must_use]
pub(super) struct Charge {
    bytes: u64,
    identity: Rc<()>,
}

#[derive(Debug)]
pub(super) struct DecodeBudget {
    limit: u64,
    total: u64,
    peak: u64,
    charges: u8,
    identity: Rc<()>,
    phase: Phase,
    output_domain: Option<Charge>,
    output_base64: Option<Charge>,
    error_fixed: Option<Charge>,
    error_message: Option<Charge>,
    terminal: bool,
}

impl DecodeBudget {
    pub(super) fn new() -> Result<Self, DecodeError> {
        let limit = target_limit(std::env::consts::OS.len(), std::env::consts::ARCH.len())
            .ok_or_else(|| DecodeError::resource(RESOURCE, None, u64::MAX))?;
        let identity_bytes = u64::try_from(std::mem::size_of::<IdentityLayout>())
            .map_err(|_| DecodeError::resource(RESOURCE, None, limit))?;
        if identity_bytes > limit {
            return Err(DecodeError::resource(RESOURCE, Some(identity_bytes), limit));
        }
        let mut budget = Self {
            limit,
            total: identity_bytes,
            peak: identity_bytes,
            charges: 0,
            identity: Rc::new(()),
            phase: Phase::Input,
            output_domain: None,
            output_base64: None,
            error_fixed: None,
            error_message: None,
            terminal: false,
        };
        // Reserve fixed error source/details before any decode branch can fail. On failure the
        // budget drops only after the owned DecodeError exists; public conversion moves it and
        // performs no further decoder allocation.
        budget.error_fixed = Some(budget.charge("decoder error details", ERROR_FIXED)?);
        Ok(budget)
    }

    pub(super) fn reserve_message_error_envelope(
        &mut self,
        message_bytes: u64,
    ) -> Result<(), DecodeError> {
        if self.error_message.is_some() || message_bytes > MAX_MESSAGE {
            return Err(DecodeError::Invariant {
                detail: "invalid message error envelope reservation",
            });
        }
        let bytes = message_bytes
            .checked_mul(ERROR_PER_MESSAGE_BYTE)
            .ok_or_else(|| DecodeError::resource(RESOURCE, None, self.limit))?;
        self.error_message = Some(self.charge("decoder message error envelope", bytes)?);
        Ok(())
    }

    pub(super) fn charge(
        &mut self,
        _site: &'static str,
        bytes: u64,
    ) -> Result<Charge, DecodeError> {
        if self.terminal {
            return Err(DecodeError::Invariant {
                detail: "decoder budget used after terminal finish",
            });
        }
        if self.charges == MAX_CHARGES {
            return Err(DecodeError::Invariant {
                detail: "too many outstanding decoder charges",
            });
        }
        let total = self
            .total
            .checked_add(bytes)
            .ok_or_else(|| DecodeError::resource(RESOURCE, None, self.limit))?;
        if total > self.limit {
            return Err(DecodeError::resource(RESOURCE, Some(total), self.limit));
        }
        self.total = total;
        self.peak = self.peak.max(total);
        self.charges += 1;
        Ok(Charge {
            bytes,
            identity: Rc::clone(&self.identity),
        })
    }

    #[expect(
        clippy::needless_pass_by_value,
        reason = "releasing consumes the non-Clone charge"
    )]
    pub(super) fn release(&mut self, charge: Charge) -> Result<(), DecodeError> {
        if self.terminal {
            return Err(DecodeError::Invariant {
                detail: "decoder charge released after terminal finish",
            });
        }
        if !Rc::ptr_eq(&self.identity, &charge.identity) {
            return Err(DecodeError::Invariant {
                detail: "charge belongs to a different decoder budget",
            });
        }
        let total = self
            .total
            .checked_sub(charge.bytes)
            .ok_or(DecodeError::Invariant {
                detail: "decoder charge exceeds current total",
            })?;
        let charges = self.charges.checked_sub(1).ok_or(DecodeError::Invariant {
            detail: "decoder charge count underflow",
        })?;
        self.total = total;
        self.charges = charges;
        Ok(())
    }

    pub(super) fn retain_outputs(
        &mut self,
        domain: Charge,
        base64: Charge,
    ) -> Result<(), DecodeError> {
        if self.terminal {
            return Err(DecodeError::Invariant {
                detail: "output charge retained after terminal finish",
            });
        }
        if !Rc::ptr_eq(&self.identity, &domain.identity)
            || !Rc::ptr_eq(&self.identity, &base64.identity)
        {
            return Err(DecodeError::Invariant {
                detail: "charge belongs to a different decoder budget",
            });
        }
        if self.output_domain.is_some() || self.output_base64.is_some() {
            return Err(DecodeError::Invariant {
                detail: "duplicate retained output charge",
            });
        }
        self.output_domain = Some(domain);
        self.output_base64 = Some(base64);
        Ok(())
    }

    /// Seals accounting after the caller has released input, message, codec, and PNG leases.
    /// The two output charges stay in `total` and are consumed with this terminal ledger; they
    /// are never released while the returned Sample owns the corresponding allocations. Error
    /// envelopes are released here only on success, after no further decoder error can be built.
    pub(super) fn finish_output(&mut self) -> Result<(), DecodeError> {
        if self.terminal {
            return Err(DecodeError::Invariant {
                detail: "decoder budget finished more than once",
            });
        }
        let Some(domain) = self.output_domain.as_ref() else {
            return Err(DecodeError::Invariant {
                detail: "decoder budget missing retained domain charge",
            });
        };
        let Some(base64) = self.output_base64.as_ref() else {
            return Err(DecodeError::Invariant {
                detail: "decoder budget missing retained base64 charge",
            });
        };
        if !matches!(self.phase, Phase::Mapping | Phase::Image) {
            return Err(DecodeError::Invariant {
                detail: "decoder budget finished before mapping and image validation",
            });
        }
        let retained = domain
            .bytes
            .checked_add(base64.bytes)
            .ok_or(DecodeError::Invariant {
                detail: "retained output charge total overflow",
            })?;
        if self.charges != 4 || self.error_fixed.is_none() || self.error_message.is_none() {
            return Err(DecodeError::Invariant {
                detail: "decoder budget has live non-output charges at terminal finish",
            });
        }
        let error_message = self.error_message.take().ok_or(DecodeError::Invariant {
            detail: "decoder budget lost its message error envelope",
        })?;
        self.release(error_message)?;
        let error_fixed = self.error_fixed.take().ok_or(DecodeError::Invariant {
            detail: "decoder budget lost its fixed error envelope",
        })?;
        self.release(error_fixed)?;
        if self.charges != 2 {
            return Err(DecodeError::Invariant {
                detail: "decoder budget has live non-output charges at terminal finish",
            });
        }
        let identity_bytes =
            u64::try_from(std::mem::size_of::<IdentityLayout>()).map_err(|_| {
                DecodeError::Invariant {
                    detail: "decoder identity size does not fit accounting",
                }
            })?;
        if self.total
            != identity_bytes
                .checked_add(retained)
                .ok_or(DecodeError::Invariant {
                    detail: "terminal decoder total overflow",
                })?
        {
            return Err(DecodeError::Invariant {
                detail: "decoder total does not match retained output charges",
            });
        }
        self.terminal = true;
        Ok(())
    }

    pub(super) fn set_phase(&mut self, phase: Phase) -> Result<(), DecodeError> {
        if !matches!(
            (self.phase, phase),
            (Phase::Input, Phase::Codec)
                | (Phase::Codec, Phase::Scan)
                | (Phase::Scan, Phase::Mapping)
                | (Phase::Mapping, Phase::Image)
        ) {
            return Err(DecodeError::Invariant {
                detail: "invalid decoder phase transition",
            });
        }
        self.phase = phase;
        Ok(())
    }
    pub(super) fn peak(&self) -> u64 {
        self.peak
    }
    #[cfg(test)]
    pub(super) fn total(&self) -> u64 {
        self.total
    }
    pub(super) fn limit(&self) -> u64 {
        self.limit
    }
}

fn target_limit(os_len: usize, arch_len: usize) -> Option<u64> {
    let r = u64::try_from(os_len)
        .ok()?
        .checked_add(1)?
        .checked_add(u64::try_from(arch_len).ok()?)?;
    let target = 2_u64
        .checked_mul(r.max(8))?
        .checked_add(r.checked_add(8)?.max(32))?
        .checked_add(3_u64.checked_mul(r.checked_mul(10)?.checked_add(296)?.max(88))?)?
        .checked_add(8192)?;
    Some(MIN_LIMIT.max(BASE.checked_add(target)?))
}

#[cfg(test)]
#[path = "budget/tests.rs"]
mod tests;
