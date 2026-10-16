//! Session-bound locators: immediate lookup, retrying expectation, atomic clicks (F11).
//!
//! [`BoundLocator`] ties a [`Locator`] to a live [`Session`]. Ordinary
//! callers need no fabricated revisions and no hand-rolled observer
//! closures: every method observes afresh through the session.
//!
//! - **Immediate lookup** ([`BoundLocator::visible_now`]): one fresh
//!   observation, resolved to the UNIQUE viewport target. Ambiguous,
//!   missing, or scrollback-only targets fail at once, without waiting.
//! - **Retrying expectation** ([`BoundLocator::expect_visible`]): the same
//!   unique-target resolution, retried on fresh observations to ONE bounded
//!   default deadline ([`BoundLocator::DEFAULT_EXPECT_VISIBLE`]);
//!   [`BoundLocator::expect_visible_within`] overrides the bound for
//!   advanced callers. Permanent errors
//!   ([`LocateError::Usage`]/[`LocateError::Unsupported`]) fail immediately,
//!   never wait out the deadline; anything else resolves to
//!   [`LocateError::Timeout`] naming the last observed state.
//! - **Atomic click** ([`BoundLocator::click`]): the OWNING worker builds one
//!   fresh observation, resolves the unique target at its own current
//!   revision, and delivers press + release with no interleaving op — one
//!   input submission, never two reads plus a separate unchecked click. The
//!   acted-on [`Span`] is returned so the caller can audit what was clicked.
//!   The click is attempted at most once: a reply timeout means the click
//!   may already have been delivered, so it is never retried.
//!
//! ## Remaining race (external, honest)
//!
//! The worker resolves and delivers atomically against the EMULATOR, but the
//! application reads the delivered bytes later: if the app redraws between
//! the emulator state the click resolved against and its own input
//! processing, the click lands where the target was, not where it is. No
//! harness-side re-validation can close that window — the bytes are already
//! in flight. Tests that need app-level confirmation must wait on an
//! app-observable effect after clicking (a redrawn marker, a mode change),
//! never on the click's return alone.
//!
//! Detached query evaluation ([`Locator::resolve`], [`Locator::resolve_obs`],
//! the detached `expect_*` family, [`PendingAction`](tuiscotti_core::locate::PendingAction))
//! stays available in `tuiscotti_core::locate` for advanced offline use.

use std::time::{Duration, Instant};

use tuiscotti_core::locate::{LocateError, Locator, Span};

use crate::tui::{MouseButton, MouseMods, Session};

/// A locator action refused or failed: either side retains its typed source.
#[derive(Debug)]
pub enum ActionError {
    /// Fresh observation or input delivery failed.
    Session(crate::tui::TuiError),
    /// Resolution, uniqueness, or staleness refused the action.
    Locate(LocateError),
}

impl std::fmt::Display for ActionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Session(e) => write!(f, "session failed: {e}"),
            Self::Locate(e) => write!(f, "locator refused: {e}"),
        }
    }
}

impl std::error::Error for ActionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Session(e) => Some(e),
            Self::Locate(e) => Some(e),
        }
    }
}

impl From<crate::tui::TuiError> for ActionError {
    fn from(e: crate::tui::TuiError) -> Self {
        Self::Session(e)
    }
}

impl From<LocateError> for ActionError {
    fn from(e: LocateError) -> Self {
        Self::Locate(e)
    }
}

/// Multi-match resolution policy for [`BoundLocator`].
///
/// [`MatchPolicy::Unique`] is the default: ambiguous matches fail closed
/// with [`LocateError::Ambiguous`] unless the caller explicitly opts into
/// [`BoundLocator::first`], [`BoundLocator::last`], [`BoundLocator::nth`],
/// [`BoundLocator::any`], or [`BoundLocator::count_at_least`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MatchPolicy {
    /// Require exactly one match (`0` → `NotFound`, `2+` → `Ambiguous`). Default.
    #[default]
    Unique,
    /// Succeed when at least one match is present (returns the first match in resolution order).
    Any,
    /// Succeed when at least `min` matches are present (returns the first match in resolution order).
    CountAtLeast(usize),
}

/// A [`Locator`] bound to a live [`Session`].
///
/// Created by [`Session::get_by`] / [`Session::get_by_text`]. `Send + Sync`
/// when the session is shared; each call observes afresh. Resolution defaults
/// to [`MatchPolicy::Unique`].
#[derive(Debug, Clone)]
pub struct BoundLocator<'s> {
    session: &'s Session,
    locator: Locator,
    policy: MatchPolicy,
}

impl Session {
    /// Bind an arbitrary locator (scoped composition via
    /// [`Locator::within`], [`Locator::before`], [`Locator::after`], ...).
    #[must_use]
    pub fn get_by(&self, locator: Locator) -> BoundLocator<'_> {
        BoundLocator {
            session: self,
            locator,
            policy: MatchPolicy::Unique,
        }
    }

    /// Bind a text locator ([`Locator::text`]) to this session.
    #[must_use]
    pub fn get_by_text(&self, text: &str) -> BoundLocator<'_> {
        self.get_by(Locator::text(text.to_string()))
    }
}

/// Poll cadence for the retrying expectation (matches the detached
/// `expect_*` family in `tuiscotti_core::locate`, which owns its own copy).
const BOUND_POLL: Duration = Duration::from_millis(5);

impl BoundLocator<'_> {
    /// Default bound for [`BoundLocator::expect_visible`] (10s, matching
    /// [`Session::DEFAULT_WAIT`](crate::tui::Session::DEFAULT_WAIT)).
    pub const DEFAULT_EXPECT_VISIBLE: Duration = Duration::from_secs(10);

    /// The underlying detached locator (for offline composition).
    #[must_use]
    pub fn locator(&self) -> &Locator {
        &self.locator
    }

    /// Active multi-match resolution policy (defaults to [`MatchPolicy::Unique`]).
    #[must_use]
    pub fn policy(&self) -> MatchPolicy {
        self.policy
    }

    /// Set the multi-match resolution [`MatchPolicy`].
    #[must_use]
    pub fn with_policy(mut self, policy: MatchPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// Select the first match in resolution order (scrollback oldest-first,
    /// then viewport row-major).
    #[must_use]
    pub fn first(mut self) -> Self {
        self.locator = self.locator.first();
        self.policy = MatchPolicy::Unique;
        self
    }

    /// Select the last match in resolution order.
    #[must_use]
    pub fn last(mut self) -> Self {
        self.locator = self.locator.last();
        self.policy = MatchPolicy::Unique;
        self
    }

    /// Select the `index`-th match in resolution order (0-based).
    #[must_use]
    pub fn nth(mut self, index: usize) -> Self {
        self.locator = self.locator.nth(index);
        self.policy = MatchPolicy::Unique;
        self
    }

    /// Allow any positive match count (`>= 1`), returning the first match in
    /// resolution order instead of failing with [`LocateError::Ambiguous`].
    #[must_use]
    pub fn any(mut self) -> Self {
        self.policy = MatchPolicy::Any;
        self
    }

    /// Require at least `min` matches on [`BoundLocator::visible_now`] /
    /// [`BoundLocator::expect_visible`], returning the first match in
    /// resolution order once `>= min` matches are present.
    #[must_use]
    pub fn count_at_least(mut self, min: usize) -> Self {
        self.policy = MatchPolicy::CountAtLeast(min);
        self
    }

    /// Match physical rows only (disable wrapped-line joining on the underlying locator).
    #[must_use]
    pub fn physical_rows(mut self) -> Self {
        self.locator = self.locator.physical_rows();
        self
    }

    /// Scope this locator to matches fully contained in `scope`.
    #[must_use]
    pub fn within(mut self, scope: Locator) -> Self {
        self.locator = Locator::within(scope, self.locator);
        self
    }

    fn resolve_policy(
        &self,
        screen: &tuiscotti_core::screen::Screen,
        revision: u64,
    ) -> Result<Span, LocateError> {
        match self.policy {
            MatchPolicy::Unique => self.locator.resolve_unique(screen, revision),
            MatchPolicy::Any => {
                let spans = self.locator.resolve(screen, revision)?;
                spans
                    .into_iter()
                    .next()
                    .ok_or_else(|| LocateError::NotFound {
                        message: "no matches".to_string(),
                    })
            }
            MatchPolicy::CountAtLeast(min) => {
                if min == 0 {
                    return Err(LocateError::Usage(
                        "count_at_least requires min >= 1".to_string(),
                    ));
                }
                let spans = self.locator.resolve_at_least(min, screen, revision)?;
                spans
                    .into_iter()
                    .next()
                    .ok_or_else(|| LocateError::NotFound {
                        message: "no matches".to_string(),
                    })
            }
        }
    }

    /// Immediate lookup: one fresh observation, resolved according to
    /// [`BoundLocator::policy`] (default [`MatchPolicy::Unique`]). Ambiguous
    /// (under `Unique`), missing, or scrollback-only targets fail at once —
    /// use [`BoundLocator::expect_visible`] to wait for one.
    /// # Errors
    ///
    /// Returns [`ActionError`] when observation fails or resolution refuses.
    pub fn visible_now(&self) -> Result<Span, ActionError> {
        let obs = self.session.observe_now()?;
        Ok(self.resolve_policy(&obs.screen, obs.revision)?)
    }

    /// Retrying expectation: [`BoundLocator::expect_visible_within`] with the
    /// bounded default deadline ([`BoundLocator::DEFAULT_EXPECT_VISIBLE`]).
    /// # Errors
    ///
    /// Returns [`ActionError`] when observation fails, permanently on
    /// [`LocateError::Usage`]/[`LocateError::Unsupported`], or with
    /// [`LocateError::Timeout`] past the deadline.
    pub fn expect_visible(&self) -> Result<Span, ActionError> {
        self.expect_visible_within(Self::DEFAULT_EXPECT_VISIBLE)
    }

    /// Retrying expectation with an explicit bound (advanced override):
    /// poll fresh observations until the locator resolves according to
    /// [`BoundLocator::policy`] (default: a UNIQUE viewport target), or the
    /// ONE `timeout` deadline. Permanent errors fail immediately without
    /// waiting; at the deadline the error is [`LocateError::Timeout`] naming
    /// the last observed state. Readiness retries never touch the input sink.
    /// # Errors
    ///
    /// Returns [`ActionError`] when observation fails, permanently on
    /// [`LocateError::Usage`]/[`LocateError::Unsupported`], or with
    /// [`LocateError::Timeout`] past the deadline.
    pub fn expect_visible_within(&self, timeout: Duration) -> Result<Span, ActionError> {
        let deadline = Instant::now() + timeout;
        loop {
            let obs = self.session.observe_now()?;
            match self.resolve_policy(&obs.screen, obs.revision) {
                Ok(span) => return Ok(span),
                Err(e) if e.is_immediate() => return Err(ActionError::Locate(e)),
                Err(e) => {
                    if Instant::now() >= deadline {
                        let prefix = match self.policy {
                            MatchPolicy::Unique => "no unique visible target",
                            MatchPolicy::Any => "no visible match",
                            MatchPolicy::CountAtLeast(_) => "insufficient visible matches",
                        };
                        return Err(ActionError::Locate(LocateError::Timeout {
                            waited: timeout,
                            reason: format!("{prefix}: {e}"),
                        }));
                    }
                }
            }
            std::thread::sleep(BOUND_POLL);
        }
    }

    /// Poll fresh observations until at least `min` viewport matches are
    /// present within [`BoundLocator::DEFAULT_EXPECT_VISIBLE`], returning all
    /// resolved [`Span`]s in resolution order.
    ///
    /// # Errors
    ///
    /// Returns [`ActionError`] when observation fails, permanently on
    /// [`LocateError::Usage`]/[`LocateError::Unsupported`], or with
    /// [`LocateError::Timeout`] past the deadline.
    pub fn expect_count_at_least(&self, min: usize) -> Result<Vec<Span>, ActionError> {
        self.expect_count_at_least_within(min, Self::DEFAULT_EXPECT_VISIBLE)
    }

    /// [`BoundLocator::expect_count_at_least`] with an explicit timeout.
    ///
    /// # Errors
    ///
    /// Same failures as [`BoundLocator::expect_count_at_least`].
    pub fn expect_count_at_least_within(
        &self,
        min: usize,
        timeout: Duration,
    ) -> Result<Vec<Span>, ActionError> {
        if min == 0 {
            return Err(ActionError::Locate(LocateError::Usage(
                "expect_count_at_least requires min >= 1".to_string(),
            )));
        }
        let deadline = Instant::now() + timeout;
        loop {
            let obs = self.session.observe_now()?;
            match self
                .locator
                .resolve_at_least(min, &obs.screen, obs.revision)
            {
                Ok(spans) => return Ok(spans),
                Err(e) if e.is_immediate() => return Err(ActionError::Locate(e)),
                Err(e) => {
                    if Instant::now() >= deadline {
                        return Err(ActionError::Locate(LocateError::Timeout {
                            waited: timeout,
                            reason: format!("insufficient visible matches: {e}"),
                        }));
                    }
                }
            }
            std::thread::sleep(BOUND_POLL);
        }
    }

    /// Left-click the viewport target as ONE worker step: the owning
    /// worker builds one fresh observation, resolves the target at its
    /// own current revision, and delivers press + release with no
    /// interleaving op. Returns the acted-on [`Span`] (audit what was
    /// clicked, including its revision).
    ///
    /// Single attempt, never retried: on a reply timeout the click may
    /// already have been delivered, and a retry would send a second press.
    /// See the module docs for the remaining external application race.
    /// # Errors
    ///
    /// Returns [`ActionError`] when resolution refuses (not found, ambiguous,
    /// scrollback-only), delivery is refused (mouse reporting off, child
    /// exited, session closed), or the reply times out (delivery then
    /// unknown — do not retry).
    pub fn click(&self) -> Result<Span, ActionError> {
        let target = match self.policy {
            MatchPolicy::Unique | MatchPolicy::CountAtLeast(_) => self.locator.clone(),
            MatchPolicy::Any => self.locator.clone().first(),
        };
        self.session
            .click_target(target, MouseButton::Left, MouseMods::NONE)
    }
}
