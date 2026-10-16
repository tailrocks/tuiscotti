//! Session errors, wait failures, and cooperative cancellation.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tuiscotti_core::screen::Observation;

/// Session failure.
#[derive(Debug, Clone)]
pub enum TuiError {
    /// Spawn failed (unresolved binary, PTY open, validation, ...).
    Spawn(String),
    /// Invalid argument (bad size, coordinates, empty input, ...).
    InvalidInput(String),
    /// Unparseable chord.
    Chord(String),
    /// Capability the backend cannot provide. Fails closed, never emulated.
    Unsupported(&'static str),
    /// Input refused: the application has not enabled the mode (mouse
    /// reporting, focus tracking, ...) that would make it meaningful.
    ModeNotEnabled(&'static str),
    /// Paste rejected: content contains the bracketed-paste delimiters.
    PasteRejected(String),
    /// PTY I/O failure.
    Io(String),
    /// Deadlines: wait expired / worker unresponsive.
    Timeout(String),
    /// Input or observation refused: the child already exited.
    ChildExited(String),
    /// Session is closed (or its worker is gone).
    Closed(String),
    /// Teardown itself failed (kill/reap/join errors from finish/close).
    Teardown(String),
    /// Signal delivery failed.
    Signal(String),
    /// `ExitWait::success` / `code` assertion failed.
    Assertion(String),
}

impl std::fmt::Display for TuiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Spawn(m) => write!(f, "spawn failed: {m}"),
            Self::InvalidInput(m) => write!(f, "invalid input: {m}"),
            Self::Chord(m) => write!(f, "bad chord: {m}"),
            Self::Unsupported(m) => write!(f, "unsupported: {m}"),
            Self::ModeNotEnabled(m) => write!(f, "mode not enabled: {m}"),
            Self::PasteRejected(m) => write!(f, "paste rejected: {m}"),
            Self::Io(m) => write!(f, "pty io: {m}"),
            Self::Timeout(m) => write!(f, "timeout: {m}"),
            Self::ChildExited(m) => write!(f, "child exited: {m}"),
            Self::Closed(m) => write!(f, "session closed: {m}"),
            Self::Teardown(m) => write!(f, "teardown failed: {m}"),
            Self::Signal(m) => write!(f, "signal failed: {m}"),
            Self::Assertion(m) => write!(f, "exit assertion failed: {m}"),
        }
    }
}

impl std::error::Error for TuiError {}

/// Wait failure. Every variant carries the latest evidence snapshot; a wait
/// never reports success without its condition holding. Evidence is boxed:
/// snapshots are large and only travel on failure paths.
#[derive(Debug, Clone)]
pub enum WaitError {
    /// Deadline passed before the condition held.
    Timeout {
        /// Time spent waiting.
        waited: Duration,
        /// Latest snapshot when the wait gave up.
        evidence: Box<Observation>,
    },
    /// Cancelled via the token.
    Cancelled {
        /// Latest snapshot when the wait gave up.
        evidence: Box<Observation>,
    },
    /// `wait_frame` only: the backend cannot track synchronized frames.
    Unsupported {
        /// Capability the backend lacks.
        capability: &'static str,
        /// Latest snapshot when the wait gave up.
        evidence: Box<Observation>,
    },
    /// The session closed before the condition held.
    Closed {
        /// Latest snapshot, if any was published.
        evidence: Option<Box<Observation>>,
    },
}

impl WaitError {
    /// Evidence snapshot carried by this failure, if any.
    #[must_use]
    pub fn evidence(&self) -> Option<&Observation> {
        match self {
            Self::Timeout { evidence, .. }
            | Self::Cancelled { evidence }
            | Self::Unsupported { evidence, .. } => Some(evidence.as_ref()),
            Self::Closed { evidence } => evidence.as_deref(),
        }
    }
}

/// Maximum rows included in a [`WaitError`] [`std::fmt::Display`] screen dump.
const MAX_DUMP_ROWS: usize = 40;

/// Maximum UTF-8 bytes of screen text included in a [`WaitError`] dump.
const MAX_DUMP_BYTES: usize = 4096;

fn bounded_screen_dump(screen: &tuiscotti_core::screen::Screen) -> String {
    let mut rows: Vec<String> = (0..screen.rows())
        .map(|y| {
            let mut row = String::new();
            for x in 0..screen.cols() {
                if let Some(c) = screen.get(x, y)
                    && !c.continuation
                {
                    row.push_str(&c.symbol);
                }
            }
            row.trim_end().to_string()
        })
        .collect();
    while rows.last().is_some_and(String::is_empty) {
        rows.pop();
    }
    if rows.is_empty() {
        return "<blank screen>".to_string();
    }
    let total_rows = rows.len();
    let shown_rows = total_rows.min(MAX_DUMP_ROWS);
    let mut out = String::new();
    let mut byte_truncated = false;
    for (idx, row) in rows.into_iter().take(shown_rows).enumerate() {
        if idx > 0 {
            if out.len() + 1 > MAX_DUMP_BYTES {
                byte_truncated = true;
                break;
            }
            out.push('\n');
        }
        if out.len() + row.len() > MAX_DUMP_BYTES {
            let budget = MAX_DUMP_BYTES.saturating_sub(out.len());
            let mut end = budget.min(row.len());
            while end > 0 && !row.is_char_boundary(end) {
                end -= 1;
            }
            out.push_str(&row[..end]);
            byte_truncated = true;
            break;
        }
        out.push_str(&row);
    }
    if byte_truncated {
        out.push_str("\n... (truncated)");
    } else if total_rows > shown_rows {
        use std::fmt::Write as _;
        write!(
            out,
            "\n... ({} more row(s) omitted)",
            total_rows - shown_rows
        )
        .ok();
    }
    out
}

impl std::fmt::Display for WaitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Timeout { waited, evidence } => write!(
                f,
                "wait timed out after {waited:?} (evidence at revision {})\n--- last screen ({}x{}) ---\n{}",
                evidence.revision,
                evidence.screen.cols(),
                evidence.screen.rows(),
                bounded_screen_dump(&evidence.screen)
            ),
            Self::Cancelled { evidence } => write!(
                f,
                "wait cancelled (evidence at revision {})\n--- last screen ({}x{}) ---\n{}",
                evidence.revision,
                evidence.screen.cols(),
                evidence.screen.rows(),
                bounded_screen_dump(&evidence.screen)
            ),
            Self::Unsupported {
                capability,
                evidence,
            } => write!(
                f,
                "wait unsupported ({capability}; evidence at revision {})\n--- last screen ({}x{}) ---\n{}",
                evidence.revision,
                evidence.screen.cols(),
                evidence.screen.rows(),
                bounded_screen_dump(&evidence.screen)
            ),
            Self::Closed {
                evidence: Some(evidence),
            } => write!(
                f,
                "wait failed: session closed (evidence at revision {})\n--- last screen ({}x{}) ---\n{}",
                evidence.revision,
                evidence.screen.cols(),
                evidence.screen.rows(),
                bounded_screen_dump(&evidence.screen)
            ),
            Self::Closed { evidence: None } => write!(f, "wait failed: session closed"),
        }
    }
}

impl std::error::Error for WaitError {}

/// Cooperative cancellation token shared across threads. Every blocking wait
/// takes one; cancelling unblocks the wait with [`WaitError::Cancelled`].
#[derive(Debug, Clone, Default)]
pub struct CancelToken {
    flag: Arc<AtomicBool>,
}

impl CancelToken {
    /// Fresh uncancelled token.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Cancel: unblock every wait holding a clone.
    pub fn cancel(&self) {
        self.flag.store(true, Ordering::SeqCst);
    }

    /// True once [`CancelToken::cancel`] ran.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::WaitError;
    use std::time::Duration;
    use tuiscotti_core::frame::{Cell, Color, Cursor, Mods};
    use tuiscotti_core::screen::{
        CaptureProvenance, CaptureReason, Observation, Screen, TermState,
    };

    fn obs_with_rows(cols: u16, rows: u16, lines: &[&str], rev: u64) -> Box<Observation> {
        let mut cells = Vec::with_capacity((cols as usize) * (rows as usize));
        for y in 0..rows {
            let line = lines.get(y as usize).copied().unwrap_or("");
            for x in 0..cols {
                let ch = line.chars().nth(x as usize).unwrap_or(' ');
                cells.push(Cell {
                    x,
                    y,
                    symbol: ch.to_string(),
                    width: 1,
                    continuation: false,
                    fg: Color::Default,
                    bg: Color::Default,
                    mods: Mods::default(),
                    underline_color: Color::Default,
                });
            }
        }
        let screen = Screen::validate(cols, rows, 0, 0, cells, Cursor::default())
            .expect("valid test screen");
        Box::new(Observation::new(
            screen,
            rev,
            CaptureReason::Manual,
            TermState::default(),
            CaptureProvenance::new(0, None, None, 0),
        ))
    }

    #[test]
    fn wait_error_display_includes_bounded_screen_dump() {
        let err = WaitError::Timeout {
            waited: Duration::from_millis(250),
            evidence: obs_with_rows(20, 4, &["Login prompt", "Password:"], 3),
        };
        let msg = err.to_string();
        assert!(
            msg.starts_with("wait timed out after 250ms (evidence at revision 3)"),
            "{msg}"
        );
        assert!(msg.contains("--- last screen (20x4) ---"), "{msg}");
        assert!(msg.contains("Login prompt\nPassword:"), "{msg}");

        let blank = WaitError::Cancelled {
            evidence: obs_with_rows(10, 3, &[], 1),
        };
        assert!(blank.to_string().contains("<blank screen>"), "{blank}");

        let owned: Vec<String> = (0..50).map(|i| format!("row-{i}")).collect();
        let refs: Vec<&str> = owned.iter().map(String::as_str).collect();
        let capped = WaitError::Unsupported {
            capability: "sync",
            evidence: obs_with_rows(20, 50, &refs, 9),
        };
        let capped_msg = capped.to_string();
        assert!(capped_msg.contains("row-0"), "{capped_msg}");
        assert!(capped_msg.contains("row-39"), "{capped_msg}");
        assert!(!capped_msg.contains("row-40"), "{capped_msg}");
        assert!(
            capped_msg.contains("... (10 more row(s) omitted)"),
            "{capped_msg}"
        );
    }
}
