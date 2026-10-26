use ratatui::{
    Frame,
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph},
};

use crate::{
    app::{App, Phase},
    game::{self, Game},
    pumpkin::BACKGROUND as NIGHT,
    terminal::{MIN_HEIGHT, MIN_WIDTH},
    title,
};

const PURPLE: Color = Color::Rgb(112, 83, 142);
const MUTED: Color = Color::Rgb(163, 148, 177);
const ORANGE: Color = Color::Rgb(241, 126, 28);
const CANDLE: Color = Color::Rgb(255, 222, 113);

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();
    frame.render_widget(Block::default().style(Style::new().bg(NIGHT)), area);
    match app.phase {
        Phase::Title => title::draw(frame, app.seconds, app.best, None, app.demo),
        Phase::GameOver if app.game.death_time >= 0.35 => {
            title::draw(
                frame,
                app.game.death_time,
                app.best,
                Some(app.game.score),
                app.demo,
            );
        }
        _ => playground(frame, app),
    }
    if app.too_small {
        overlay(
            frame,
            " A little more moonlight ",
            &format!(
                "Resize to continue\nAt least {MIN_WIDTH} × {MIN_HEIGHT} · now {} × {}\nYour flight is frozen.  Q quits.",
                area.width, area.height
            ),
        );
    } else if app.phase == Phase::Paused {
        overlay(
            frame,
            " PAUSED ",
            "Even pumpkins need a breath.\n\nP / Escape to resume · Q to quit",
        );
    }
}

fn playground(frame: &mut Frame, app: &App) {
    let screen = frame.area();
    if screen.width < 8 || screen.height < 8 {
        return;
    }
    let area = flight_area(screen);
    let game = &app.game;
    let hud = Line::from(vec![
        Span::styled(" FLAPPY JACK ", Style::new().fg(ORANGE).bold()),
        Span::styled(
            format!("   SCORE {:03}", game.score),
            Style::new().fg(CANDLE).bold(),
        ),
        Span::styled(format!("   BEST {:03}", app.best), MUTED),
        Span::styled(if app.demo { "   • DEMO" } else { "" }, PURPLE),
    ]);
    frame.render_widget(hud, Rect::new(area.x, area.y - 2, area.width, 1));
    let buf = frame.buffer_mut();
    cemetery(buf, area, game.time);
    for gate in &game.gates {
        let left = world_x(area, gate.x);
        let right = world_x(area, gate.x + game::GATE_WIDTH);
        let gap_top = world_y(area, gate.gap_y - gate.gap_half);
        let gap_bottom = world_y(area, gate.gap_y + gate.gap_half);
        for y in i32::from(area.top())..i32::from(area.bottom()) {
            if y > gap_top && y < gap_bottom {
                continue;
            }
            let color = if gate.passed {
                Color::Rgb(62, 69, 70)
            } else {
                Color::Rgb(108, 95, 127)
            };
            let tip = y == gap_top || y == gap_bottom;
            for x in left..=right {
                if x < i32::from(area.left()) || x >= i32::from(area.right()) {
                    continue;
                }
                let edge = x == left || x == right;
                let rail = (x - left) % 2 == (y / 4) % 2;
                let glyph = if tip {
                    if y == gap_top { '▼' } else { '▲' }
                } else if y % 4 == 0 {
                    '═'
                } else if edge {
                    '║'
                } else if rail {
                    '│'
                } else {
                    ' '
                };
                put(buf, x, y, glyph, color);
            }
        }
    }
    for candy in &game.candies {
        let x = world_x(area, candy.x);
        let y = world_y(area, candy.y);
        put(buf, x - 1, y, '›', PURPLE);
        put(buf, x, y, '◆', CANDLE);
        put(buf, x + 1, y, '‹', PURPLE);
    }
    let x = world_x(area, game::PLAYER_X);
    let y = world_y(area, game.y);
    if game.alive {
        little_jack(buf, x, y, game);
        if game.time < 1.0 {
            let hint = "Space to flap • find your rhythm";
            buf.set_string(
                area.x + area.width.saturating_sub(hint.len() as u16) / 2,
                area.y + 1,
                hint,
                MUTED,
            );
        }
    } else {
        for i in 0..15 {
            let angle = i as f32 * 2.4;
            let t = game.death_time;
            let dx = angle.cos() * t * (10.0 + (i % 3) as f32 * 4.0);
            let dy = angle.sin() * t * 9.0 + 18.0 * t * t;
            put(
                buf,
                x + dx as i32,
                y + dy as i32,
                if i % 3 == 0 { '·' } else { '•' },
                if i % 2 == 0 { CANDLE } else { ORANGE },
            );
        }
    }
    let ground = area.bottom() + 1;
    for x in area.left() - 2..area.right() + 2 {
        put(
            buf,
            i32::from(x),
            i32::from(ground),
            if x % 4 == 0 { '┴' } else { '─' },
            Color::Rgb(66, 54, 71),
        );
    }
    frame.render_widget(
        Line::from(vec![
            Span::styled("SPACE", CANDLE),
            Span::raw(" flap   "),
            Span::styled("P / ESC", CANDLE),
            Span::raw(" pause   "),
            Span::styled("Q", CANDLE),
            Span::raw(" quit    Gate +1 · Candy ◆ +3"),
        ])
        .centered()
        .style(MUTED),
        Rect::new(area.x - 2, area.bottom() + 2, area.width + 4, 1),
    );
}

fn cemetery(buf: &mut Buffer, area: Rect, time: f32) {
    for y in area.top()..area.bottom() {
        let low = f32::from(y - area.y) / f32::from(area.height.max(1));
        for x in area.left()..area.right() {
            buf[(x, y)].set_bg(Color::Rgb(
                8 + (low * 4.0) as u8,
                7 + (low * 3.0) as u8,
                17 + (low * 6.0) as u8,
            ));
        }
    }
    for i in 0..29 {
        let x = (i * 37 + 11) % u32::from(area.width);
        let y = (i * 11 + 3) % u32::from((area.height * 2 / 3).max(1));
        put(
            buf,
            i32::from(area.x) + x as i32,
            i32::from(area.y) + y as i32,
            if i % 5 == 0 { '✦' } else { '·' },
            Color::Rgb(57, 46, 73),
        );
    }
    let mx = i32::from(area.x + area.width * 4 / 5);
    for (dy, line) in [" .-─. ", "(  ◜ )", " `──' "].iter().enumerate() {
        for (dx, c) in line.chars().enumerate() {
            put(
                buf,
                mx + dx as i32,
                i32::from(area.y) + dy as i32,
                c,
                Color::Rgb(138, 130, 151),
            );
        }
    }
    let floor = i32::from(area.bottom()) - 1;
    for i in 0..12 {
        let pos = (i as f32 * 11.0 - time * 1.8).rem_euclid(f32::from(area.width) + 12.0) - 6.0;
        let x = i32::from(area.x) + pos as i32;
        let tall = i % 3 == 0;
        let lines: &[&str] = if tall {
            &["  ┬  ", " ─┼─ ", "  │  ", " ▄█▄ "]
        } else {
            &[" ╭─╮ ", " │†│ ", "▄███▄"]
        };
        for (dy, line) in lines.iter().enumerate() {
            for (dx, c) in line.chars().enumerate() {
                if c != ' '
                    && x + dx as i32 >= i32::from(area.left())
                    && x + (dx as i32) < i32::from(area.right())
                {
                    put(
                        buf,
                        x + dx as i32,
                        floor - lines.len() as i32 + dy as i32,
                        c,
                        Color::Rgb(37, 31, 48),
                    );
                }
            }
        }
    }
    // Background ghosts stay dim so they never look like obstacles.
    for i in 0..2 {
        let x = (f32::from(area.width) - time * (1.4 + i as f32) + i as f32 * 41.0)
            .rem_euclid(f32::from(area.width));
        let y = 5.0 + i as f32 * 6.0 + (time * 0.8 + i as f32).sin();
        for (dx, c) in "(··)".chars().enumerate() {
            put(
                buf,
                i32::from(area.x) + x as i32 + dx as i32,
                i32::from(area.y) + y as i32,
                c,
                Color::Rgb(48, 42, 61),
            );
        }
    }
}

fn little_jack(buf: &mut Buffer, x: i32, y: i32, game: &Game) {
    let flapping = game.flap_time < 0.16;
    let leaf_y = y - if flapping { 3 } else { 2 };
    put(
        buf,
        x - 3,
        leaf_y,
        if flapping { '╲' } else { '╱' },
        Color::Rgb(125, 159, 71),
    );
    put(
        buf,
        x + 3,
        leaf_y,
        if flapping { '╱' } else { '╲' },
        Color::Rgb(125, 159, 71),
    );
    put(buf, x - 4, leaf_y, '❧', Color::Rgb(125, 159, 71));
    put(buf, x + 4, leaf_y, '☙', Color::Rgb(125, 159, 71));
    put(buf, x, y - 2, '╹', Color::Rgb(113, 127, 58));
    for (i, c) in "▄███▄".chars().enumerate() {
        put(buf, x - 2 + i as i32, y - 1, c, ORANGE);
    }
    let worried = game.danger() > 0.45;
    let face = if worried {
        "◖◉█◉◗"
    } else {
        "◖▲█▲◗"
    };
    for (i, c) in face.chars().enumerate() {
        put(
            buf,
            x - 2 + i as i32,
            y,
            c,
            if i == 1 || i == 3 {
                Style::new().fg(CANDLE).bg(Color::Rgb(198, 77, 13))
            } else {
                Style::new().fg(ORANGE)
            },
        );
    }
    let mouth = if worried {
        "╰─o─╯"
    } else {
        "╰v^v╯"
    };
    for (i, c) in mouth.chars().enumerate() {
        put(
            buf,
            x - 2 + i as i32,
            y + 1,
            c,
            if i == 0 || i == 4 { ORANGE } else { CANDLE },
        );
    }
    if flapping {
        put(buf, x - 4, y + 1, '·', Color::Rgb(123, 81, 43));
    }
}

fn flight_area(screen: Rect) -> Rect {
    // Keep the visible core larger than its hitbox, even in a very tall terminal.
    let width = screen.width.saturating_sub(4).min(game::WIDTH as u16);
    let height = screen.height.saturating_sub(7).min(game::HEIGHT as u16);
    Rect::new(
        screen.x + (screen.width - width) / 2,
        screen.y + (screen.height - height) / 2,
        width,
        height,
    )
}

fn world_x(area: Rect, x: f32) -> i32 {
    i32::from(area.x) + (x / game::WIDTH * f32::from(area.width)).round() as i32
}
fn world_y(area: Rect, y: f32) -> i32 {
    i32::from(area.y) + (y / game::HEIGHT * f32::from(area.height.saturating_sub(1))).round() as i32
}
fn put(buf: &mut Buffer, x: i32, y: i32, glyph: char, style: impl Into<Style>) {
    if x < i32::from(buf.area.left())
        || y < i32::from(buf.area.top())
        || x >= i32::from(buf.area.right())
        || y >= i32::from(buf.area.bottom())
    {
        return;
    }
    buf[(x as u16, y as u16)].set_char(glyph).set_style(style);
}

fn overlay(frame: &mut Frame, title: &str, message: &str) {
    let area = frame.area();
    let width = area.width.min(60);
    let height = area.height.min(7);
    let popup = Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    );
    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(message)
            .centered()
            .style(Style::new().fg(CANDLE).bg(NIGHT))
            .block(
                Block::bordered()
                    .title(title)
                    .border_style(PURPLE)
                    .title_style(Style::new().fg(ORANGE).bold()),
            ),
        popup,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn colliding_core_always_visibly_overlaps_a_gate_tip_at_every_size() {
        for (w, h) in [(80, 28), (100, 30), (160, 50)] {
            let area = flight_area(Rect::new(0, 0, w, h));
            for top in [true, false] {
                let mut app = App::new(true);
                app.phase = Phase::Playing;
                app.game.gates.truncate(1);
                let gate = &mut app.game.gates[0];
                gate.x = game::PLAYER_X - 1.0;
                let boundary = gate.gap_y + if top { -gate.gap_half } else { gate.gap_half };
                app.game.y = boundary
                    + if top {
                        game::PLAYER_RADIUS - 0.05
                    } else {
                        -game::PLAYER_RADIUS + 0.05
                    };
                let tip_x = world_x(area, gate.x + game::GATE_WIDTH) as u16;
                let tip_y = world_y(area, boundary);
                let core_y = world_y(area, app.game.y);
                assert!(if top {
                    core_y - 1 <= tip_y
                } else {
                    core_y + 1 >= tip_y
                });
                let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
                terminal.draw(|f| draw(f, &app)).unwrap();
                assert_eq!(
                    terminal.backend().buffer()[(tip_x, tip_y as u16)].symbol(),
                    if top { "▼" } else { "▲" }
                );
                app.game.tick(1.0 / 120.0);
                assert!(!app.game.alive);
            }
        }
    }

    #[test]
    fn every_state_and_small_resize_render_without_panicking() {
        for (w, h) in [(0, 0), (1, 1), (40, 12), (80, 28), (100, 30), (160, 50)] {
            let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
            let mut app = App::new(true);
            for phase in [Phase::Title, Phase::Playing, Phase::Paused, Phase::GameOver] {
                app.phase = phase;
                app.too_small = w < MIN_WIDTH || h < MIN_HEIGHT;
                app.game.death_time = 1.0;
                terminal.draw(|f| draw(f, &app)).unwrap();
            }
        }
    }
}
