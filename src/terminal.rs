use std::{
    io::{self, stderr, stdout},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

use ratatui::{
    Frame,
    crossterm::{
        cursor::Show,
        event::{
            self, Event, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags,
            PushKeyboardEnhancementFlags,
        },
        execute,
        style::ResetColor,
        terminal::LeaveAlternateScreen,
    },
};

use crate::app::App;

pub const MIN_WIDTH: u16 = 80;
pub const MIN_HEIGHT: u16 = 28;
const FRAME_INTERVAL: Duration = Duration::from_nanos(1_000_000_000 / 30);
const TICK_INTERVAL: Duration = Duration::from_nanos(1_000_000_000 / 120 + 1);
const MAX_EVENTS: usize = 64;

static KEYBOARD_PUSHED: AtomicBool = AtomicBool::new(false);

fn restore_keyboard() {
    if KEYBOARD_PUSHED.swap(false, Ordering::Relaxed)
        && execute!(stdout(), PopKeyboardEnhancementFlags).is_err()
    {
        let _ = execute!(stderr(), PopKeyboardEnhancementFlags);
    }
}

struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        restore_keyboard();
        let restored = ratatui::try_restore();
        let cursor = execute!(stdout(), Show, ResetColor);
        if restored.is_err() || cursor.is_err() {
            // A broken stdout must not prevent best-effort cleanup through the other TTY stream.
            let _ = execute!(stderr(), LeaveAlternateScreen, Show, ResetColor);
        }
    }
}

pub fn run(app: &mut App, mut draw: impl FnMut(&mut Frame, &App)) -> io::Result<()> {
    // Arm before try_init: an initialization error can leave raw mode/alternate screen active.
    let _guard = TerminalGuard;
    let mut terminal = ratatui::try_init()?;
    let flags = KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
        | KeyboardEnhancementFlags::REPORT_EVENT_TYPES
        | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES;
    // Unsupported terminals ignore this; legacy Windows reports Unsupported and keeps native events.
    if execute!(stdout(), PushKeyboardEnhancementFlags(flags)).is_ok() {
        KEYBOARD_PUSHED.store(true, Ordering::Relaxed);
        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            restore_keyboard();
            hook(info);
        }));
    }
    let size = terminal.size()?;
    app.advance(Duration::ZERO, too_small(size.width, size.height));
    let mut previous = Instant::now();
    let mut next_frame = previous;

    while !app.quit {
        let now = Instant::now();
        app.advance(now.saturating_duration_since(previous), app.too_small);
        previous = now;
        if now >= next_frame {
            next_frame = now + FRAME_INTERVAL;
            terminal.draw(|frame| {
                let area = frame.area();
                let small = too_small(area.width, area.height);
                if app.too_small != small {
                    app.advance(Duration::ZERO, small);
                    previous = Instant::now();
                }
                draw(frame, app);
            })?;
        }
        let wait = next_frame
            .saturating_duration_since(Instant::now())
            .min(TICK_INTERVAL);
        if event::poll(wait)? {
            // Bound draining so a key flood never starves simulation or redraws.
            for _ in 0..MAX_EVENTS {
                match event::read()? {
                    Event::Key(key) => app.key(key),
                    Event::Resize(width, height) => {
                        app.advance(Duration::ZERO, too_small(width, height));
                        previous = Instant::now();
                        next_frame = previous;
                    }
                    _ => {}
                }
                if app.quit || !event::poll(Duration::ZERO)? {
                    break;
                }
            }
        }
    }
    Ok(())
}

fn too_small(width: u16, height: u16) -> bool {
    width < MIN_WIDTH || height < MIN_HEIGHT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimum_size_includes_the_boundary_and_zero_is_safe() {
        assert!(!too_small(80, 28));
        assert!(!too_small(120, 40));
        assert!(too_small(79, 28));
        assert!(too_small(80, 27));
        assert!(too_small(0, 0));
    }

    #[test]
    fn simulation_deadline_is_faster_than_rendering_and_both_block() {
        assert!(!TICK_INTERVAL.is_zero());
        assert!(!FRAME_INTERVAL.is_zero());
        assert!(TICK_INTERVAL < FRAME_INTERVAL);
        assert!((FRAME_INTERVAL.as_secs_f64() - 1.0 / 30.0).abs() < 1e-8);
        assert!((TICK_INTERVAL.as_secs_f64() - 1.0 / 120.0).abs() < 1e-8);
    }
}
