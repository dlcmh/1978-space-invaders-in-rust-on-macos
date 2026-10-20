//! The authoritative Space Invaders simulation.
//!
//! The game advances in fixed 60 Hz ticks. It knows nothing about Metal,
//! `AppKit`, audio, windows, or wall-clock time; the app shell feeds it ticks and
//! consumes [`GameEvent`]s for sound and presentation. That split makes the
//! whole simulation runnable in `cargo test` and in the standalone checks
//! harness.

use crate::bunker::Bunker;
use crate::geometry::Rect;
use crate::layout::{
    BASE_ENEMY_BULLETS, BONUS_LIFE_SCORE, BUNKER_COUNT, ENEMY_BULLET_SPEED, FORMATION_COLUMNS,
    FORMATION_DROP, FORMATION_ROWS, FORMATION_SPACING_X, FORMATION_SPACING_Y, FORMATION_X,
    FORMATION_Y, HUD_BOTTOM, MYSTERY_HEIGHT, MYSTERY_SPEED, MYSTERY_WIDTH, MYSTERY_Y, PLAY_LEFT,
    PLAY_RIGHT, PLAYER_BULLET_SPEED, PLAYER_HEIGHT, PLAYER_SPEED, PLAYER_WIDTH, PLAYER_Y,
    SCREEN_WIDTH, STARTING_LIVES,
};
use crate::rng::Rng;

/// The top-level game phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Title screen, waiting for the player.
    Attract,
    /// A short pause before play begins again.
    Ready {
        /// Ticks until [`Phase::Playing`].
        ticks_left: u32,
    },
    /// Normal gameplay.
    Playing,
    /// The cannon has been hit; the explosion plays out.
    PlayerDeath {
        /// Ticks until the next ready or game-over screen.
        ticks_left: u32,
    },
    /// The last invader is gone; the next wave is being prepared.
    WaveClear {
        /// Ticks until the next wave.
        ticks_left: u32,
    },
    /// Round over.
    GameOver,
}

/// Commands accepted by [`Game`]. Input is expressed as intent, not as direct
/// mutation of game state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    /// Start a new game from the attract or game-over screen.
    Start,
    /// Fire the cannon if it is able to.
    Fire,
    /// Hold or release the fire button. While held, the cannon reloads and
    /// fires again as soon as its single bullet leaves play.
    SetFire(bool),
    /// Pause or resume a live round.
    TogglePause,
    /// Hold or release the left movement direction.
    SetLeft(bool),
    /// Hold or release the right movement direction.
    SetRight(bool),
}

/// The three invader species and their canonical point values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienKind {
    /// Top row, 30 points.
    Squid,
    /// Middle rows, 20 points.
    Crab,
    /// Bottom rows, 10 points.
    Octopus,
}

impl AlienKind {
    /// Points awarded for destroying this species.
    #[must_use]
    pub const fn points(self) -> u32 {
        match self {
            Self::Squid => 30,
            Self::Crab => 20,
            Self::Octopus => 10,
        }
    }

    /// Sprite width in logical pixels.
    #[must_use]
    pub const fn width(self) -> f32 {
        match self {
            Self::Squid => 8.0,
            Self::Crab => 11.0,
            Self::Octopus => 12.0,
        }
    }

    /// Sprite height in logical pixels.
    #[must_use]
    pub const fn height(self) -> f32 {
        8.0
    }
}

/// One living member of the formation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Alien {
    /// Species.
    pub kind: AlienKind,
    /// Formation row, 0 at the top.
    pub row: usize,
    /// Formation column, 0 at the left.
    pub column: usize,
    /// Top-left x in arcade pixels.
    pub x: f32,
    /// Top-left y in arcade pixels.
    pub y: f32,
    /// Whether this alien is still alive.
    pub alive: bool,
}

impl Alien {
    fn new(kind: AlienKind, row: usize, column: usize, x: f32, y: f32) -> Self {
        Self {
            kind,
            row,
            column,
            x,
            y,
            alive: true,
        }
    }

    /// Bounding rectangle used for collision checks.
    #[must_use]
    pub fn rect(self) -> Rect {
        Rect::new(self.x, self.y, self.kind.width(), self.kind.height())
    }
}

/// The player's laser cannon.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Player {
    /// Left edge in arcade pixels.
    pub x: f32,
    /// Top edge in arcade pixels.
    pub y: f32,
    /// Cleared during the death animation.
    pub alive: bool,
}

impl Player {
    fn new() -> Self {
        Self {
            x: (SCREEN_WIDTH - PLAYER_WIDTH) / 2.0,
            y: PLAYER_Y,
            alive: true,
        }
    }

    /// Bounding rectangle used for collision checks.
    #[must_use]
    pub fn rect(self) -> Rect {
        Rect::new(self.x, self.y, PLAYER_WIDTH, PLAYER_HEIGHT)
    }

    /// Horizontal centre, used as the bullet spawn point.
    #[must_use]
    pub fn centre_x(self) -> f32 {
        self.x + PLAYER_WIDTH / 2.0
    }
}

/// A projectile in flight.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bullet {
    /// Left edge.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Width, always one logical pixel.
    pub w: f32,
    /// Height in logical pixels.
    pub h: f32,
    /// Vertical velocity; negative values travel upward.
    pub velocity: f32,
}

impl Bullet {
    /// Bounding rectangle used for collision checks.
    #[must_use]
    pub fn rect(self) -> Rect {
        Rect::new(self.x, self.y, self.w, self.h)
    }
}

/// The bonus ship crossing the top of the screen.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MysteryShip {
    /// Left edge.
    pub x: f32,
    /// Horizontal direction, `1.0` or `-1.0`.
    pub direction: f32,
    /// Score the ship is currently worth.
    pub points: u32,
}

/// A short-lived visual effect produced by the simulation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Explosion {
    /// Centre x in arcade pixels.
    pub x: f32,
    /// Centre y in arcade pixels.
    pub y: f32,
    /// What exploded.
    pub kind: ExplosionKind,
    /// Ticks left before the effect disappears.
    pub ticks_left: u32,
    /// Duration the effect started with.
    pub duration: u32,
}

/// The reason an explosion exists.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExplosionKind {
    /// A destroyed invader.
    Invader,
    /// The player's cannon.
    Player,
    /// A projectile hitting cover or another projectile.
    Spark,
    /// The mystery ship.
    Mystery,
}

/// A floating score shown after a mystery-ship kill.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScorePopup {
    /// Left edge of the text.
    pub x: f32,
    /// Baseline y of the text.
    pub y: f32,
    /// Score to display.
    pub points: u32,
    /// Ticks left before it disappears.
    pub ticks_left: u32,
}

/// Semantic things that happened during a tick.
///
/// The app turns these into sound; the checks harness counts and asserts on
/// them. Presentation never inspects or mutates simulation internals directly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameEvent {
    /// A fresh round began.
    GameStarted,
    /// The next wave is ready.
    WaveStarted,
    /// The formation stepped; drives the four-note marching sound.
    AlienStep,
    /// The player fired.
    PlayerShoot,
    /// An enemy fired.
    EnemyShoot,
    /// An invader was destroyed.
    InvaderKilled {
        /// Species destroyed.
        kind: AlienKind,
        /// Points awarded.
        points: u32,
    },
    /// The player's cannon was hit.
    PlayerHit,
    /// The mystery ship entered the screen.
    MysteryAppeared,
    /// The mystery ship was destroyed.
    MysteryKilled {
        /// Points awarded.
        points: u32,
    },
    /// Every invader is gone.
    WaveCleared,
    /// The single bonus life was awarded.
    ExtraLife,
    /// The round is over.
    GameOver,
}

/// The complete, deterministic game state.
#[derive(Debug)]
pub struct Game {
    phase: Phase,
    paused: bool,
    score: u32,
    high_score: u32,
    lives: u32,
    level: u32,
    player: Player,
    player_bullet: Option<Bullet>,
    enemy_bullets: Vec<Bullet>,
    aliens: Vec<Alien>,
    bunkers: [Bunker; BUNKER_COUNT],
    direction: f32,
    step_timer: u32,
    animation_frame: u8,
    fire_timer: u32,
    mystery_timer: u32,
    mystery: Option<MysteryShip>,
    explosions: Vec<Explosion>,
    score_popups: Vec<ScorePopup>,
    events: Vec<GameEvent>,
    left_held: bool,
    right_held: bool,
    fire_held: bool,
    shots_fired: u32,
    bonus_life_awarded: bool,
    rng: Rng,
    ticks: u64,
}

impl Game {
    /// Creates an attract-mode game with the supplied persistent high score.
    #[must_use]
    pub fn new(high_score: u32) -> Self {
        Self::with_seed(high_score, 0x1978_5EED_CAFE_BEEF)
    }

    /// Creates a game with an explicit random seed. Used by deterministic tests.
    #[must_use]
    pub fn with_seed(high_score: u32, seed: u64) -> Self {
        let mut game = Self {
            phase: Phase::Attract,
            paused: false,
            score: 0,
            high_score,
            lives: STARTING_LIVES,
            level: 1,
            player: Player::new(),
            player_bullet: None,
            enemy_bullets: Vec::new(),
            aliens: Vec::with_capacity(FORMATION_COLUMNS * FORMATION_ROWS),
            bunkers: Bunker::standard_set(),
            direction: 1.0,
            step_timer: 0,
            animation_frame: 0,
            fire_timer: 90,
            mystery_timer: 900,
            mystery: None,
            explosions: Vec::new(),
            score_popups: Vec::new(),
            events: Vec::new(),
            left_held: false,
            right_held: false,
            fire_held: false,
            shots_fired: 0,
            bonus_life_awarded: false,
            rng: Rng::new(seed),
            ticks: 0,
        };
        game.build_formation();
        game
    }

    /// Advances the simulation by exactly one 60 Hz tick.
    pub fn tick(&mut self) {
        self.ticks = self.ticks.saturating_add(1);
        if matches!(self.phase, Phase::Attract | Phase::GameOver) || self.paused {
            return;
        }

        match self.phase {
            Phase::Ready { ticks_left } => {
                if ticks_left <= 1 {
                    self.phase = Phase::Playing;
                } else {
                    self.phase = Phase::Ready {
                        ticks_left: ticks_left - 1,
                    };
                }
            }
            Phase::Playing => self.tick_playing(),
            Phase::PlayerDeath { ticks_left } => self.tick_player_death(ticks_left),
            Phase::WaveClear { ticks_left } => self.tick_wave_clear(ticks_left),
            Phase::Attract | Phase::GameOver => {}
        }
        self.tick_effects();
    }

    /// Applies a player intent.
    pub fn command(&mut self, command: Command) {
        match command {
            Command::Start => {
                if matches!(self.phase, Phase::Attract | Phase::GameOver) {
                    self.start_game();
                }
            }
            Command::Fire => self.try_fire(),
            Command::SetFire(held) => {
                self.fire_held = held;
                if held {
                    self.try_fire();
                }
            }
            Command::TogglePause => {
                if matches!(self.phase, Phase::Ready { .. } | Phase::Playing) {
                    self.paused = !self.paused;
                }
            }
            Command::SetLeft(held) => self.left_held = held,
            Command::SetRight(held) => self.right_held = held,
        }
    }

    /// Events queued by the most recent ticks.
    #[must_use]
    pub fn events(&self) -> &[GameEvent] {
        &self.events
    }

    /// Drains and returns queued events.
    pub fn take_events(&mut self) -> Vec<GameEvent> {
        std::mem::take(&mut self.events)
    }

    /// The current phase.
    #[must_use]
    pub const fn phase(&self) -> Phase {
        self.phase
    }

    /// Whether the player paused a live round.
    #[must_use]
    pub const fn is_paused(&self) -> bool {
        self.paused
    }

    /// Current score.
    #[must_use]
    pub const fn score(&self) -> u32 {
        self.score
    }

    /// Best score seen by this game, including the injected persistent value.
    #[must_use]
    pub const fn high_score(&self) -> u32 {
        self.high_score
    }

    /// Remaining lives.
    #[must_use]
    pub const fn lives(&self) -> u32 {
        self.lives
    }

    /// Current wave number, starting at 1.
    #[must_use]
    pub const fn level(&self) -> u32 {
        self.level
    }

    /// The player's cannon.
    #[must_use]
    pub const fn player(&self) -> Player {
        self.player
    }

    /// The player's in-flight bullet, if any.
    #[must_use]
    pub const fn player_bullet(&self) -> Option<Bullet> {
        self.player_bullet
    }

    /// Enemy bullets currently in flight.
    #[must_use]
    pub fn enemy_bullets(&self) -> &[Bullet] {
        &self.enemy_bullets
    }

    /// The full 55-slot formation, including dead members.
    #[must_use]
    pub fn aliens(&self) -> &[Alien] {
        &self.aliens
    }

    /// Number of invaders still alive.
    #[must_use]
    pub fn aliens_alive(&self) -> usize {
        self.aliens.iter().filter(|alien| alien.alive).count()
    }

    /// The four bunkers.
    #[must_use]
    pub const fn bunkers(&self) -> &[Bunker; BUNKER_COUNT] {
        &self.bunkers
    }

    /// The mystery ship, when present.
    #[must_use]
    pub const fn mystery_ship(&self) -> Option<MysteryShip> {
        self.mystery
    }

    /// Active explosions.
    #[must_use]
    pub fn explosions(&self) -> &[Explosion] {
        &self.explosions
    }

    /// Active mystery-ship score popups.
    #[must_use]
    pub fn score_popups(&self) -> &[ScorePopup] {
        &self.score_popups
    }

    /// The current horizontal formation direction, `1.0` or `-1.0`.
    #[must_use]
    pub const fn formation_direction(&self) -> f32 {
        self.direction
    }

    /// Alternating invader animation frame, 0 or 1.
    #[must_use]
    pub const fn animation_frame(&self) -> u8 {
        self.animation_frame
    }

    /// Number of shots the player has fired this round.
    #[must_use]
    pub const fn shots_fired(&self) -> u32 {
        self.shots_fired
    }

    /// Total ticks elapsed, including attract mode. The renderer uses this for
    /// purely decorative animation.
    #[must_use]
    pub const fn ticks(&self) -> u64 {
        self.ticks
    }

    /// Ticks left in the current timed phase.
    #[must_use]
    pub const fn phase_ticks_left(&self) -> u32 {
        match self.phase {
            Phase::Ready { ticks_left }
            | Phase::PlayerDeath { ticks_left }
            | Phase::WaveClear { ticks_left } => ticks_left,
            Phase::Attract | Phase::Playing | Phase::GameOver => 0,
        }
    }

    fn emit(&mut self, event: GameEvent) {
        if self.events.len() >= 256 {
            self.events.drain(..128);
        }
        self.events.push(event);
    }

    fn start_game(&mut self) {
        self.score = 0;
        self.lives = STARTING_LIVES;
        self.level = 1;
        self.shots_fired = 0;
        self.bonus_life_awarded = false;
        self.paused = false;
        self.left_held = false;
        self.right_held = false;
        self.player = Player::new();
        self.explosions.clear();
        self.score_popups.clear();
        self.enemy_bullets.clear();
        self.player_bullet = None;
        self.mystery = None;
        self.emit(GameEvent::GameStarted);
        self.begin_wave(90);
    }

    fn begin_wave(&mut self, ready_ticks: u32) {
        self.build_formation();
        self.bunkers = Bunker::standard_set();
        self.player = Player::new();
        self.player_bullet = None;
        self.enemy_bullets.clear();
        self.mystery = None;
        self.mystery_timer = self.rng.range(700, 1_200);
        self.fire_timer = self.rng.range(60, 120);
        self.direction = 1.0;
        self.step_timer = 0;
        self.animation_frame = 0;
        self.explosions.clear();
        self.score_popups.clear();
        self.phase = Phase::Ready {
            ticks_left: ready_ticks,
        };
        self.emit(GameEvent::WaveStarted);
    }

    fn build_formation(&mut self) {
        self.aliens.clear();
        let base_y = FORMATION_Y + (self.level.saturating_sub(1).min(4) as f32) * FORMATION_DROP;
        for row in 0..FORMATION_ROWS {
            let kind = match row {
                0 => AlienKind::Squid,
                1 | 2 => AlienKind::Crab,
                _ => AlienKind::Octopus,
            };
            for column in 0..FORMATION_COLUMNS {
                let centre = FORMATION_X + column as f32 * FORMATION_SPACING_X + 8.0;
                let y = base_y + row as f32 * FORMATION_SPACING_Y;
                self.aliens.push(Alien::new(
                    kind,
                    row,
                    column,
                    centre - kind.width() / 2.0,
                    y,
                ));
            }
        }
    }

    fn try_fire(&mut self) {
        if matches!(self.phase, Phase::Playing)
            && !self.paused
            && self.player.alive
            && self.player_bullet.is_none()
        {
            self.fire_player_bullet();
        }
    }

    fn tick_playing(&mut self) {
        self.move_player();
        self.move_player_bullet();
        self.move_enemy_bullets();
        if !matches!(self.phase, Phase::Playing) {
            return;
        }
        self.step_formation();
        if !matches!(self.phase, Phase::Playing) {
            return;
        }
        self.fire_enemy_bullet();
        self.move_mystery_ship();

        if self.aliens_alive() == 0 {
            self.phase = Phase::WaveClear { ticks_left: 120 };
            self.player_bullet = None;
            self.enemy_bullets.clear();
            self.mystery = None;
            self.emit(GameEvent::WaveCleared);
            return;
        }

        if self.fire_held {
            self.try_fire();
        }
    }

    fn move_player(&mut self) {
        if !self.player.alive {
            return;
        }
        let direction = match (self.left_held, self.right_held) {
            (true, false) => -1.0,
            (false, true) => 1.0,
            _ => 0.0,
        };
        let max_x = PLAY_RIGHT - PLAYER_WIDTH;
        self.player.x = (self.player.x + direction * PLAYER_SPEED).clamp(PLAY_LEFT, max_x);
    }

    fn fire_player_bullet(&mut self) {
        self.player_bullet = Some(Bullet {
            x: self.player.centre_x() - 0.5,
            y: self.player.y - 4.0,
            w: 1.0,
            h: 4.0,
            velocity: -PLAYER_BULLET_SPEED,
        });
        self.shots_fired = self.shots_fired.saturating_add(1);
        self.emit(GameEvent::PlayerShoot);
    }

    fn move_player_bullet(&mut self) {
        let Some(mut bullet) = self.player_bullet else {
            return;
        };
        bullet.y += bullet.velocity;
        if bullet.y + bullet.h < HUD_BOTTOM {
            self.player_bullet = None;
            return;
        }

        if self.damage_bunker_at(bullet.rect(), true) {
            self.spark(bullet.x, bullet.y);
            self.player_bullet = None;
            return;
        }

        let hit = self
            .aliens
            .iter_mut()
            .find(|alien| alien.alive && alien.rect().overlaps(bullet.rect()));
        if let Some(alien) = hit {
            alien.alive = false;
            let kind = alien.kind;
            let centre_x = alien.x + kind.width() / 2.0;
            let centre_y = alien.y + kind.height() / 2.0;
            let points = kind.points();
            self.explosions.push(Explosion {
                x: centre_x,
                y: centre_y,
                kind: ExplosionKind::Invader,
                ticks_left: 18,
                duration: 18,
            });
            self.award(points);
            self.emit(GameEvent::InvaderKilled { kind, points });
            self.player_bullet = None;
            return;
        }

        if let Some(ship) = self.mystery {
            let ship_rect = Rect::new(ship.x, MYSTERY_Y, MYSTERY_WIDTH, MYSTERY_HEIGHT);
            if ship_rect.overlaps(bullet.rect()) {
                self.score_popups.push(ScorePopup {
                    x: ship.x,
                    y: MYSTERY_Y - 9.0,
                    points: ship.points,
                    ticks_left: 60,
                });
                self.explosions.push(Explosion {
                    x: ship.x + MYSTERY_WIDTH / 2.0,
                    y: MYSTERY_Y + MYSTERY_HEIGHT / 2.0,
                    kind: ExplosionKind::Mystery,
                    ticks_left: 24,
                    duration: 24,
                });
                self.award(ship.points);
                self.emit(GameEvent::MysteryKilled {
                    points: ship.points,
                });
                self.mystery = None;
                self.player_bullet = None;
                return;
            }
        }

        self.player_bullet = Some(bullet);
    }

    fn move_enemy_bullets(&mut self) {
        let mut kept = Vec::with_capacity(self.enemy_bullets.len());
        for mut bullet in std::mem::take(&mut self.enemy_bullets) {
            bullet.y += ENEMY_BULLET_SPEED;
            if bullet.y > 232.0 {
                continue;
            }

            if let Some(player_bullet) = self.player_bullet {
                if player_bullet.rect().overlaps(bullet.rect()) {
                    self.spark(bullet.x, bullet.y);
                    self.player_bullet = None;
                    continue;
                }
            }

            if self.damage_bunker_at(bullet.rect(), false) {
                self.spark(bullet.x, bullet.y);
                continue;
            }

            if self.player.alive && self.player.rect().inset(1.0).overlaps(bullet.rect()) {
                self.hit_player();
                return;
            }
            kept.push(bullet);
        }
        self.enemy_bullets = kept;
    }

    fn damage_bunker_at(&mut self, projectile: Rect, rising: bool) -> bool {
        for bunker in &mut self.bunkers {
            if let Some((col, row)) = bunker.impact(projectile, rising) {
                bunker.carve(col, row, &mut self.rng);
                return true;
            }
        }
        false
    }

    fn spark(&mut self, x: f32, y: f32) {
        self.explosions.push(Explosion {
            x,
            y,
            kind: ExplosionKind::Spark,
            ticks_left: 8,
            duration: 8,
        });
    }

    fn step_formation(&mut self) {
        if self.aliens_alive() == 0 {
            return;
        }
        self.step_timer += 1;
        let interval = self.formation_interval();
        if self.step_timer < interval {
            return;
        }
        self.step_timer = 0;

        let mut min_x = f32::MAX;
        let mut max_x = f32::MIN;
        for alien in self.aliens.iter().filter(|alien| alien.alive) {
            min_x = min_x.min(alien.x);
            max_x = max_x.max(alien.x + alien.kind.width());
        }
        let dx = 2.0 * self.direction;
        let would_cross = min_x + dx < PLAY_LEFT || max_x + dx > PLAY_RIGHT;
        let drop = if would_cross {
            self.direction = -self.direction;
            FORMATION_DROP
        } else {
            0.0
        };
        let shift = if would_cross { 0.0 } else { dx };

        let mut landed = false;
        for alien in self.aliens.iter_mut().filter(|alien| alien.alive) {
            alien.x += shift;
            alien.y += drop;
            if alien.y + alien.kind.height() >= PLAYER_Y {
                landed = true;
            }
        }
        self.animation_frame ^= 1;
        self.emit(GameEvent::AlienStep);

        if drop > 0.0 {
            for alien in self.aliens.iter().filter(|alien| alien.alive) {
                let rect = alien.rect();
                for bunker in &mut self.bunkers {
                    let bunker_rect = Rect::new(
                        bunker.x(),
                        bunker.y(),
                        bunker.width() as f32,
                        bunker.height() as f32,
                    );
                    if rect.overlaps(bunker_rect) {
                        bunker.crush(rect);
                    }
                }
            }
        }

        if landed {
            self.end_round();
        }
    }

    fn formation_interval(&self) -> u32 {
        let alive = self.aliens_alive() as f32;
        let level_speedup = self.level.saturating_sub(1).min(6) * 3;
        let base = (alive * 0.58 + 2.0).round().clamp(3.0, 36.0) as u32;
        base.saturating_sub(level_speedup).max(2)
    }

    fn fire_enemy_bullet(&mut self) {
        if self.fire_timer > 0 {
            self.fire_timer -= 1;
            return;
        }
        let max_bullets = BASE_ENEMY_BULLETS + (self.level.saturating_sub(1) as usize / 2);
        if self.enemy_bullets.len() >= max_bullets.min(5) {
            self.fire_timer = 30;
            return;
        }
        if let Some((x, y)) = self.choose_shooter() {
            self.enemy_bullets.push(Bullet {
                x,
                y,
                w: 1.0,
                h: 4.0,
                velocity: ENEMY_BULLET_SPEED,
            });
            self.emit(GameEvent::EnemyShoot);
        }
        let base = self.rng.range(35, 95);
        let level_penalty = self.level.saturating_sub(1).min(5) * 3;
        self.fire_timer = base.saturating_sub(level_penalty).max(14);
    }

    fn choose_shooter(&mut self) -> Option<(f32, f32)> {
        for _ in 0..32 {
            let column = self.rng.range(0, FORMATION_COLUMNS as u32) as usize;
            let mut lowest: Option<&Alien> = None;
            for alien in self
                .aliens
                .iter()
                .filter(|alien| alien.alive && alien.column == column)
            {
                if lowest.is_none_or(|current| alien.y > current.y) {
                    lowest = Some(alien);
                }
            }
            if let Some(alien) = lowest {
                return Some((
                    alien.x + alien.kind.width() / 2.0 - 0.5,
                    alien.y + alien.kind.height(),
                ));
            }
        }
        None
    }

    fn move_mystery_ship(&mut self) {
        if let Some(mut ship) = self.mystery {
            ship.x += ship.direction * MYSTERY_SPEED;
            if ship.x + MYSTERY_WIDTH < 0.0 || ship.x > SCREEN_WIDTH {
                self.mystery = None;
            } else {
                self.mystery = Some(ship);
            }
            return;
        }

        if self.mystery_timer > 0 {
            self.mystery_timer -= 1;
            return;
        }
        let from_left = self.rng.next_u32() & 1 == 0;
        let slot = self.shots_fired % 15;
        let points = match slot {
            14 => 300,
            10..=13 => 150,
            5..=9 => 100,
            _ => 50,
        };
        self.mystery = Some(MysteryShip {
            x: if from_left {
                -MYSTERY_WIDTH
            } else {
                SCREEN_WIDTH
            },
            direction: if from_left { 1.0 } else { -1.0 },
            points,
        });
        self.mystery_timer = self.rng.range(700, 1_300);
        self.emit(GameEvent::MysteryAppeared);
    }

    fn hit_player(&mut self) {
        self.lives = self.lives.saturating_sub(1);
        self.player.alive = false;
        self.player_bullet = None;
        self.enemy_bullets.clear();
        self.explosions.push(Explosion {
            x: self.player.centre_x(),
            y: self.player.y + PLAYER_HEIGHT / 2.0,
            kind: ExplosionKind::Player,
            ticks_left: 80,
            duration: 80,
        });
        self.phase = Phase::PlayerDeath { ticks_left: 90 };
        self.emit(GameEvent::PlayerHit);
    }

    fn tick_player_death(&mut self, ticks_left: u32) {
        if ticks_left > 1 {
            self.phase = Phase::PlayerDeath {
                ticks_left: ticks_left - 1,
            };
            return;
        }
        if self.lives == 0 {
            self.end_round();
        } else {
            self.player = Player::new();
            self.enemy_bullets.clear();
            self.phase = Phase::Ready { ticks_left: 60 };
        }
    }

    fn tick_wave_clear(&mut self, ticks_left: u32) {
        if ticks_left > 1 {
            self.phase = Phase::WaveClear {
                ticks_left: ticks_left - 1,
            };
            return;
        }
        self.level = self.level.saturating_add(1);
        self.begin_wave(60);
    }

    fn end_round(&mut self) {
        self.phase = Phase::GameOver;
        self.paused = false;
        self.player.alive = false;
        self.player_bullet = None;
        self.enemy_bullets.clear();
        self.mystery = None;
        self.emit(GameEvent::GameOver);
    }

    fn award(&mut self, points: u32) {
        self.score = self.score.saturating_add(points);
        if self.score > self.high_score {
            self.high_score = self.score;
        }
        if !self.bonus_life_awarded && self.score >= BONUS_LIFE_SCORE {
            self.bonus_life_awarded = true;
            self.lives = self.lives.saturating_add(1);
            self.emit(GameEvent::ExtraLife);
        }
    }

    fn tick_effects(&mut self) {
        for effect in &mut self.explosions {
            effect.ticks_left = effect.ticks_left.saturating_sub(1);
        }
        self.explosions.retain(|effect| effect.ticks_left > 0);

        for popup in &mut self.score_popups {
            popup.ticks_left = popup.ticks_left.saturating_sub(1);
            popup.y -= 0.25;
        }
        self.score_popups.retain(|popup| popup.ticks_left > 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::BUNKER_COUNT;

    fn started_game() -> Game {
        let mut game = Game::with_seed(0, 0x1978);
        game.command(Command::Start);
        for _ in 0..95 {
            game.tick();
            let _ = game.take_events();
        }
        assert_eq!(game.phase(), Phase::Playing);
        game
    }

    fn kill_all_aliens(game: &mut Game) {
        for alien in &mut game.aliens {
            alien.alive = false;
        }
    }

    #[test]
    fn attract_ignores_gameplay_input_until_start() {
        let mut game = Game::with_seed(0, 1);
        game.command(Command::Fire);
        game.command(Command::SetLeft(true));
        for _ in 0..30 {
            game.tick();
        }
        assert_eq!(game.phase(), Phase::Attract);
        assert!(game.player_bullet().is_none());
        assert_eq!(game.aliens_alive(), 55);
    }

    fn count_shots(game: &mut Game) -> usize {
        game.take_events()
            .iter()
            .filter(|event| matches!(event, GameEvent::PlayerShoot))
            .count()
    }

    #[test]
    fn holding_fire_relaunches_the_bullet() {
        let mut game = started_game();
        game.command(Command::SetFire(true));
        let mut shots = 0;
        for _ in 0..300 {
            if matches!(game.phase(), Phase::PlayerDeath { .. } | Phase::GameOver) {
                break;
            }
            game.tick();
            shots += count_shots(&mut game);
        }
        assert!(shots >= 2, "held fire should reload, got {shots} shots");
    }

    #[test]
    fn releasing_fire_stops_after_the_bullet_leaves() {
        let mut game = started_game();
        game.command(Command::SetFire(true));
        for _ in 0..120 {
            game.tick();
            let _ = game.take_events();
        }
        game.command(Command::SetFire(false));
        while game.player_bullet().is_some() {
            game.tick();
            let _ = game.take_events();
        }
        let mut shots = 0;
        for _ in 0..120 {
            game.tick();
            shots += count_shots(&mut game);
        }
        assert_eq!(shots, 0, "released fire must not reload");
    }

    #[test]
    fn paused_held_fire_does_not_shoot() {
        let mut game = started_game();
        game.command(Command::TogglePause);
        game.command(Command::SetFire(true));
        for _ in 0..60 {
            game.tick();
        }
        assert!(game.player_bullet().is_none());
    }

    #[test]
    fn start_sets_up_the_canonical_formation() {
        let game = started_game();
        assert_eq!(game.aliens().len(), 55);
        assert_eq!(game.aliens_alive(), 55);
        assert_eq!(game.lives(), 3);
        assert_eq!(game.level(), 1);
        assert_eq!(game.bunkers().len(), BUNKER_COUNT);
        assert_eq!(
            game.aliens()
                .iter()
                .filter(|a| a.kind == AlienKind::Squid)
                .count(),
            11
        );
        assert_eq!(
            game.aliens()
                .iter()
                .filter(|a| a.kind == AlienKind::Crab)
                .count(),
            22
        );
        assert_eq!(
            game.aliens()
                .iter()
                .filter(|a| a.kind == AlienKind::Octopus)
                .count(),
            22
        );
    }

    #[test]
    fn player_moves_and_is_clamped() {
        let mut game = started_game();
        game.command(Command::SetLeft(true));
        for _ in 0..200 {
            game.tick();
        }
        assert!((game.player().x - PLAY_LEFT).abs() < f32::EPSILON);
        game.command(Command::SetLeft(false));
        game.command(Command::SetRight(true));
        for _ in 0..200 {
            game.tick();
        }
        assert!((game.player().rect().right() - PLAY_RIGHT).abs() < f32::EPSILON);
    }

    #[test]
    fn player_bullet_destroys_an_invader_and_scores() {
        let mut game = started_game();
        let target = game
            .aliens
            .iter()
            .filter(|alien| alien.alive && alien.column == 5)
            .max_by(|a, b| a.y.total_cmp(&b.y))
            .copied()
            .unwrap();
        game.player.x = target.x + target.kind.width() / 2.0 - PLAYER_WIDTH / 2.0;
        game.command(Command::Fire);
        assert!(game.player_bullet().is_some());
        for _ in 0..80 {
            game.tick();
            let events = game.take_events();
            if events
                .iter()
                .any(|event| matches!(event, GameEvent::InvaderKilled { .. }))
            {
                break;
            }
        }
        assert_eq!(game.aliens_alive(), 54);
        assert_eq!(game.score(), target.kind.points());
        assert!(game.high_score() >= game.score());
    }

    #[test]
    fn bunkers_absorb_and_remember_damage() {
        let mut game = started_game();
        let intact_before: usize = game.bunkers.iter().map(Bunker::intact_cells).sum();
        game.player.x = 24.0 + 10.0 - PLAYER_WIDTH / 2.0;
        game.command(Command::Fire);
        for _ in 0..20 {
            game.tick();
        }
        let intact_after: usize = game.bunkers.iter().map(Bunker::intact_cells).sum();
        assert!(intact_after < intact_before, "the bunker should be chipped");
        assert!(game.player_bullet().is_none());
    }

    #[test]
    fn formation_advances_drops_and_reverses() {
        let mut game = started_game();
        let start_y = game.aliens()[0].y;
        let start_min = game
            .aliens
            .iter()
            .filter(|a| a.alive)
            .map(|a| a.x)
            .fold(f32::MAX, f32::min);
        let mut reversed = false;
        let mut dropped = false;
        for _ in 0..600 {
            game.tick();
            let _ = game.take_events();
            if game.formation_direction() < 0.0 {
                reversed = true;
            }
            if game.aliens()[0].y > start_y + 1.0 {
                dropped = true;
                break;
            }
        }
        assert!(reversed, "formation must bounce off a wall");
        assert!(dropped, "formation must step down after a bounce");
        assert!(game.aliens()[0].x >= start_min - 1.0);
    }

    #[test]
    fn pause_freezes_the_world_but_not_the_pause_toggle() {
        let mut game = started_game();
        game.command(Command::TogglePause);
        assert!(game.is_paused());
        let tick = game.ticks();
        let x = game.player().x;
        game.command(Command::SetRight(true));
        for _ in 0..60 {
            game.tick();
        }
        assert_eq!(game.ticks(), tick + 60);
        assert!((game.player().x - x).abs() < f32::EPSILON);
        game.command(Command::TogglePause);
        assert!(!game.is_paused());
    }

    #[test]
    fn player_death_costs_a_life_then_readies_again() {
        let mut game = started_game();
        game.enemy_bullets.push(Bullet {
            x: game.player().x + 5.0,
            y: game.player().y,
            w: 1.0,
            h: 4.0,
            velocity: ENEMY_BULLET_SPEED,
        });
        game.tick();
        assert!(matches!(game.phase(), Phase::PlayerDeath { .. }));
        assert_eq!(game.lives(), 2);
        assert!(!game.player().alive);
        for _ in 0..100 {
            game.tick();
        }
        assert!(matches!(game.phase(), Phase::Ready { .. }));
        assert!(game.player().alive);
    }

    #[test]
    fn three_hits_end_the_round() {
        let mut game = started_game();
        for expected_lives in [2, 1, 0] {
            game.enemy_bullets.push(Bullet {
                x: game.player().x + 5.0,
                y: game.player().y,
                w: 1.0,
                h: 4.0,
                velocity: ENEMY_BULLET_SPEED,
            });
            game.tick();
            assert_eq!(game.lives(), expected_lives);
            for _ in 0..120 {
                game.tick();
            }
            if expected_lives > 0 {
                for _ in 0..70 {
                    game.tick();
                }
            }
        }
        assert_eq!(game.phase(), Phase::GameOver);
        game.command(Command::Start);
        assert_eq!(game.phase(), Phase::Ready { ticks_left: 90 });
        assert_eq!(game.lives(), 3);
        assert_eq!(game.score(), 0);
    }

    #[test]
    fn clearing_a_wave_starts_the_next_one_faster_and_lower() {
        let mut game = started_game();
        let old_y = game.aliens()[0].y;
        let old_interval = game.formation_interval();
        kill_all_aliens(&mut game);
        game.tick();
        assert!(matches!(game.phase(), Phase::WaveClear { .. }));
        for _ in 0..121 {
            game.tick();
        }
        assert_eq!(game.level(), 2);
        assert_eq!(game.aliens_alive(), 55);
        assert!(game.aliens()[0].y > old_y);
        assert!(game.formation_interval() < old_interval);
    }

    #[test]
    fn extra_life_is_awarded_once() {
        let mut game = started_game();
        game.award(BONUS_LIFE_SCORE);
        assert_eq!(game.lives(), 4);
        game.award(0);
        assert_eq!(game.lives(), 4);
    }

    #[test]
    fn mystery_ship_can_be_summoned_and_shot() {
        let mut game = started_game();
        game.mystery_timer = 0;
        game.tick();
        assert!(game.mystery_ship().is_some());
        let ship = game.mystery_ship().unwrap();
        game.player_bullet = Some(Bullet {
            x: ship.x + 4.0,
            y: MYSTERY_Y + 1.0,
            w: 1.0,
            h: 4.0,
            velocity: -PLAYER_BULLET_SPEED,
        });
        game.tick();
        assert!(game.mystery_ship().is_none());
        assert!(game.score() >= 50);
        assert!(
            game.events()
                .iter()
                .any(|event| matches!(event, GameEvent::MysteryKilled { .. }))
        );
    }

    #[test]
    fn invaders_reaching_the_cannon_end_the_round() {
        let mut game = started_game();
        for alien in &mut game.aliens {
            if alien.alive {
                alien.y = PLAYER_Y - 8.0;
            }
        }
        game.step_timer = 100;
        game.tick();
        assert_eq!(game.phase(), Phase::GameOver);
    }

    #[test]
    fn enemy_fire_spawns_bullets_and_they_hit_cover() {
        let mut game = started_game();
        let before: usize = game.bunkers.iter().map(Bunker::intact_cells).sum();
        game.fire_timer = 0;
        for _ in 0..2_000 {
            game.tick();
            let _ = game.take_events();
            if !game.enemy_bullets().is_empty() {
                break;
            }
        }
        assert!(!game.enemy_bullets().is_empty());
        let after: usize = game.bunkers.iter().map(Bunker::intact_cells).sum();
        assert!(after <= before);
    }
}
