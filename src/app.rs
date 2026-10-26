use std::{path::PathBuf, time::Duration};

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::{game::Game, storage};

const HZ: u128 = 120;
const TICK: u128 = 1_000_000_000;
const DT: f32 = 1.0 / HZ as f32;
const MAX_ELAPSED: Duration = Duration::from_millis(250);
const TRANSITION_GUARD: Duration = Duration::from_millis(350);
const SEED: u64 = 0x4a41_434b;

#[derive(Eq, PartialEq, Debug, Clone, Copy)]
pub enum Phase {
    Title,
    Playing,
    Paused,
    GameOver,
}

pub struct App {
    pub phase: Phase,
    pub game: Game,
    pub seconds: f32,
    pub best: u32,
    pub demo: bool,
    pub quit: bool,
    pub too_small: bool,
    accumulator: u128,
    clock: Duration,
    guard: Option<(KeyCode, Duration)>,
    title_ticks: u32,
    seed: u64,
    score_path: Option<PathBuf>,
}

impl App {
    pub fn new(demo: bool) -> Self {
        // Unit tests and attract recordings must never read or write a player's record.
        let score_path = if demo || cfg!(test) {
            None
        } else {
            storage::best_path()
        };
        let best = score_path.as_deref().map(storage::load).unwrap_or(0);
        Self {
            phase: Phase::Title,
            game: Game::new(SEED),
            seconds: 0.0,
            best,
            demo,
            quit: false,
            too_small: false,
            accumulator: 0,
            clock: Duration::ZERO,
            guard: None,
            title_ticks: 0,
            seed: SEED,
            score_path,
        }
    }

    pub fn advance(&mut self, elapsed: Duration, too_small: bool) {
        self.clock = self.clock.saturating_add(elapsed);
        let resized = self.too_small != too_small;
        self.too_small = too_small;
        if self.quit || too_small || resized || self.phase == Phase::Paused {
            self.accumulator = 0;
            return;
        }
        let elapsed = elapsed.min(MAX_ELAPSED);
        self.seconds += elapsed.as_secs_f32();
        // Integer fractional ticks retain the exact remainder at every rendering cadence.
        self.accumulator += elapsed.as_nanos() * HZ;
        while self.accumulator >= TICK {
            self.accumulator -= TICK;
            match self.phase {
                Phase::Title => {
                    self.title_ticks += 1;
                    if self.demo && self.title_ticks >= 600 {
                        self.start();
                    }
                }
                Phase::Playing => {
                    if self.demo {
                        self.game.demo_input();
                    }
                    self.game.tick(DT);
                    if !self.game.alive {
                        self.phase = Phase::GameOver;
                        self.arm_guard(KeyCode::Char(' '));
                        if self.game.score > self.best {
                            self.best = self.game.score;
                            if !self.demo
                                && let Some(path) = &self.score_path
                            {
                                let _ = storage::save(path, self.best);
                            }
                        }
                    }
                }
                Phase::GameOver => {
                    self.game.tick(DT);
                    if self.demo && self.game.death_time >= 1.5 {
                        self.start();
                    }
                }
                Phase::Paused => unreachable!(),
            }
        }
    }

    pub fn key(&mut self, key: KeyEvent) {
        let code = match key.code {
            KeyCode::Char(c) => KeyCode::Char(c.to_ascii_lowercase()),
            other => other,
        };
        if key.kind == KeyEventKind::Release {
            if self.guard.is_some_and(|(guarded, _)| guarded == code) {
                self.guard = None;
            }
            return;
        }
        if key.kind != KeyEventKind::Press {
            return;
        }
        if code == KeyCode::Char('q')
            || (code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL))
        {
            self.quit = true;
            return;
        }
        if self.quit || self.too_small {
            return;
        }
        if let Some((guarded, until)) = self.guard
            && code == guarded
            && self.clock < until
        {
            // shortcut: legacy Press cannot identify a held key after 350 ms quiet; use release reporting where supported.
            self.arm_guard(code);
            return;
        }
        match (self.phase, code) {
            (Phase::Title | Phase::GameOver, KeyCode::Char(' '))
            | (Phase::GameOver, KeyCode::Char('r')) => self.start(),
            (Phase::Playing, KeyCode::Char(' ')) => {
                self.game.flap();
                return;
            }
            (Phase::Playing, KeyCode::Char('p') | KeyCode::Esc) => self.phase = Phase::Paused,
            (Phase::Paused, KeyCode::Char('p') | KeyCode::Esc) => self.phase = Phase::Playing,
            _ => return,
        }
        self.accumulator = 0;
        self.arm_guard(code);
    }

    fn start(&mut self) {
        self.game = Game::new(self.seed);
        if !self.demo {
            self.seed = self.seed.wrapping_add(1);
        }
        self.phase = Phase::Playing;
    }

    fn arm_guard(&mut self, code: KeyCode) {
        self.guard = Some((code, self.clock.saturating_add(TRANSITION_GUARD)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(app: &mut App, code: char) {
        app.key(KeyEvent::new(KeyCode::Char(code), KeyModifiers::NONE));
    }

    fn advance(app: &mut App, millis: u64) {
        app.advance(Duration::from_millis(millis), false);
    }

    #[test]
    fn start_and_restart_never_flap_and_legacy_repeats_are_guarded() {
        let mut app = App::new(false);
        press(&mut app, 'r');
        assert_eq!(app.phase, Phase::Title);
        press(&mut app, ' ');
        assert_eq!(app.phase, Phase::Playing);
        assert_eq!(app.game.velocity, 0.0);
        press(&mut app, ' ');
        assert_eq!(app.game.velocity, 0.0);
        advance(&mut app, 200);
        let velocity = app.game.velocity;
        press(&mut app, ' ');
        assert_eq!(app.game.velocity, velocity);
        advance(&mut app, 200);
        let velocity = app.game.velocity;
        press(&mut app, ' ');
        assert_eq!(app.game.velocity, velocity);
        // Let the debounce expire without advancing physics into a collision.
        app.phase = Phase::Paused;
        advance(&mut app, 400);
        app.phase = Phase::Playing;
        press(&mut app, ' ');
        assert!(app.game.velocity < 0.0);
        app.phase = Phase::GameOver;
        press(&mut app, 'r');
        assert_eq!(app.phase, Phase::Playing);
        assert_eq!(app.game.velocity, 0.0);
        app.phase = Phase::GameOver;
        app.clock += TRANSITION_GUARD;
        press(&mut app, ' ');
        assert_eq!(app.phase, Phase::Playing);
        assert_eq!(app.game.velocity, 0.0);
    }

    #[test]
    fn enhanced_delayed_repeats_do_not_toggle_and_release_allows_an_immediate_tap() {
        let mut app = App::new(false);
        press(&mut app, ' ');
        app.key(KeyEvent::new_with_kind(
            KeyCode::Char(' '),
            KeyModifiers::NONE,
            KeyEventKind::Release,
        ));
        press(&mut app, ' ');
        assert!(app.game.velocity < 0.0);
        press(&mut app, 'p');
        advance(&mut app, 500);
        app.key(KeyEvent::new_with_kind(
            KeyCode::Char('p'),
            KeyModifiers::NONE,
            KeyEventKind::Repeat,
        ));
        assert_eq!(app.phase, Phase::Paused);
        app.key(KeyEvent::new_with_kind(
            KeyCode::Char('p'),
            KeyModifiers::NONE,
            KeyEventKind::Release,
        ));
        press(&mut app, 'p');
        assert_eq!(app.phase, Phase::Playing);
    }

    #[test]
    fn only_press_events_act_and_quit_works_while_too_small() {
        let mut app = App::new(false);
        for kind in [KeyEventKind::Repeat, KeyEventKind::Release] {
            app.key(KeyEvent::new_with_kind(
                KeyCode::Char(' '),
                KeyModifiers::NONE,
                kind,
            ));
            assert_eq!(app.phase, Phase::Title);
            app.key(KeyEvent::new_with_kind(
                KeyCode::Char('q'),
                KeyModifiers::NONE,
                kind,
            ));
            assert!(!app.quit);
        }
        app.advance(Duration::ZERO, true);
        press(&mut app, ' ');
        assert_eq!(app.phase, Phase::Title);
        app.key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert!(app.quit);
        let mut other = App::new(false);
        press(&mut other, 'Q');
        assert!(other.quit);
    }

    #[test]
    fn pause_and_resize_freeze_without_catchup_and_keep_manual_pause() {
        let mut app = App::new(false);
        press(&mut app, ' ');
        advance(&mut app, 40);
        press(&mut app, 'p');
        let frozen = (app.game.time, app.game.y, app.seconds);
        advance(&mut app, 1000);
        press(&mut app, ' ');
        assert_eq!((app.game.time, app.game.y, app.seconds), frozen);
        app.advance(Duration::from_secs(1), true);
        press(&mut app, 'p');
        assert_eq!(app.phase, Phase::Paused);
        app.advance(Duration::from_secs(1), false);
        assert_eq!(app.phase, Phase::Paused);
        assert_eq!((app.game.time, app.game.y, app.seconds), frozen);
        press(&mut app, 'p');
        assert_eq!(app.phase, Phase::Playing);
        advance(&mut app, 0);
        assert_eq!((app.game.time, app.game.y, app.seconds), frozen);
        advance(&mut app, 10);
        assert!(app.game.time > frozen.0);
        let frozen = (app.game.time, app.game.y, app.seconds);
        app.advance(Duration::from_secs(1), true);
        app.advance(Duration::from_secs(1), true);
        app.advance(Duration::from_secs(1), false);
        assert_eq!((app.game.time, app.game.y, app.seconds), frozen);
        advance(&mut app, 0);
        assert_eq!((app.game.time, app.game.y, app.seconds), frozen);
    }

    #[test]
    fn pause_toggle_does_not_repeat_and_esc_resumes() {
        let mut app = App::new(false);
        press(&mut app, ' ');
        press(&mut app, 'p');
        press(&mut app, 'p');
        assert_eq!(app.phase, Phase::Paused);
        app.key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(app.phase, Phase::Playing);
        app.key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(app.phase, Phase::Playing);
    }

    #[test]
    fn accumulator_is_exact_across_frame_sizes_and_stalls_are_clamped() {
        let mut fast = App::new(true);
        let mut slow = App::new(true);
        for _ in 0..1000 {
            fast.advance(Duration::from_millis(7), false);
        }
        for _ in 0..200 {
            slow.advance(Duration::from_millis(35), false);
        }
        assert_eq!(fast.phase, slow.phase);
        assert_eq!(fast.game.time, slow.game.time);
        assert_eq!(fast.game.y, slow.game.y);
        assert_eq!(fast.game.velocity, slow.game.velocity);
        assert_eq!(fast.game.score, slow.game.score);
        assert_eq!(fast.accumulator, slow.accumulator);
        let mut app = App::new(false);
        press(&mut app, ' ');
        app.advance(Duration::from_secs(60), false);
        assert!((app.game.time - 0.25).abs() < 0.00001);
        assert_eq!(app.seconds, 0.25);
    }

    #[test]
    fn demo_waits_five_seconds_then_plays_and_never_opens_a_score_file() {
        let mut app = App::new(true);
        assert!(app.score_path.is_none());
        for _ in 0..49 {
            advance(&mut app, 100);
        }
        assert_eq!(app.phase, Phase::Title);
        advance(&mut app, 100);
        assert_eq!(app.phase, Phase::Playing);
        assert_eq!(app.game.time, 0.0);
        assert_eq!(app.game.velocity, 0.0);
        advance(&mut app, 10);
        assert!(app.game.time > 0.0);
        assert!(App::new(false).score_path.is_none());
    }

    #[test]
    fn death_keeps_the_game_and_saves_best_but_storage_failure_is_nonfatal() {
        let mut app = App::new(false);
        // A path below a regular file cannot be created, even when tests run as root.
        let dir = storage::tests::TestDir::new();
        let blocker = dir.0.join("file");
        std::fs::write(&blocker, "unchanged").unwrap();
        app.score_path = Some(blocker.join("best-score"));
        press(&mut app, ' ');
        app.game.score = 17;
        app.game.alive = false;
        advance(&mut app, 10);
        assert_eq!(app.phase, Phase::GameOver);
        assert_eq!(app.best, 17);
        assert_eq!(app.game.score, 17);
        let death_time = app.game.death_time;
        advance(&mut app, 100);
        assert!(app.game.death_time > death_time);
        press(&mut app, ' ');
        assert_eq!(app.phase, Phase::GameOver);
        advance(&mut app, 250);
        advance(&mut app, 100);
        press(&mut app, ' ');
        assert_eq!(app.phase, Phase::Playing);
        assert_eq!(app.game.velocity, 0.0);
        assert_eq!(app.best, 17);
        assert_eq!(std::fs::read_to_string(blocker).unwrap(), "unchanged");
    }

    #[test]
    fn new_best_persists_only_for_a_real_game_to_an_injected_test_path() {
        let dir = storage::tests::TestDir::new();
        for demo in [false, true] {
            let path = dir.0.join(if demo { "demo" } else { "player" });
            let mut app = App::new(demo);
            app.score_path = Some(path.clone());
            press(&mut app, ' ');
            app.game.score = 23;
            app.game.alive = false;
            advance(&mut app, 10);
            assert_eq!(app.best, 23);
            assert_eq!(path.exists(), !demo);
            if !demo {
                assert_eq!(storage::load(&path), 23);
            }
        }
    }
}
