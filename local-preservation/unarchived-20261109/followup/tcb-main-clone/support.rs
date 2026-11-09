use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use junie_tui::clock::TestRequest;
use tui_test::{
    CheckedMouseAction, KeyAction, LaunchEnvMode, LaunchOptions, MouseOptions, OpenOptions,
    Operation, OperationResult, RunOptions, Session, SnapshotCell, SnapshotScope, TerminalSnapshot,
    WaitQuietOptions,
};

const TIMEOUT: Duration = Duration::from_secs(10);

fn evidence_root() -> PathBuf {
    if let Some(root) = std::env::var_os("TUISCOTTI_CAPTURE_ROOT") {
        return PathBuf::from(root);
    }
    PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .parent()
        .expect("the target directory holds the tmp directory")
        .join("functional")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Point {
    pub x: u16,
    pub y: u16,
}

pub struct App {
    session: Session,
    directory: PathBuf,
    actions: RefCell<Vec<String>>,
    checkpoint: Cell<usize>,
}

impl App {
    pub fn new(name: &str, bin: &str, args: &[&str], cols: u16, rows: u16, color: &str) -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = evidence_root().join(format!("{name}-{}-{stamp}", std::process::id()));
        let home = directory.join("home");
        std::fs::create_dir_all(&home).expect("create isolated application home");
        let mut launch = LaunchOptions::new(bin)
            .with_args(args.iter().copied())
            .with_cwd(&home)
            .with_env_mode(LaunchEnvMode::Cleared)
            .with_env("HOME", &home)
            .with_env("XDG_CONFIG_HOME", home.join("config"))
            .with_env("XDG_CACHE_HOME", home.join("cache"))
            .with_env("XDG_DATA_HOME", home.join("data"))
            .with_env("SHELL", "/bin/sh")
            .with_env("PATH", "/usr/bin:/bin")
            .with_env("TERM", "xterm-256color")
            .with_env("COLORTERM", "truecolor")
            .with_env("LANG", "C.UTF-8")
            .with_env("LC_ALL", "C.UTF-8")
            .with_env("TZ", "UTC")
            .with_env("JUNIE_TUI_CLIENT_PLATFORM", "macos")
            .with_env("JUNIE_TUI_TEST_CLOCK", "1")
            .with_env("HOLLA_NO_HISTORY", "1");
        if color == "nocolor" {
            launch = launch.with_env("NO_COLOR", "1");
        } else {
            launch = launch.with_args(["--color", color]);
        }
        let app = Self {
            session: Session::new(name),
            directory,
            actions: RefCell::new(vec![format!("spawn {bin} {args:?} {cols}x{rows} {color}")]),
            checkpoint: Cell::new(0),
        };
        let defaults = OpenOptions::default();
        app.session
            .spawn_with(
                launch,
                RunOptions {
                    backend: defaults.backend,
                    program: String::new(),
                    args: Vec::new(),
                    profile: defaults.profile,
                    cols,
                    rows,
                    cwd: None,
                    env: Vec::new(),
                    wait_ready: Some(false),
                    restart: false,
                    timeouts: defaults.timeouts,
                    recording: defaults.recording,
                    style: defaults.style,
                },
            )
            .expect("spawn real application on a PTY");
        app
    }

    pub fn screen(&self) -> TerminalSnapshot {
        self.fence();
        self.capture()
    }

    fn capture(&self) -> TerminalSnapshot {
        self.session
            .capture_snapshot(SnapshotScope::ActiveScreen)
            .expect("capture live terminal")
    }

    pub fn wait_screen(
        &self,
        reason: &str,
        predicate: impl Fn(&TerminalSnapshot) -> bool,
    ) -> TerminalSnapshot {
        self.wait_screen_within(reason, TIMEOUT, predicate)
    }

    fn wait_screen_within(
        &self,
        reason: &str,
        budget: Duration,
        predicate: impl Fn(&TerminalSnapshot) -> bool,
    ) -> TerminalSnapshot {
        let deadline = Instant::now() + budget;
        loop {
            let screen = self.screen();
            self.assert_running(reason);
            if predicate(&screen) {
                self.record(reason, &screen);
                return screen;
            }
            assert!(
                Instant::now() < deadline,
                "timed out: {reason}\n{}",
                text(&screen)
            );
            std::thread::sleep(Duration::from_millis(25));
        }
    }

    pub fn stable_screen(&self) -> TerminalSnapshot {
        let fingerprint = |screen: &TerminalSnapshot| {
            let mut bytes = serde_json::to_vec(screen.cells()).expect("serialize cells");
            let cursor = screen.cursor();
            bytes.extend_from_slice(&cursor.col().to_le_bytes());
            bytes.extend_from_slice(&cursor.row().to_le_bytes());
            bytes.push(u8::from(cursor.visible()));
            bytes
        };
        let deadline = Instant::now() + Duration::from_millis(5000);
        let mut screen = self.screen();
        let mut previous = fingerprint(&screen);
        let mut stable_since = Instant::now();
        loop {
            std::thread::sleep(Duration::from_millis(100));
            let candidate = self.screen();
            let current = fingerprint(&candidate);
            if current != previous {
                stable_since = Instant::now();
            }
            screen = candidate;
            previous = current;
            if stable_since.elapsed() >= Duration::from_millis(700) || Instant::now() >= deadline {
                break;
            }
        }
        screen
    }

    pub fn visual(&self, test_id: &str) {
        let _ = self.settle();
        let screen = self.stable_screen();

        let sample = tuiscotti_latest::sample::Sample::state(
            test_id,
            screen,
            tuiscotti_latest::sample::Observation::empty(),
        )
        .expect("build Tuiscotti state sample");
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let store = tuiscotti_latest::check::FrozenStore::new(
            &root.join("tests/functional/baselines"),
            &evidence_root().join("artifacts"),
        )
        .expect("open Tuiscotti frozen store");
        let outcome = store
            .check(&sample.to_bytes().expect("serialize Tuiscotti sample"))
            .expect("run Tuiscotti frozen check");
        if !outcome.is_pass() && std::env::var_os("TUISCOTTI_CAPTURE_ONLY").is_some() {
            self.log(format!("capture-only {test_id}: {outcome:?}"));
            return;
        }
        assert!(
            outcome.is_pass(),
            "visual mismatch for {test_id}: {outcome:?}"
        );
        self.log(format!("visual {test_id} pass"));
    }

    pub fn try_sustained(
        &self,
        hold: Duration,
        predicate: impl Fn(&TerminalSnapshot) -> bool,
    ) -> bool {
        if !predicate(&self.screen()) {
            return false;
        }
        let deadline = Instant::now() + hold * 4 + Duration::from_secs(2);
        let mut since: Option<Instant> = None;
        loop {
            let screen = self.screen();
            if predicate(&screen) {
                let start = *since.get_or_insert_with(Instant::now);
                if start.elapsed() >= hold {
                    return true;
                }
            } else {
                since = None;
            }
            if Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    pub fn wait_text(&self, needle: &str) -> TerminalSnapshot {
        self.wait_screen(&format!("visible {needle}"), |screen| {
            text(screen).contains(needle)
        })
    }

    pub fn wait_text_after_animation(&self, needle: &str, animation: Duration) -> TerminalSnapshot {
        self.wait_screen_within(
            &format!("visible {needle}"),
            TIMEOUT + animation,
            |screen| text(screen).contains(needle),
        )
    }

    pub fn wait_absent(&self, needle: &str) -> TerminalSnapshot {
        self.wait_screen(&format!("absent {needle}"), |screen| {
            !text(screen).contains(needle)
        })
    }

    pub fn settle(&self) -> TerminalSnapshot {
        self.fence();
        self.session
            .wait_quiet(&WaitQuietOptions::new(TIMEOUT, Duration::from_millis(150)).unwrap())
            .expect("terminal must settle after the semantic assertion");
        let screen = self.screen();
        self.record("settled", &screen);
        screen
    }

    pub fn point(&self, needle: &str) -> Point {
        find(&self.wait_text(needle), needle)
    }

    pub fn key(&self, key: &str) {
        self.log(format!("key {key}"));
        self.session
            .execute(Operation::Key {
                keys: vec![key.to_string()],
                action: KeyAction::Press,
            })
            .unwrap_or_else(|e| panic!("key {key}: {e}"));
    }

    pub fn type_text(&self, value: &str) {
        self.log(format!("type {} UTF-8 bytes", value.len()));
        self.session
            .write_bytes(value.as_bytes())
            .expect("type through terminal input");
    }

    pub fn paste(&self, value: &str) {
        self.log(format!("paste {} UTF-8 bytes", value.len()));
        self.session
            .paste(value)
            .expect("paste using negotiated terminal mode");
    }

    pub fn advance_clock(&self, milliseconds: u64) {
        self.paste(&TestRequest::advance(milliseconds));
    }

    fn accepts_paste(&self) -> bool {
        let OperationResult::State(state) = self
            .session
            .execute(Operation::State)
            .expect("process state")
        else {
            panic!("missing process state");
        };
        state.exited.is_none() && state.modes.get("bracketed_paste") == Some(&true)
    }

    fn bells(&self) -> u64 {
        let OperationResult::BellCount(count) = self
            .session
            .execute(Operation::GetBellCount)
            .expect("bell count")
        else {
            panic!("missing bell count");
        };
        count
    }

    fn fence(&self) {
        if !self.accepts_paste() {
            return;
        }
        let before = self.bells();
        self.log("fence".to_string());
        self.session
            .paste(TestRequest::FENCE)
            .expect("fence through bracketed paste");
        let deadline = Instant::now() + TIMEOUT;
        while self.bells() == before {
            self.assert_running("waiting for the application to handle the input");
            assert!(
                Instant::now() < deadline,
                "the application did not acknowledge the fence within {TIMEOUT:?} (bells {before} -> {})\n{}",
                self.bells(),
                text(&self.capture())
            );
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    pub fn move_to(&self, point: Point) {
        self.mouse(CheckedMouseAction::Motion {
            x: point.x,
            y: point.y,
        });
    }

    pub fn down(&self, point: Point) {
        self.mouse(CheckedMouseAction::Press {
            x: point.x,
            y: point.y,
            options: MouseOptions::default(),
        });
    }

    pub fn up(&self, point: Point) {
        self.mouse(CheckedMouseAction::Release {
            x: point.x,
            y: point.y,
            options: MouseOptions::default(),
        });
    }

    pub fn drag_to(&self, point: Point) {
        self.mouse(CheckedMouseAction::Drag {
            x: point.x,
            y: point.y,
            options: MouseOptions::default(),
        });
    }

    fn mouse(&self, action: CheckedMouseAction) {
        self.log(format!("mouse {action:?}"));
        self.session
            .mouse_checked(action)
            .expect("application must negotiate SGR mouse input for this action");
    }

    pub fn click(&self, point: Point) {
        self.down(point);
        self.up(point);
    }

    pub fn click_text(&self, needle: &str) {
        self.click(self.point(needle));
    }

    pub fn wheel(&self, point: Point, down: bool, amount: usize) {
        self.move_to(point);

        let bytes = tui_test::input::mouse::scroll(point.x, point.y, !down);
        self.log(format!("wheel {point:?} down={down} amount={amount}"));
        for _ in 0..amount {
            self.session
                .write_bytes(bytes.as_bytes())
                .expect("wheel through application PTY");
        }
    }

    pub fn resize(&self, cols: u16, rows: u16) {
        self.log(format!("resize {cols}x{rows}"));
        self.session
            .execute(Operation::Resize { cols, rows })
            .expect("resize PTY");
    }

    pub fn finish(&self) {
        self.assert_running("before Ctrl+Q");
        self.key("ctrl+q");
        self.session
            .execute(Operation::WaitExit {
                timeout_ms: Some(10_000),
            })
            .expect("application must exit after Ctrl+Q");
        let OperationResult::State(state) =
            self.session.execute(Operation::State).expect("exit state")
        else {
            panic!("missing exit state");
        };
        assert_eq!(state.exited, Some(0), "normal application exit");
        assert_eq!(state.exit_signal, None, "application must not be killed");
        assert_eq!(state.mouse_mode, "none", "mouse tracking restored on exit");
        assert!(state.cursor.visible, "terminal cursor restored on exit");
        for mode in ["alternate_screen", "bracketed_paste"] {
            assert_eq!(
                state.modes.get(mode),
                Some(&false),
                "terminal mode {mode} restored on exit"
            );
        }
        self.log("clean exit 0".to_string());
    }

    fn assert_running(&self, reason: &str) {
        let OperationResult::State(state) = self
            .session
            .execute(Operation::State)
            .expect("process state")
        else {
            panic!("missing process state while {reason}");
        };
        assert!(
            state.exited.is_none(),
            "application exited while {reason}: {:?}\n{}",
            state.exited,
            text(&self.capture())
        );
    }

    fn log(&self, message: String) {
        self.actions.borrow_mut().push(message);
        std::fs::write(
            self.directory.join("actions.txt"),
            self.actions.borrow().join("\n"),
        )
        .expect("persist functional action trace");
    }

    fn record(&self, label: &str, screen: &TerminalSnapshot) {
        let number = self.checkpoint.get();
        self.checkpoint.set(number + 1);
        self.log(format!("checkpoint {number:03}: {label}"));
        let path = self.directory.join(format!("{number:03}"));
        std::fs::write(
            path.with_extension("json"),
            serde_json::to_vec(screen).unwrap(),
        )
        .expect("save terminal cells");
        std::fs::write(path.with_extension("txt"), text(screen)).expect("save terminal text");
    }
}

impl Drop for App {
    fn drop(&mut self) {
        if let Ok(screen) = self.session.capture_snapshot(SnapshotScope::ActiveScreen) {
            let _ = std::fs::write(self.directory.join("last.txt"), text(&screen));
            if let Ok(json) = serde_json::to_vec(&screen) {
                let _ = std::fs::write(self.directory.join("last.json"), json);
            }
        }
        let _ = std::fs::write(
            self.directory.join("actions.txt"),
            self.actions.borrow().join("\n"),
        );
        let _ = self.session.close();
        let _ = std::fs::remove_dir_all(self.directory.join("home"));
        if std::thread::panicking() {
            eprintln!("functional evidence: {}", self.directory.display());
        }
    }
}

pub fn text(screen: &TerminalSnapshot) -> String {
    (0..screen.rows())
        .map(|row| row_text(screen, row))
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn row_text(screen: &TerminalSnapshot, row: u32) -> String {
    screen
        .row(row)
        .expect("row in terminal")
        .iter()
        .map(SnapshotCell::text)
        .collect()
}

pub fn find_all(screen: &TerminalSnapshot, needle: &str) -> Vec<Point> {
    assert!(!needle.is_empty(), "locator must not be empty");
    let mut matches = Vec::new();
    for row in 0..screen.rows() {
        let line = row_text(screen, row);
        for (byte, _) in line.match_indices(needle) {
            let mut offset = 0;
            for (column, cell) in screen.row(row).unwrap().iter().enumerate() {
                let next = offset + cell.text().len();
                if byte >= offset && byte < next {
                    matches.push(Point {
                        x: u16::try_from(column).unwrap(),
                        y: u16::try_from(row).unwrap(),
                    });
                    break;
                }
                offset = next;
            }
        }
    }
    matches
}

pub fn find(screen: &TerminalSnapshot, needle: &str) -> Point {
    let matches = find_all(screen, needle);
    assert_eq!(
        matches.len(),
        1,
        "expected one physical-row match for {needle:?}, got {matches:?}\n{}",
        text(screen)
    );
    matches[0]
}

pub fn cell_bg(screen: &TerminalSnapshot, point: Point) -> tui_test::SnapshotColor {
    screen
        .cell_at(u32::from(point.x), u32::from(point.y))
        .expect("cell within terminal")
        .bg()
}

pub fn crop(screen: &TerminalSnapshot, point: Point, width: u16, height: u16) -> Vec<SnapshotCell> {
    (point.y..point.y + height)
        .flat_map(|y| {
            (point.x..point.x + width).map(move |x| {
                screen
                    .cell_at(u32::from(x), u32::from(y))
                    .expect("crop within terminal")
                    .clone()
            })
        })
        .collect()
}
