use anyhow::Result;
use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use std::io;

use std::time::{Duration, Instant};

use crate::app::App;

pub struct EventHandler {
    pub tick_rate: Duration,
    pub last_tick: Instant,
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> Self {
        Self {
            tick_rate,
            last_tick: Instant::now(),
        }
    }

    pub fn next(&mut self) -> Result<Option<Event>> {
        let timeout = self.tick_rate.checked_sub(self.last_tick.elapsed()).unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            let event = event::read()?;
            self.last_tick = Instant::now();
            Ok(Some(event))
        } else {
            Ok(None)
        }
    }
}

pub fn setup_terminal() -> Result<ratatui::Terminal<ratatui::backend::CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let terminal = ratatui::Terminal::new(backend)?;
    Ok(terminal)
}

pub fn restore_terminal(
    terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<io::Stdout>>,
) -> Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

pub fn handle_events(app: &mut App, event: Event) -> Result<bool> {
    match event {
        Event::Key(key_event) => handle_key_event(app, key_event)?,
        Event::Mouse(_) => {}
        Event::Resize(_, _) => {}
        Event::FocusGained | Event::FocusLost | Event::Paste(_) => {}
    }
    Ok(true)
}

fn handle_key_event(app: &mut App, key_event: KeyEvent) -> Result<()> {
    match key_event.code {
        KeyCode::Char('q') | KeyCode::Char('Q') => {
            return Err(anyhow::anyhow!("Quit"));
        }
        KeyCode::Char(' ') => {
            match app.playback_state {
                crate::app::PlaybackState::Playing => app.pause(),
                crate::app::PlaybackState::Paused => app.resume(),
                crate::app::PlaybackState::Stopped => {
                    app.play()?;
                }
            }
        }
        KeyCode::Char('n') | KeyCode::Char('N') => {
            app.next()?;
        }
        KeyCode::Char('p') | KeyCode::Char('P') => {
            app.previous()?;
        }
        KeyCode::Char('s') | KeyCode::Char('S') => {
            app.toggle_shuffle();
        }
        KeyCode::Char('+') | KeyCode::Char('=') => {
            let new_volume = (app.volume + 0.1).min(1.0);
            app.set_volume(new_volume);
        }
        KeyCode::Char('-') => {
            let new_volume = (app.volume - 0.1).max(0.0);
            app.set_volume(new_volume);
        }
        KeyCode::Char('0') => {
            app.set_volume(0.0);
        }
        KeyCode::Char('1') => {
            app.set_volume(0.1);
        }
        KeyCode::Char('2') => {
            app.set_volume(0.2);
        }
        KeyCode::Char('3') => {
            app.set_volume(0.3);
        }
        KeyCode::Char('4') => {
            app.set_volume(0.4);
        }
        KeyCode::Char('5') => {
            app.set_volume(0.5);
        }
        KeyCode::Char('6') => {
            app.set_volume(0.6);
        }
        KeyCode::Char('7') => {
            app.set_volume(0.7);
        }
        KeyCode::Char('8') => {
            app.set_volume(0.8);
        }
        KeyCode::Char('9') => {
            app.set_volume(0.9);
        }
        KeyCode::Char('m') | KeyCode::Char('M') => {
            app.set_volume(1.0);
        }
        KeyCode::Right => {
            app.next()?;
        }
        KeyCode::Left => {
            app.previous()?;
        }
        KeyCode::Up => {
            let new_volume = (app.volume + 0.05).min(1.0);
            app.set_volume(new_volume);
        }
        KeyCode::Down => {
            let new_volume = (app.volume - 0.05).max(0.0);
            app.set_volume(new_volume);
        }
        _ => {}
    }
    Ok(())
} 