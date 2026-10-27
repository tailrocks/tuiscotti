//! First-success example: render a production-style dashboard view and save it as a versioned sample.

use std::path::PathBuf;

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use tuiscotti::render_profile::{PROFILE_NAME, platform_key, resolve_profile};
use tuiscotti::sample::{Observation, Sample};
use tuiscotti::view::{ViewSettings, capture_view};

#[derive(Debug, Clone, PartialEq, Eq)]
struct DeployEntry {
    name: String,
    healthy: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Dashboard {
    title: String,
    entries: Vec<DeployEntry>,
}

fn dashboard() -> Dashboard {
    Dashboard {
        title: "deploys".to_string(),
        entries: vec![
            DeployEntry {
                name: "api-east".to_string(),
                healthy: true,
            },
            DeployEntry {
                name: "api-west".to_string(),
                healthy: false,
            },
        ],
    }
}

fn draw_dashboard(frame: &mut Frame<'_>, dashboard: &Dashboard) {
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(frame.area());
    frame.render_widget(
        Paragraph::new(dashboard.title.clone()).block(Block::default().borders(Borders::ALL)),
        areas[0],
    );
    let items: Vec<ListItem<'_>> = dashboard
        .entries
        .iter()
        .map(|entry| {
            let style = if entry.healthy {
                Style::default()
            } else {
                Style::default().add_modifier(Modifier::BOLD)
            };
            ListItem::new(entry.name.clone()).style(style)
        })
        .collect();
    frame.render_widget(
        List::new(items).block(Block::default().borders(Borders::ALL)),
        areas[1],
    );
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut out = PathBuf::from("hello.tui.json");
    let mut pixels = false;
    for arg in std::env::args().skip(1) {
        if arg == "--pixels" {
            pixels = true;
        } else {
            out = PathBuf::from(arg);
        }
    }
    let dashboard = dashboard();
    let snapshot = capture_view(80, 24, &ViewSettings::builtin(), |frame| {
        draw_dashboard(frame, &dashboard);
    })?;
    let bytes = if pixels {
        let profile = resolve_profile(PROFILE_NAME)?;
        let test_id = format!("tutorial/hello-dashboard/pixels-{}", platform_key());
        Sample::with_pixels(
            &test_id,
            &snapshot,
            Observation::noted("hello example"),
            profile,
        )?
        .to_bytes()?
    } else {
        Sample::state(
            "tutorial/hello-dashboard",
            snapshot,
            Observation::noted("hello example"),
        )?
        .to_bytes()?
    };
    std::fs::write(&out, &bytes)?;
    println!("wrote {} ({} bytes)", out.display(), bytes.len());
    Ok(())
}
