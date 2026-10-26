//! Deterministic world simulation; the app owns the fixed 120 Hz accumulator.

pub const WIDTH: f32 = 100.0;
pub const HEIGHT: f32 = 24.0;
pub const PLAYER_X: f32 = 24.0;
pub const GATE_WIDTH: f32 = 5.0;
pub const PLAYER_RADIUS: f32 = 0.6;

const PLAYER_RADIUS_X: f32 = 1.2;
const GRAVITY: f32 = 26.0;
const FLAP_VELOCITY: f32 = -10.8;
const MAX_FALL_SPEED: f32 = 16.0;
const BASE_SPEED: f32 = 16.0;
const MAX_SPEED: f32 = 23.0;
const GATE_SPACING: f32 = 36.0;
const MAX_GAP_HALF: f32 = 4.5;
const MIN_GAP_HALF: f32 = 3.5;
const MAX_GAP_SHIFT: f32 = 3.0;

#[derive(Debug, Clone, PartialEq)]
pub struct Gate {
    /// Left edge, in world coordinates.
    pub x: f32,
    pub gap_y: f32,
    pub gap_half: f32,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Candy {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Game {
    pub time: f32,
    pub y: f32,
    pub velocity: f32,
    pub gates: Vec<Gate>,
    pub candies: Vec<Candy>,
    pub score: u32,
    pub gates_passed: u32,
    pub alive: bool,
    pub death_time: f32,
    /// Seconds since the last deliberate flap, not a timestamp.
    pub flap_time: f32,
    rng: u64,
    spawned_gates: u64,
}

impl Game {
    pub fn new(seed: u64) -> Self {
        let mut game = Self {
            time: 0.0,
            y: HEIGHT / 2.0,
            velocity: 0.0,
            gates: Vec::new(),
            candies: Vec::new(),
            score: 0,
            gates_passed: 0,
            alive: true,
            death_time: 0.0,
            flap_time: 1.0,
            rng: seed,
            spawned_gates: 0,
        };
        for index in 0..3 {
            game.spawn_gate(80.0 + index as f32 * GATE_SPACING);
        }
        game
    }

    /// Call with 1/120 s, independently of the rendering frame rate.
    pub fn tick(&mut self, dt: f32) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        if !self.alive {
            self.death_time += dt;
            return;
        }

        self.time += dt;
        self.flap_time += dt;
        self.velocity = (self.velocity + GRAVITY * dt).min(MAX_FALL_SPEED);
        self.y += self.velocity * dt;
        let distance = self.speed() * dt;
        for gate in &mut self.gates {
            gate.x -= distance;
        }
        for candy in &mut self.candies {
            candy.x -= distance;
        }

        if self.collides() {
            self.alive = false;
            self.death_time = 0.0;
            return;
        }

        for gate in &mut self.gates {
            if !gate.passed && gate.x + GATE_WIDTH <= PLAYER_X - PLAYER_RADIUS_X {
                gate.passed = true;
                self.score = self.score.saturating_add(1);
                self.gates_passed = self.gates_passed.saturating_add(1);
            }
        }
        self.candies.retain(|candy| {
            let collected = (candy.x - PLAYER_X).abs() <= PLAYER_RADIUS_X + 0.6
                && (candy.y - self.y).abs() <= PLAYER_RADIUS + 0.6;
            if collected {
                self.score = self.score.saturating_add(3);
            }
            !collected && candy.x >= -1.0
        });
        self.gates.retain(|gate| gate.x + GATE_WIDTH >= 0.0);
        while self
            .gates
            .last()
            .is_none_or(|gate| gate.x < WIDTH + GATE_SPACING)
        {
            let x = self
                .gates
                .last()
                .map_or(WIDTH, |gate| gate.x + GATE_SPACING);
            self.spawn_gate(x);
        }
    }

    pub fn flap(&mut self) {
        if self.alive {
            self.velocity = FLAP_VELOCITY;
            self.flap_time = 0.0;
        }
    }

    /// Anticipates nearby walls and gate edges, without affecting physics.
    pub fn danger(&self) -> f32 {
        if !self.alive {
            return 1.0;
        }
        let predicted_y = self.y + self.velocity * 0.2;
        let wall_clearance = self
            .y
            .min(predicted_y)
            .min(HEIGHT - self.y.max(predicted_y))
            - PLAYER_RADIUS;
        let mut danger = (1.0 - wall_clearance / 4.0).clamp(0.0, 1.0);
        for gate in &self.gates {
            if gate.x + GATE_WIDTH <= PLAYER_X - PLAYER_RADIUS_X {
                continue;
            }
            let distance = (gate.x - PLAYER_X - PLAYER_RADIUS_X).max(0.0);
            let proximity = (1.0 - distance / 12.0).clamp(0.0, 1.0);
            let offset = (self.y - gate.gap_y)
                .abs()
                .max((predicted_y - gate.gap_y).abs());
            let clearance = gate.gap_half - PLAYER_RADIUS - offset;
            danger = danger.max(proximity * (1.0 - clearance / 2.5).clamp(0.0, 1.0));
        }
        danger
    }

    /// Attract-mode controller: only the same deliberate input a player can make.
    pub fn demo_input(&mut self) {
        let target = self
            .gates
            .iter()
            .find(|gate| gate.x + GATE_WIDTH > PLAYER_X - PLAYER_RADIUS_X)
            .map_or(HEIGHT / 2.0, |gate| gate.gap_y);
        if self.velocity >= 0.0 && self.y > target + 0.65 {
            self.flap();
        }
    }

    fn collides(&self) -> bool {
        // Touching is safe; only overlap of the small inner hitbox is lethal.
        self.y < PLAYER_RADIUS
            || self.y > HEIGHT - PLAYER_RADIUS
            || self.gates.iter().any(|gate| {
                PLAYER_X + PLAYER_RADIUS_X > gate.x
                    && PLAYER_X - PLAYER_RADIUS_X < gate.x + GATE_WIDTH
                    && (self.y < gate.gap_y - gate.gap_half + PLAYER_RADIUS
                        || self.y > gate.gap_y + gate.gap_half - PLAYER_RADIUS)
            })
    }

    fn difficulty(&self) -> f32 {
        (self.gates_passed as f32 / 40.0).min(1.0)
    }

    fn speed(&self) -> f32 {
        BASE_SPEED + (MAX_SPEED - BASE_SPEED) * self.difficulty()
    }

    fn spawn_gate(&mut self, x: f32) {
        let previous_y = self.gates.last().map_or(HEIGHT / 2.0, |gate| gate.gap_y);
        let gap_y = (previous_y + (self.random() * 2.0 - 1.0) * MAX_GAP_SHIFT)
            .clamp(MAX_GAP_HALF + 1.0, HEIGHT - MAX_GAP_HALF - 1.0);
        let gap_half = MAX_GAP_HALF - (MAX_GAP_HALF - MIN_GAP_HALF) * self.difficulty();
        self.gates.push(Gate {
            x,
            gap_y,
            gap_half,
            passed: false,
        });
        self.spawned_gates = self.spawned_gates.wrapping_add(1);
        if self.spawned_gates.is_multiple_of(3) {
            self.candies.push(Candy {
                x: x + GATE_WIDTH / 2.0,
                y: gap_y,
            });
        }
    }

    fn random(&mut self) -> f32 {
        // SplitMix64 also accepts seed zero without a degenerate state.
        self.rng = self.rng.wrapping_add(0x9e3779b97f4a7c15);
        let mut bits = self.rng;
        bits = (bits ^ (bits >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        bits = (bits ^ (bits >> 27)).wrapping_mul(0x94d049bb133111eb);
        bits ^= bits >> 31;
        (bits >> 40) as f32 / 16_777_216.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 120.0;

    fn open_game() -> Game {
        let mut game = Game::new(42);
        game.gates.clear();
        game.candies.clear();
        game
    }

    fn nearby_gate() -> Gate {
        Gate {
            x: PLAYER_X - GATE_WIDTH / 2.0,
            gap_y: 12.0,
            gap_half: 4.0,
            passed: false,
        }
    }

    #[test]
    fn starts_central_with_runway_and_no_flap() {
        let game = Game::new(0);
        assert_eq!(game.y, HEIGHT / 2.0);
        assert_eq!(game.velocity, 0.0);
        assert_eq!(game.time, 0.0);
        assert!(game.flap_time > 0.5);
        assert!(game.gates[0].x - PLAYER_X > BASE_SPEED * 3.0);
        assert!(!game.collides());
    }

    #[test]
    fn exact_world_boundaries_are_safe_but_overlap_kills() {
        let mut game = open_game();
        for y in [PLAYER_RADIUS, HEIGHT - PLAYER_RADIUS] {
            game.y = y;
            assert!(!game.collides());
        }
        for y in [PLAYER_RADIUS - 0.0001, HEIGHT - PLAYER_RADIUS + 0.0001] {
            game.y = y;
            assert!(game.collides());
        }
    }

    #[test]
    fn exact_gap_boundaries_are_safe_but_overlap_kills() {
        let mut game = open_game();
        game.gates.push(nearby_gate());
        let top = 12.0 - 4.0 + PLAYER_RADIUS;
        let bottom = 12.0 + 4.0 - PLAYER_RADIUS;
        for y in [top, bottom] {
            game.y = y;
            assert!(!game.collides());
        }
        for y in [top - 0.0001, bottom + 0.0001] {
            game.y = y;
            assert!(game.collides());
        }
    }

    #[test]
    fn horizontal_hitbox_uses_aspect_correction_and_safe_contact() {
        let mut game = open_game();
        game.y = 4.0;
        game.gates.push(nearby_gate());
        game.gates[0].x = PLAYER_X + PLAYER_RADIUS_X;
        assert!(!game.collides());
        game.gates[0].x -= 0.0001;
        assert!(game.collides());
        game.gates[0].x = PLAYER_X - PLAYER_RADIUS_X - GATE_WIDTH;
        assert!(!game.collides());
        game.gates[0].x += 0.0001;
        assert!(game.collides());
    }

    #[test]
    fn tick_detects_collision_and_only_death_time_advances_afterward() {
        let mut game = open_game();
        game.y = HEIGHT;
        game.tick(DT);
        assert!(!game.alive);
        assert_eq!(game.death_time, 0.0);
        let mut expected = game.clone();
        expected.death_time = 0.5;
        game.flap();
        game.tick(0.5);
        assert_eq!(game, expected);
    }

    #[test]
    fn deliberate_flap_resets_impulse_and_elapsed_flap_time() {
        let mut game = open_game();
        game.tick(DT);
        assert!(game.velocity > 0.0);
        game.flap();
        assert_eq!(game.velocity, FLAP_VELOCITY);
        assert_eq!(game.flap_time, 0.0);
        game.tick(DT);
        assert_eq!(game.flap_time, DT);
        assert!(game.velocity > FLAP_VELOCITY);
    }

    #[test]
    fn each_gate_scores_once_only_after_its_trailing_edge_clears() {
        let mut game = open_game();
        let mut gate = nearby_gate();
        gate.x = PLAYER_X - PLAYER_RADIUS_X - GATE_WIDTH + BASE_SPEED * DT * 1.5;
        game.gates.push(gate);
        game.tick(DT);
        assert_eq!(game.score, 0);
        game.tick(DT);
        assert_eq!(game.score, 1);
        assert_eq!(game.gates_passed, 1);
        assert!(game.gates[0].passed);
        for _ in 0..10 {
            game.tick(DT);
        }
        assert_eq!(game.score, 1);
        assert_eq!(game.gates_passed, 1);
    }

    #[test]
    fn candy_adds_three_once_without_raising_difficulty() {
        let mut game = open_game();
        game.candies.push(Candy {
            x: PLAYER_X,
            y: game.y,
        });
        game.tick(DT);
        assert_eq!(game.score, 3);
        assert_eq!(game.gates_passed, 0);
        assert!(game.candies.iter().all(|candy| candy.x > PLAYER_X + 10.0));
        game.tick(DT);
        assert_eq!(game.score, 3);
        assert_eq!(game.speed(), BASE_SPEED);
    }

    #[test]
    fn uncollected_candy_leaves_the_world() {
        let mut game = open_game();
        game.candies.push(Candy { x: -0.99, y: 2.0 });
        game.tick(DT);
        assert_eq!(game.score, 0);
        assert!(game.candies.iter().all(|candy| candy.x >= -1.0));
    }

    #[test]
    fn difficulty_and_adjacent_gap_shift_are_bounded() {
        for gates_passed in [0, 20, 40, u32::MAX] {
            let mut game = open_game();
            game.gates_passed = gates_passed;
            assert!((BASE_SPEED..=MAX_SPEED).contains(&game.speed()));
            for index in 0..300 {
                game.spawn_gate(80.0 + index as f32 * GATE_SPACING);
            }
            for gate in &game.gates {
                assert!((MIN_GAP_HALF..=MAX_GAP_HALF).contains(&gate.gap_half));
                assert!(gate.gap_y - gate.gap_half >= 1.0);
                assert!(gate.gap_y + gate.gap_half <= HEIGHT - 1.0);
            }
            for pair in game.gates.windows(2) {
                assert!((pair[1].gap_y - pair[0].gap_y).abs() <= MAX_GAP_SHIFT + 0.00001);
                assert_eq!(pair[1].x - pair[0].x, GATE_SPACING);
            }
            assert_eq!(game.candies.len(), 100);
            if gates_passed >= 40 {
                assert_eq!(game.speed(), MAX_SPEED);
                assert_eq!(game.gates[0].gap_half, MIN_GAP_HALF);
            }
        }
    }

    #[test]
    fn downward_speed_is_capped() {
        let mut game = open_game();
        game.velocity = MAX_FALL_SPEED;
        game.tick(DT);
        assert_eq!(game.velocity, MAX_FALL_SPEED);
    }

    #[test]
    fn worried_metric_is_bounded_and_anticipates_nearby_danger() {
        let mut game = open_game();
        assert_eq!(game.danger(), 0.0);
        game.y = 1.0;
        assert!(game.danger() > 0.8);
        game.y = 12.0;
        game.gates.push(nearby_gate());
        assert_eq!(game.danger(), 0.0);
        game.y = 8.0;
        assert_eq!(game.danger(), 1.0);
        for y in 0..=24 {
            for velocity in [-10.8, 0.0, 16.0] {
                game.y = y as f32;
                game.velocity = velocity;
                assert!((0.0..=1.0).contains(&game.danger()));
            }
        }
        game.alive = false;
        assert_eq!(game.danger(), 1.0);
    }

    #[test]
    fn seed_and_inputs_reproduce_every_bit_of_state() {
        assert_ne!(Game::new(0).gates, Game::new(1).gates);
        let mut first = Game::new(u64::MAX);
        let mut second = Game::new(u64::MAX);
        for _ in 0..120 * 30 {
            first.demo_input();
            second.demo_input();
            first.tick(DT);
            second.tick(DT);
            assert_eq!(first, second);
        }
    }

    fn simulate_render_frames(fps: u32) -> Game {
        let mut game = Game::new(19);
        let mut accumulator = 0.0_f64;
        for _ in 0..fps * 30 {
            accumulator += 1.0 / fps as f64;
            while accumulator + 1e-12 >= 1.0 / 120.0 {
                game.demo_input();
                game.tick(DT);
                accumulator -= 1.0 / 120.0;
            }
        }
        game
    }

    #[test]
    fn render_rate_does_not_change_fixed_step_simulation() {
        let reference = simulate_render_frames(30);
        assert!(reference.alive);
        for fps in [17, 24, 60, 144] {
            assert_eq!(reference, simulate_render_frames(fps), "render fps {fps}");
        }
    }

    #[test]
    fn demo_survives_three_minutes_across_seeds_and_maximum_difficulty() {
        for seed in [0, 1, 2, 3, 7, 19, 42, 123, 1024, u64::MAX] {
            let mut game = Game::new(seed);
            for tick in 0..120 * 180 {
                game.demo_input();
                game.tick(DT);
                assert!(game.alive, "seed {}, tick {}: {:?}", seed, tick, game);
            }
            assert!(game.gates_passed > 80);
            assert!(game.score > game.gates_passed);
            assert_eq!(game.speed(), MAX_SPEED);
            assert!(game.gates.len() <= 5);
            assert!(game.candies.len() <= 3);
        }
    }

    #[test]
    fn invalid_elapsed_time_is_a_noop() {
        let mut game = Game::new(42);
        let expected = game.clone();
        for dt in [0.0, -DT, f32::NAN, f32::INFINITY] {
            game.tick(dt);
            assert_eq!(game, expected);
        }
    }
}
