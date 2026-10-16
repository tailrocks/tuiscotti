//! Typed input vocabulary: keys, chords, mouse, focus, signals.

use super::error::TuiError;

/// A key independent of modifiers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Key {
    /// A Unicode character (letters, digits, punctuation, space, ...).
    Char(char),
    /// Enter/Return.
    Enter,
    /// Tab.
    Tab,
    /// Backspace.
    Backspace,
    /// Escape.
    Escape,
    /// Up arrow.
    Up,
    /// Down arrow.
    Down,
    /// Left arrow.
    Left,
    /// Right arrow.
    Right,
    /// Home.
    Home,
    /// End.
    End,
    /// Page up.
    PageUp,
    /// Page down.
    PageDown,
    /// Insert.
    Insert,
    /// Delete (forward delete).
    Delete,
    /// Function key 1..=12.
    F(u8),
}

/// Extra modifiers beyond ctrl/alt/shift.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct KeyExtMods {
    /// Super/Cmd/Win: needs the kitty keyboard protocol, else `Unsupported`.
    pub sup: bool,
}

/// Modifier set for [`Key`] input.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct KeyMods {
    /// Control held.
    pub ctrl: bool,
    /// Alt/Opt held.
    pub alt: bool,
    /// Shift held.
    pub shift: bool,
    /// Extra modifiers.
    pub ext: KeyExtMods,
}

impl KeyMods {
    /// No modifiers.
    pub const NONE: KeyMods = KeyMods {
        ctrl: false,
        alt: false,
        shift: false,
        ext: KeyExtMods { sup: false },
    };
    /// Control only.
    pub const CTRL: KeyMods = KeyMods {
        ctrl: true,
        alt: false,
        shift: false,
        ext: KeyExtMods { sup: false },
    };
    /// Alt only.
    pub const ALT: KeyMods = KeyMods {
        ctrl: false,
        alt: true,
        shift: false,
        ext: KeyExtMods { sup: false },
    };
    /// Shift only.
    pub const SHIFT: KeyMods = KeyMods {
        ctrl: false,
        alt: false,
        shift: true,
        ext: KeyExtMods { sup: false },
    };

    /// True when no modifier is held.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        !self.ctrl && !self.alt && !self.shift && !self.ext.sup
    }
}

/// Key event kind. Without the kitty keyboard protocol, `Up` (release)
/// emits no bytes — legacy encodings cannot represent releases — while
/// `Press`, `Down`, and `Repeat` all emit the key's legacy bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyEventKind {
    /// A complete press (down immediately followed by up).
    Press,
    /// Key down (no automatic release).
    Down,
    /// Auto-repeat tick while held.
    Repeat,
    /// Key release.
    Up,
}

/// Parse a typed chord such as `"Ctrl+P"`, `"ctrl-f"`, `"Alt+Enter"`,
/// `"alt-0"`, `"Shift+F5"`, `"backtab"`, `"F1"`, or a bare `"a"` / `"-"`.
///
/// Modifiers (case-insensitive, any order, joined by `+` or `-`):
/// `Ctrl`/`Control`/`Ctl`, `Alt`/`Opt`/`Meta`, `Shift`,
/// `Super`/`Cmd`/`Win`/`Command`. The final segment names the key:
/// `Enter`/`Return`, `Tab`, `BackTab` (`Shift+Tab`), `Backspace`/`BS`,
/// `Esc`/`Escape`, `Space`, `Plus` (or `"+"` / `"ctrl++"` / `"ctrl-+"`),
/// arrows, `Home`/`End`, `PageUp`/`PgUp`, `PageDown`/`PgDn`,
/// `Insert`/`Ins`, `Delete`/`Del`, `F1`..`F12`, or any single character.
///
/// # Errors
///
/// Returns `TuiError::Chord` for empty/unknown modifiers or keys.
pub fn parse_chord(text: &str) -> Result<(Key, KeyMods), TuiError> {
    let bad = |m: String| TuiError::Chord(m);
    if text.is_empty() {
        return Err(bad("empty chord".to_string()));
    }
    let mut mods = KeyMods::NONE;
    let mut rest = text;
    while rest.chars().count() > 1 {
        let Some(idx) = rest.find(['+', '-']) else {
            break;
        };
        let m = &rest[..idx];
        rest = &rest[idx + 1..];
        match m.to_ascii_lowercase().as_str() {
            "ctrl" | "control" | "ctl" => mods.ctrl = true,
            "alt" | "opt" | "meta" => mods.alt = true,
            "shift" => mods.shift = true,
            "super" | "cmd" | "win" | "windows" | "command" => mods.ext.sup = true,
            "" => return Err(bad(format!("empty modifier in chord {text:?}"))),
            other => return Err(bad(format!("unknown modifier {other:?} in chord {text:?}"))),
        }
    }
    let name = rest;
    if name.is_empty() {
        return Err(bad(format!("empty key in chord {text:?}")));
    }
    let key = match name.to_ascii_lowercase().as_str() {
        "enter" | "return" => Key::Enter,
        "tab" => Key::Tab,
        "backtab" => {
            mods.shift = true;
            Key::Tab
        }
        "backspace" | "bs" => Key::Backspace,
        "esc" | "escape" => Key::Escape,
        "space" => Key::Char(' '),
        "plus" => Key::Char('+'),
        "up" => Key::Up,
        "down" => Key::Down,
        "left" => Key::Left,
        "right" => Key::Right,
        "home" => Key::Home,
        "end" => Key::End,
        "pageup" | "pgup" => Key::PageUp,
        "pagedown" | "pgdn" => Key::PageDown,
        "insert" | "ins" => Key::Insert,
        "delete" | "del" => Key::Delete,
        _ => {
            if let Some(rest) = name.strip_prefix('F').or_else(|| name.strip_prefix('f'))
                && !rest.is_empty()
                && rest.bytes().all(|b| b.is_ascii_digit())
            {
                match rest.parse::<u8>() {
                    Ok(n) if !rest.starts_with('0') && (1..=12).contains(&n) => Key::F(n),
                    _ => return Err(bad(format!("bad function key {name:?}"))),
                }
            } else if name.chars().count() == 1 {
                Key::Char(name.chars().next().unwrap_or('?'))
            } else {
                return Err(bad(format!("unknown key {name:?}")));
            }
        }
    };
    Ok((key, mods))
}

/// Mouse button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    /// Left button.
    Left,
    /// Middle button.
    Middle,
    /// Right button.
    Right,
}

/// Wheel direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wheel {
    /// Wheel up.
    Up,
    /// Wheel down.
    Down,
    /// Wheel left.
    Left,
    /// Wheel right.
    Right,
}

/// Modifier set for mouse input.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MouseMods {
    /// Shift held.
    pub shift: bool,
    /// Alt held.
    pub alt: bool,
    /// Control held.
    pub ctrl: bool,
}

impl MouseMods {
    /// No modifiers.
    pub const NONE: MouseMods = MouseMods {
        shift: false,
        alt: false,
        ctrl: false,
    };
}

/// Process signal for [`Session::signal`](super::session::Session::signal). Unix only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    /// SIGINT.
    Int,
    /// Termination signal (`SIGTERM`).
    Term,
    /// Kill signal (`SIGKILL`).
    Kill,
    /// Quit signal (`SIGQUIT`).
    Quit,
    /// Hangup signal (`SIGHUP`).
    Hup,
    /// Raw signal number.
    Custom(i32),
}

#[cfg(unix)]
impl Signal {
    pub(crate) fn number(self) -> i32 {
        match self {
            Signal::Int => termpane::process::SIGINT,
            Signal::Term => termpane::process::SIGTERM,
            Signal::Kill => termpane::process::SIGKILL,
            Signal::Quit => termpane::process::SIGQUIT,
            Signal::Hup => termpane::process::SIGHUP,
            Signal::Custom(n) => n,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Key, KeyMods, parse_chord};

    #[test]
    fn parse_chord_distinguishes_letter_f_from_function_keys() {
        for (spec, expected) in [
            ("f", (Key::Char('f'), KeyMods::NONE)),
            ("F", (Key::Char('F'), KeyMods::NONE)),
            ("ctrl+f", (Key::Char('f'), KeyMods::CTRL)),
            ("ctrl-f", (Key::Char('f'), KeyMods::CTRL)),
            ("alt-0", (Key::Char('0'), KeyMods::ALT)),
            ("shift+F", (Key::Char('F'), KeyMods::SHIFT)),
            ("shift-F", (Key::Char('F'), KeyMods::SHIFT)),
            ("Alt+F", (Key::Char('F'), KeyMods::ALT)),
            ("backtab", (Key::Tab, KeyMods::SHIFT)),
            ("BackTab", (Key::Tab, KeyMods::SHIFT)),
            ("-", (Key::Char('-'), KeyMods::NONE)),
            ("ctrl+-", (Key::Char('-'), KeyMods::CTRL)),
            ("ctrl--", (Key::Char('-'), KeyMods::CTRL)),
            ("+", (Key::Char('+'), KeyMods::NONE)),
            ("ctrl++", (Key::Char('+'), KeyMods::CTRL)),
            ("ctrl-+", (Key::Char('+'), KeyMods::CTRL)),
            ("plus", (Key::Char('+'), KeyMods::NONE)),
            ("ctrl+plus", (Key::Char('+'), KeyMods::CTRL)),
            ("F1", (Key::F(1), KeyMods::NONE)),
            ("F12", (Key::F(12), KeyMods::NONE)),
            ("f12", (Key::F(12), KeyMods::NONE)),
            ("Ctrl+F5", (Key::F(5), KeyMods::CTRL)),
        ] {
            assert_eq!(parse_chord(spec).expect(spec), expected, "{spec}");
        }
        for spec in ["Ctrl+Shift+f", "ctrl-shift-f"] {
            let (k, m) = parse_chord(spec).expect("ctrl+shift+f");
            assert_eq!(k, Key::Char('f'));
            assert!(m.ctrl && m.shift && !m.alt);
        }
        let (bt_k, bt_m) = parse_chord("ctrl+backtab").expect("ctrl+backtab");
        assert_eq!(bt_k, Key::Tab);
        assert!(bt_m.ctrl && bt_m.shift && !bt_m.alt);

        for bad_fn in ["F0", "F01", "f01", "F13"] {
            let err = parse_chord(bad_fn)
                .expect_err("invalid function key")
                .to_string();
            assert!(err.contains("bad function key"), "{bad_fn}: {err}");
        }
        for bad_spec in ["fx", "foo-bar", "++", "Ctrl+"] {
            assert!(parse_chord(bad_spec).is_err(), "{bad_spec} must fail");
        }
    }
}
