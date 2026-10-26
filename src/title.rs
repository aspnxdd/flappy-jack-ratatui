//! Attract and game-over screens share the live 3D hero, not a recorded image.
use ratatui::{
    Frame,
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
};

use crate::pumpkin::{self, BACKGROUND};

const ORANGE: Color = Color::Rgb(246, 153, 67);
const CREAM: Color = Color::Rgb(246, 220, 164);
const MUTED: Color = Color::Rgb(153, 137, 175);
const PURPLE: Color = Color::Rgb(92, 66, 124);

fn centered(frame: &mut Frame, y: u16, line: Line<'_>) {
    let area = frame.area();
    if y < area.height {
        frame.render_widget(
            line.centered(),
            Rect::new(area.x, area.y + y, area.width, 1),
        );
    }
}

fn letter(c: char) -> [u8; 7] {
    match c {
        'F' => [31, 16, 16, 30, 16, 16, 16],
        'L' => [16, 16, 16, 16, 16, 16, 31],
        'A' => [14, 17, 17, 31, 17, 17, 17],
        'P' => [30, 17, 17, 30, 16, 16, 16],
        'Y' => [17, 17, 10, 4, 4, 4, 4],
        'J' => [7, 2, 2, 2, 18, 18, 12],
        'C' => [14, 17, 16, 16, 16, 17, 14],
        'K' => [17, 18, 20, 24, 20, 18, 17],
        _ => [0; 7],
    }
}

fn wordmark(buf: &mut Buffer, area: Rect) {
    let width = 63;
    if area.width < width || area.height < 5 {
        return;
    }
    let mut x = area.x + (area.width - width) / 2;
    for c in "FLAPPY JACK".chars() {
        if c == ' ' {
            x += 4;
            continue;
        }
        let rows = letter(c);
        for y in 0..4 {
            for col in 0..5 {
                let top = rows[y * 2] & (1 << (4 - col)) != 0;
                let bottom = y * 2 + 1 < 7 && rows[y * 2 + 1] & (1 << (4 - col)) != 0;
                let color = Color::Rgb(244 - y as u8 * 13, 162 - y as u8 * 20, 86 - y as u8 * 15);
                buf[(x + col, area.y + 1 + y as u16)]
                    .set_symbol("▀")
                    .set_fg(if top { color } else { BACKGROUND })
                    .set_bg(if bottom { color } else { BACKGROUND });
            }
        }
        x += 6;
    }
}

fn moon(buf: &mut Buffer, area: Rect) {
    if area.width < 65 || area.height < 20 {
        return;
    }
    let x0 = area.right() - 16;
    let y0 = area.y + 7;
    for y in 0..5 {
        for x in 0..10 {
            let sample = |py: u16| {
                let dx = x as f32 - 4.5;
                let dy = py as f32 - 4.5;
                if dx * dx + dy * dy > 20.0 {
                    return None;
                }
                let crater = ((dx + 1.5).powi(2) + (dy + 1.8).powi(2) < 2.4)
                    || ((dx - 1.0).powi(2) + (dy - 2.0).powi(2) < 1.5);
                let shade = if (dx - 3.5).powi(2) + (dy + 1.7).powi(2) < 23.0 {
                    0.29
                } else if crater {
                    0.67
                } else {
                    0.86
                };
                Some(Color::Rgb(
                    (164.0 * shade) as u8,
                    (157.0 * shade) as u8,
                    (183.0 * shade) as u8,
                ))
            };
            let cell = &mut buf[(x0 + x, y0 + y)];
            let top = sample(y * 2);
            let bottom = sample(y * 2 + 1);
            if top.is_some() || bottom.is_some() {
                let old_top = if cell.symbol() == "▀" {
                    cell.fg
                } else {
                    cell.bg
                };
                let old_bottom = cell.bg;
                cell.set_symbol("▀")
                    .set_fg(top.unwrap_or(old_top))
                    .set_bg(bottom.unwrap_or(old_bottom));
            }
        }
    }
}

fn ghost(buf: &mut Buffer, area: Rect, x: u16, y: u16, seconds: f32, phase: f32) {
    let float = ((seconds * 0.9 + phase).sin() + 1.0) as u16;
    let color = Color::Rgb(61, 49, 82);
    for (dy, line) in [" .-. ", "(o o)", "|   |", "'~'~'"].iter().enumerate() {
        let row = y + dy as u16 + float;
        if x + 5 <= area.width && row < area.height {
            buf.set_string(area.x + x, area.y + row, line, color);
        }
    }
}

/// Title when `result` is `None`; otherwise a score card with immediate restart help.
pub fn draw(frame: &mut Frame, seconds: f32, best: u32, result: Option<u32>, demo: bool) {
    let area = frame.area();
    if area.is_empty() {
        return;
    }
    let seconds = if seconds.is_finite() { seconds } else { 0.0 };
    for cell in &mut frame.buffer_mut().content {
        cell.set_symbol(" ")
            .set_style(Style::new().bg(BACKGROUND).fg(MUTED));
    }
    if area.height < 16 || area.width < 65 {
        centered(
            frame,
            0,
            Line::styled("FLAPPY JACK", Style::new().fg(ORANGE).bold()),
        );
        centered(
            frame,
            area.height.saturating_sub(2),
            Line::styled("Press Space to awaken.", CREAM),
        );
        return;
    }

    wordmark(frame.buffer_mut(), area);
    centered(
        frame,
        5,
        Line::styled(
            if result.is_some() {
                "FLAPPY JACK  /  THE CANDLE WENT OUT"
            } else {
                "FLAPPY JACK  /  A MIDNIGHT FLIGHT"
            },
            MUTED,
        ),
    );
    let hero_width = area.width.min(92);
    let hero = Rect::new(
        area.x + (area.width - hero_width) / 2,
        area.y + 6,
        hero_width,
        area.height - 12,
    );
    pumpkin::render(frame.buffer_mut(), hero, seconds);
    moon(frame.buffer_mut(), area);
    ghost(frame.buffer_mut(), area, 6, area.height / 2, seconds, 0.0);
    ghost(
        frame.buffer_mut(),
        area,
        area.width - 12,
        area.height / 2 + 3,
        seconds,
        2.2,
    );

    let strong = Style::new().fg(CREAM).bold();
    let lines = match result {
        Some(score) => [
            Line::from(vec![
                Span::styled("SCORE ", MUTED),
                Span::styled(score.to_string(), strong),
                Span::styled("      BEST ", MUTED),
                Span::styled(best.max(score).to_string(), Style::new().fg(ORANGE).bold()),
            ]),
            Line::styled("Space/R restart", strong),
            Line::styled("Q quit", MUTED),
        ],
        None => [
            Line::styled("Press Space to awaken.", strong),
            Line::from(vec![
                Span::styled("BEST ", MUTED),
                Span::styled(best.to_string(), ORANGE),
                Span::styled("    •    THREAD THE GATES. STEAL THE SWEETS.", MUTED),
            ]),
            Line::styled(
                if demo {
                    "DEMO  •  a restless spirit takes the wheel    Q quit"
                } else {
                    "SPACE flap     P / ESC pause     Q quit"
                },
                if demo { ORANGE } else { PURPLE },
            ),
        ],
    };
    for (offset, line) in [5, 3, 1].into_iter().zip(lines) {
        centered(frame, area.height - offset, line);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};

    fn screen(width: u16, height: u16, result: Option<u32>, demo: bool) -> Buffer {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|frame| draw(frame, 0.0, 12, result, demo))
            .unwrap();
        terminal.backend().buffer().clone()
    }

    fn text(buf: &Buffer) -> String {
        buf.content.iter().map(|cell| cell.symbol()).collect()
    }

    #[test]
    fn title_and_game_over_keep_actions_and_scores_visible() {
        for (width, height) in [(80, 28), (100, 30), (140, 40)] {
            let title = screen(width, height, None, false);
            let contents = text(&title);
            assert!(contents.contains("FLAPPY JACK"));
            assert!(contents.contains("Press Space to awaken"));
            assert!(contents.contains("BEST 12"));
            assert!(contents.contains("Q quit"));
            let hero_orange = title
                .content
                .iter()
                .filter(
                    |cell| matches!(cell.fg, Color::Rgb(r, g, b) if r > 70 && g < r / 2 && b < 50),
                )
                .count();
            assert!(hero_orange > 100, "live hero is present at t=0");
            let game_over = text(&screen(width, height, Some(17), false));
            assert!(game_over.contains("SCORE 17"));
            assert!(game_over.contains("BEST 17"));
            assert!(game_over.contains("Space/R restart"));
        }
    }

    #[test]
    fn demo_is_labeled_and_tiny_transient_sizes_do_not_panic() {
        assert!(text(&screen(100, 30, None, true)).contains("DEMO"));
        for (width, height) in [(1, 1), (40, 12), (65, 16)] {
            screen(width, height, None, false);
        }
    }
}
