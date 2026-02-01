mod app;
mod config;
mod events;
mod icons;
mod launch;
mod sway;
mod ui;

use crossterm::{
    ExecutableCommand,
    cursor::SetCursorStyle,
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use eyre::Result;
use ratatui::{
    Terminal,
    backend::{Backend, CrosstermBackend},
};
use std::{
    io::{self, Write},
    time::{Duration, Instant},
};

use app::{App, Focus, Mode, SinglePaneMode};
use config::{CursorShape, load_launcher_config};

fn main() -> Result<()> {
    color_eyre::install()?;

    let cfg = load_launcher_config();

    let single_pane_mode = if cfg.dmenu {
        SinglePaneMode::Dmenu
    } else {
        SinglePaneMode::DesktopApps
    };

    let start_mode = match cfg.start_mode {
        config::StartMode::Dual => Mode::DualPane,
        config::StartMode::Single => Mode::SinglePane,
    };

    let mut app = App::new(single_pane_mode, start_mode, &cfg);

    let print_only = std::env::args().any(|arg| arg == "--print-selection");
    let sway_mode = std::env::args().any(|arg| arg == "--sway");
    
    let mut sway_client = if sway_mode {
        sway::Client::connect().ok()
    } else {
        None
    };

    let mut fullscreen_window_id = None;
    if let Some(client) = &mut sway_client {
        if let Ok(Some(id)) = client.get_focused_fullscreen_node_id() {
            fullscreen_window_id = Some(id);
            let _ = client.set_fullscreen(false, Some(id));
        }
    }

    enable_raw_mode()?;

    let res = if print_only {
        run_with_writer(io::stderr(), &mut app, &cfg)
    } else {
        run_with_writer(io::stdout(), &mut app, &cfg)
    };

    disable_raw_mode()?;

    if let Err(err) = res {
        eprintln!("Error: {err:?}");
    }

    if let Some(ref cmd) = app.app_to_launch {
        if print_only {
            // Just print the command to stdout - useful for those who wish to pipe to swayexec or similar
            // Check if app needs terminal
            if let Some(entry) = app.apps.iter().find(|a| &a.exec == cmd) {
                if entry.terminal || entry.needs_terminal() {
                    println!("{} {}", app.config.terminal, cmd);
                } else {
                    println!("{}", cmd);
                }
            } else {
                println!("{}", cmd);
            }
        } else {
            // directly launch
            if sway_mode {
                let full_cmd = if let Some(entry) = app.apps.iter().find(|a| &a.exec == cmd).cloned() {
                    app.add_to_recent(entry.name.clone());
                    let command = crate::launch::build_command(&entry, &app.config);
                    // Simple reconstruction of command string for sway exec
                    let prog = command.get_program().to_string_lossy();
                    let args = command.get_args()
                        .map(|a| {
                            let s = a.to_string_lossy();
                            if s.contains(' ') {
                                format!("\"{}\"", s)
                            } else {
                                s.into_owned()
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(" ");
                    format!("{} {}", prog, args)
                } else {
                    cmd.clone()
                };

                if let Some(client) = &mut sway_client {
                    let _ = client.exec(&full_cmd);
                }
            } else {
                if let Some(entry) = app.apps.iter().find(|a| &a.exec == cmd).cloned() {
                    app.add_to_recent(entry.name.clone());
                    crate::launch::launch_app(&entry, &app.config);
                } else {
                    let _ = std::process::Command::new("sh").arg("-c").arg(cmd).spawn();
                }
            }
        }
    } else {
        // User cancelled
        if let Some(id) = fullscreen_window_id {
            if let Some(client) = &mut sway_client {
                let _ = client.set_fullscreen(true, Some(id));
            }
        }
    }

    Ok(())
}

fn run_with_writer<W: Write + ExecutableCommand>(
    mut writer: W,
    app: &mut App,
    cfg: &config::DstlConfig,
) -> Result<()> {
    set_cursor_color(&mut writer, &cfg.colors.cursor_color)?;

    execute!(writer, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(writer);
    let mut terminal = Terminal::new(backend)?;

    warmup_icons(&mut terminal, app, cfg)?;

    if app.mode == Mode::DualPane && !app.categories.is_empty() {
        let old_focus = app.focus;
        app.focus = Focus::Categories;
        terminal.draw(|f| ui::draw(f, app, cfg.search_position.clone(), cfg))?;
        app.focus = old_focus;
    }

    let res = run_app(&mut terminal, app, cfg);

    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    // Reset cursor color to default
    reset_cursor_color(terminal.backend_mut())?;

    res
}

/// Set the cursor color using ANSI escape codes
fn set_cursor_color<W: Write>(writer: &mut W, color_hex: &str) -> Result<()> {
    if let Some((r, g, b)) = parse_hex_color(color_hex) {
        // OSC 12 ; color ST - Set cursor color
        write!(writer, "\x1b]12;rgb:{:02x}/{:02x}/{:02x}\x07", r, g, b)?;
        writer.flush()?;
    }
    Ok(())
}

/// Reset cursor color to terminal default
fn reset_cursor_color<W: Write>(writer: &mut W) -> Result<()> {
    // OSC 112 ST - Reset cursor color
    write!(writer, "\x1b]112\x07")?;
    writer.flush()?;
    Ok(())
}

/// Parse hex color string to RGB values
fn parse_hex_color(color: &str) -> Option<(u8, u8, u8)> {
    let color = color.trim();

    if !color.starts_with('#') {
        return None;
    }

    let hex = &color[1..];

    match hex.len() {
        // #RGB format
        3 => {
            let r = u8::from_str_radix(&hex[0..1], 16).ok()?;
            let g = u8::from_str_radix(&hex[1..2], 16).ok()?;
            let b = u8::from_str_radix(&hex[2..3], 16).ok()?;
            Some((r * 17, g * 17, b * 17))
        }
        // #RRGGBB format
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some((r, g, b))
        }
        // #RRGGBBAA format (ignore alpha)
        8 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some((r, g, b))
        }
        _ => None,
    }
}

fn run_app<B: Backend + ExecutableCommand>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    cfg: &config::DstlConfig,
) -> Result<()> {
    let mut last_input = Instant::now();

    loop {
        app.update_cursor_blink();

        terminal.draw(|f| ui::draw(f, app, cfg.search_position.clone(), cfg))?;

        // Always show cursor (input always active)
        // Set shape based on blink interval
        let style = if cfg.colors.cursor_blink_interval > 0 {
            // Use steady cursor - we'll handle blinking manually
            match cfg.colors.cursor_shape {
                CursorShape::Block => SetCursorStyle::SteadyBlock,
                CursorShape::Underline => SetCursorStyle::SteadyUnderScore,
                CursorShape::Pipe => SetCursorStyle::SteadyBar,
            }
        } else {
            // Use terminal's built-in blinking
            match cfg.colors.cursor_shape {
                CursorShape::Block => SetCursorStyle::BlinkingBlock,
                CursorShape::Underline => SetCursorStyle::BlinkingUnderScore,
                CursorShape::Pipe => SetCursorStyle::BlinkingBar,
            }
        };
        
        terminal.backend_mut().execute(style)?;

        // Handle manual cursor blinking if interval is set
        if cfg.colors.cursor_blink_interval > 0 {
            if app.cursor_visible {
                terminal.show_cursor()?;
            } else {
                terminal.hide_cursor()?;
            }
        } else {
            terminal.show_cursor()?;
        }

        let tick = Duration::from_millis(50);

        if cfg.timeout > 0 && last_input.elapsed().as_secs() >= cfg.timeout {
            break;
        }

        if event::poll(tick)? {
            if let Event::Key(key) = event::read()? {
                last_input = Instant::now();
                if events::handle_key(app, key)? {
                    break;
                }
            }
        }
    }

    Ok(())
}

fn warmup_icons<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &App,
    cfg: &config::DstlConfig,
) -> Result<()> {
    if app.categories.is_empty() {
        return Ok(());
    }

    let mut tmp = app.clone();
    tmp.focus = Focus::Apps;
    terminal.draw(|f| ui::draw(f, &mut tmp, cfg.search_position.clone(), cfg))?;

    if app.mode == Mode::DualPane {
        tmp.focus = Focus::Categories;
        terminal.draw(|f| ui::draw(f, &mut tmp, cfg.search_position.clone(), cfg))?;
    }

    Ok(())
}
