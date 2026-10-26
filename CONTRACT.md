# Flappy Jack integration contract

The repository began empty. The coordinator owns integration and `Cargo.toml`, `Cargo.lock`, `src/lib.rs`, `src/main.rs`, `src/ui.rs`, README, and tools/ verification scripts. Workers own only the files below. No external runtime assets; only dependency is Ratatui 0.30.2 with its re-exported Crossterm. Rust 2024. Use `ratatui::crossterm`, not a separate dependency. Do not edit another owner's files. Commit only your files when finished; coordinator reviews and cherry-picks. Other workers' stubs remain until integrated. Check your module independently if whole-crate tests are blocked by stubs.

## Renderer/title worker
Owns `src/pumpkin.rs`, `src/title.rs`.
- `pub fn pumpkin::render(buf: &mut ratatui::buffer::Buffer, area: ratatui::layout::Rect, seconds: f32)` draws real rotating 3D ribbed, carved jack-o'-lantern with stem. Prefer half-block RGB cells, aspect correction, depth handling, surface normals, orange ribs and warm flickering apertures. Lightweight software mesh rasterizer or implicit surface, not a flat image. Compelling initial orientation at seconds=0.
- `pub fn title::draw(frame: &mut ratatui::Frame, seconds: f32, best: u32, result: Option<u32>, demo: bool)` draws title or game-over screen, with 3D pumpkin reused. Title exact text FLAPPY JACK and Press Space to awaken. Game-over score/best and Space/R restart. Nearly black backdrop, restrained purple, moon and subtle ghosts. At 100x30 pumpkin should be prominent. May scale layout for minimum80x28. Include small renderer regression tests.

## Gameplay worker
Owns `src/game.rs` only. No terminal or Ratatui imports.
World coordinates: `pub const WIDTH: f32 = 100.0; pub const HEIGHT: f32 = 24.0; pub const PLAYER_X: f32 = 24.0; pub const GATE_WIDTH: f32 = 5.0; pub const PLAYER_RADIUS: f32` (vertical hitbox around 0.6; horizontal aspect correction around 1.2).
`pub struct Gate { pub x: f32, pub gap_y: f32, pub gap_half: f32, pub passed: bool }`
`pub struct Candy { pub x: f32, pub y: f32 }`
`pub struct Game { pub time: f32, pub y: f32, pub velocity: f32, pub gates: Vec<Gate>, pub candies: Vec<Candy>, pub score: u32, pub gates_passed: u32, pub alive: bool, pub death_time: f32, pub flap_time: f32, ...private fields allowed }`
- `pub fn Game::new(seed: u64) -> Self` reproducible gates, initial safe central height, velocity=0; enough runway for title/start (no implicit flap).
- `pub fn tick(&mut self, dt: f32)` shared fixed step 1/120s, after death advances death_time only. Gravity, clamped difficulty/speed, passable gaps with bounded adjacent centers, collision fair inner hitbox, score exactly once per passed gate, candy gives +3 and removed. Seeded own small PRNG, no rand crate.
- `pub fn flap(&mut self)` deliberate impulse, stores flap_time (elapsed since flap; tick increments).
- `pub fn danger(&self) -> f32` 0..1 for worried expression.
- `pub fn demo_input(&mut self)` deterministic autopilot intended to survive attract recording; call every fixed tick before tick. Must not change normal input/physics.
Tests cover exact collision boundaries, scoring once, candy, difficulty caps, reproducibility and frame-rate independence using fixed step or equivalent.

## App/terminal/storage worker
Owns `src/app.rs`, `src/terminal.rs`, `src/storage.rs` only.
`pub enum Phase { Title, Playing, Paused, GameOver }` derive Eq PartialEq Debug Clone Copy.
`pub struct App { pub phase: Phase, pub game: crate::game::Game, pub seconds: f32, pub best: u32, pub demo: bool, pub quit: bool, pub too_small: bool, ...private fields allowed }`
`App::new(demo: bool)`, `App::advance(elapsed: std::time::Duration, too_small: bool)`, `App::key(key: ratatui::crossterm::event::KeyEvent)`.
Fixed 120Hz physics; visual seconds independent; clamp long stalls. Space starts from Title or GameOver without also flapping; Playing Space flaps. R immediate restart only GameOver. P/Esc toggles pause, Q quits (Ctrl-C also helpful). Ignore KeyEventKind::Repeat and Release deliberately; short debounce for legacy repeated Press around transitions. Don't accidentally flap from same held key across transitions. Death transitions GameOver, retains Game for seed splatter .35sec; restart permitted <=.5sec. When too small (<80x28), freeze physics/input except quit; restore without catch-up; remember manual pause. --demo spends ~5sec on title then uses game.demo_input, cycles if dead. Store best locally in XDG_DATA_HOME/flappy-jack/best-score or HOME/.local/share/...; storage failures non-fatal. Never touch actual user's best in tests or demo.
`pub fn terminal::run(app: &mut App, draw: impl FnMut(&mut ratatui::Frame, &App)) -> std::io::Result<()>`: 30FPS rendering independent of 120Hz simulation; crossterm blocking poll with timeout, drain events bounded, detect resize, restore terminal normal/error/panic via Ratatui built-in hook and guard; no busy loop. Export `pub const MIN_WIDTH: u16=80; pub const MIN_HEIGHT:u16=28`. Use ratatui::try_init or run helper appropriate current docs. Helpful resize overlay belongs coordinator UI, set app.too_small accurately. Unit tests cover state transition controls/no accidental flap, pause/resizing freeze, fixed-step accumulator; storage isolated test dirs.

## Coordinator UI
`pub fn ui::draw(frame: &mut ratatui::Frame, app: &crate::app::App)` title/gameover delegates title, playing draws procedural cemetery and tiny expressive pumpkin; uses world scaling, scoreboard + candy +3 explanation, pause and resize overlays. GameOver first ~.35sec can show seed splatter from game, then title result. Integration and terminal verification owned coordinator.

Official API references consulted: https://docs.rs/ratatui/0.30.2/ratatui/fn.init.html (built-in panic hook), https://docs.rs/ratatui/0.30.2/ratatui/struct.Frame.html , https://docs.rs/ratatui/0.30.2/ratatui/buffer/struct.Buffer.html , https://docs.rs/crossterm/latest/crossterm/event/enum.KeyEventKind.html . Consult official APIs you use. Minimal sync loop; no Tokio.
