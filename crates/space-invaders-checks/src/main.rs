//! Standalone behavioral checks for the headless simulation.
//!
//! This executable deliberately depends only on `space-invaders-core`. It
//! exercises the public command/tick/read API exactly as the app does, which
//! catches integration mistakes that unit tests with private access can miss.
//! Run it with `make checks`.

use std::fmt::Debug;
use std::process::ExitCode;

use space_invaders_core::{
    AlienKind, Bunker, Command, Game, GameEvent, Phase, SCREEN_HEIGHT, SCREEN_WIDTH,
};

struct Checker {
    passed: u32,
    failed: u32,
}

impl Checker {
    fn new() -> Self {
        Self {
            passed: 0,
            failed: 0,
        }
    }

    fn check(&mut self, condition: bool, message: impl AsRef<str>) {
        if condition {
            self.passed += 1;
        } else {
            self.failed += 1;
            eprintln!("FAIL: {}", message.as_ref());
        }
    }

    // Values are only formatted on failure; passing them by value keeps the
    // call sites in this harness readable.
    #[allow(clippy::needless_pass_by_value)]
    fn eq<T: PartialEq + Debug>(&mut self, left: T, right: T, message: impl AsRef<str>) {
        if left == right {
            self.passed += 1;
        } else {
            self.failed += 1;
            eprintln!(
                "FAIL: {} (left {:?}, right {:?})",
                message.as_ref(),
                left,
                right
            );
        }
    }
}

fn started_game(seed: u64) -> Game {
    let mut game = Game::with_seed(0, seed);
    game.command(Command::Start);
    for _ in 0..95 {
        game.tick();
        let _ = game.take_events();
    }
    game
}

fn centre_of_lowest_alien(game: &Game, column: usize) -> Option<f32> {
    game.aliens()
        .iter()
        .filter(|alien| alien.alive && alien.column == column)
        .max_by(|a, b| a.y.total_cmp(&b.y))
        .map(|alien| alien.x + alien.kind.width() / 2.0)
}

fn ensure_playing(game: &mut Game) {
    for _ in 0..800 {
        match game.phase() {
            Phase::Playing => return,
            Phase::Attract | Phase::GameOver => game.command(Command::Start),
            Phase::Ready { .. } | Phase::WaveClear { .. } | Phase::PlayerDeath { .. } => {}
        }
        game.tick();
        let _ = game.take_events();
    }
}

fn aim_at(game: &mut Game, target_centre: f32) {
    let mut guard = 0;
    loop {
        let player_centre = game.player().centre_x();
        let delta = target_centre - player_centre;
        game.command(Command::SetLeft(delta < -1.0));
        game.command(Command::SetRight(delta > 1.0));
        if delta.abs() <= 1.0 || guard > 200 {
            break;
        }
        game.tick();
        let _ = game.take_events();
        guard += 1;
    }
    game.command(Command::SetLeft(false));
    game.command(Command::SetRight(false));
}

fn shoot_column(game: &mut Game, column: usize) -> bool {
    let before = game.aliens_alive();
    for _ in 0..6 {
        ensure_playing(game);
        let Some(target) = centre_of_lowest_alien(game, column) else {
            return false;
        };
        aim_at(game, target);
        ensure_playing(game);
        game.command(Command::Fire);
        for _ in 0..90 {
            game.tick();
            let _ = game.take_events();
            if game.aliens_alive() < before {
                return true;
            }
            if matches!(game.phase(), Phase::PlayerDeath { .. } | Phase::GameOver) {
                break;
            }
        }
    }
    false
}

fn main() -> ExitCode {
    let mut c = Checker::new();

    // Layout and attract mode.
    c.eq(SCREEN_WIDTH, 224.0, "logical screen width");
    c.eq(SCREEN_HEIGHT, 256.0, "logical screen height");
    let mut game = Game::with_seed(12_345, 0x1978);
    c.eq(game.phase(), Phase::Attract, "starts in attract mode");
    game.command(Command::Start);
    c.check(
        matches!(game.phase(), Phase::Ready { .. }),
        "Start enters the ready phase",
    );
    c.eq(game.score(), 0, "new round starts at zero score");
    c.eq(game.lives(), 3, "new round starts with three lives");
    c.eq(game.level(), 1, "new round starts on wave 1");
    c.eq(game.aliens().len(), 55, "55 invaders are allocated");
    c.eq(game.aliens_alive(), 55, "55 invaders are alive");
    c.eq(game.bunkers().len(), 4, "four bunkers exist");

    // The canonical formation composition and point values.
    let mut squids = 0;
    let mut crabs = 0;
    let mut octopuses = 0;
    for alien in game.aliens() {
        match alien.kind {
            AlienKind::Squid => squids += 1,
            AlienKind::Crab => crabs += 1,
            AlienKind::Octopus => octopuses += 1,
        }
        c.check(alien.row < 5, "alien row is inside the formation");
        c.check(alien.column < 11, "alien column is inside the formation");
        c.check(alien.alive, "freshly spawned alien is alive");
        c.check(
            alien.x >= 0.0 && alien.x + alien.kind.width() <= SCREEN_WIDTH,
            "alien starts inside the screen",
        );
        c.check(
            alien.y >= 0.0 && alien.y < 160.0,
            "alien starts above cover",
        );
    }
    c.eq(squids, 11, "11 squid");
    c.eq(crabs, 22, "22 crab");
    c.eq(octopuses, 22, "22 octopus");
    c.eq(AlienKind::Squid.points(), 30, "squid points");
    c.eq(AlienKind::Crab.points(), 20, "crab points");
    c.eq(AlienKind::Octopus.points(), 10, "octopus points");

    // Bunker cell geometry.
    for bunker in game.bunkers() {
        c.eq(bunker.width(), 22, "bunker width");
        c.eq(bunker.height(), 16, "bunker height");
        c.check(bunker.intact_cells() > 200, "bunker is mostly intact");
        let mut cell_count = 0;
        for row in 0..bunker.height() {
            for col in 0..bunker.width() {
                cell_count += usize::from(bunker.cell(col, row));
                c.check(col < 22 && row < 16, "bunker cell is in bounds");
            }
        }
        c.eq(cell_count, bunker.intact_cells(), "cell count matches grid");
    }

    // The roof opening and doorway survive the procedural construction.
    let bunker: Bunker = game.bunkers()[0];
    c.check(!bunker.cell(0, 0), "bunker roof corner is rounded");
    c.check(bunker.cell(11, 2), "bunker roof centre is solid");
    c.check(!bunker.cell(11, 15), "bunker doorway is open");

    // Ready state advances to play and movement is clamped.
    for _ in 0..95 {
        game.tick();
        let _ = game.take_events();
    }
    c.eq(game.phase(), Phase::Playing, "ready advances to playing");
    game.command(Command::SetLeft(true));
    for _ in 0..150 {
        game.tick();
        let _ = game.take_events();
    }
    c.check(game.player().x >= 8.0, "player stays inside the left wall");
    game.command(Command::SetLeft(false));
    game.command(Command::SetRight(true));
    for _ in 0..150 {
        game.tick();
        let _ = game.take_events();
    }
    c.check(
        game.player().rect().right() <= 216.0,
        "player stays inside the right wall",
    );
    game.command(Command::SetRight(false));

    // Aim, fire, and verify scoring and destruction.
    let before_alive = game.aliens_alive();
    let before_score = game.score();
    c.check(shoot_column(&mut game, 5), "a bullet destroys an invader");
    c.check(
        game.aliens_alive() < before_alive,
        "formation loses an invader",
    );
    c.check(game.score() > before_score, "destroying an invader scores");
    c.check(
        game.high_score() >= game.score(),
        "high score tracks the current score",
    );

    // Bunker damage from the player's own shot.
    let intact_before: usize = game.bunkers().iter().map(Bunker::intact_cells).sum();
    let bunker_x = game.bunkers()[0].x() + 8.0;
    ensure_playing(&mut game);
    aim_at(&mut game, bunker_x);
    ensure_playing(&mut game);
    for _ in 0..80 {
        game.tick();
        let _ = game.take_events();
        if game.player_bullet().is_none() {
            break;
        }
    }
    game.command(Command::Fire);
    for _ in 0..30 {
        game.tick();
        let _ = game.take_events();
    }
    let intact_after: usize = game.bunkers().iter().map(Bunker::intact_cells).sum();
    c.check(intact_after < intact_before, "player fire damages a bunker");

    // Pause freezes the simulation.
    game.command(Command::TogglePause);
    c.check(game.is_paused(), "pause sets the pause flag");
    let frozen_x = game.player().x;
    let frozen_score = game.score();
    game.command(Command::SetRight(true));
    for _ in 0..90 {
        game.tick();
    }
    c.check(
        (game.player().x - frozen_x).abs() < f32::EPSILON,
        "paused player does not move",
    );
    c.eq(game.score(), frozen_score, "paused score does not change");
    game.command(Command::TogglePause);
    c.check(!game.is_paused(), "pause toggles back off");

    // Holding fire reloads the cannon without further commands.
    let mut held = started_game(0x5EED);
    ensure_playing(&mut held);
    held.command(Command::SetFire(true));
    let mut held_shots = 0;
    for _ in 0..300 {
        held.tick();
        held_shots += held
            .take_events()
            .iter()
            .filter(|event| matches!(event, GameEvent::PlayerShoot))
            .count();
        if matches!(held.phase(), Phase::PlayerDeath { .. } | Phase::GameOver) {
            break;
        }
    }
    c.check(held_shots >= 2, "held fire reloads the cannon");

    // Releasing the button stops the reloads once the live bullet leaves.
    held.command(Command::SetFire(false));
    let mut settle = 0;
    while held.player_bullet().is_some() && settle < 200 {
        held.tick();
        let _ = held.take_events();
        settle += 1;
    }
    let mut shots_after_release = 0;
    for _ in 0..120 {
        held.tick();
        shots_after_release += held
            .take_events()
            .iter()
            .filter(|event| matches!(event, GameEvent::PlayerShoot))
            .count();
    }
    c.check(
        shots_after_release == 0,
        "released fire stops shooting after the live bullet",
    );

    // Enemy fire is produced and stays inside the screen.
    let mut saw_enemy_bullet = false;
    for _ in 0..2_000 {
        game.tick();
        let events = game.take_events();
        if events.iter().any(|e| matches!(e, GameEvent::EnemyShoot)) {
            saw_enemy_bullet = true;
        }
        for bullet in game.enemy_bullets() {
            c.check(
                bullet.x >= 0.0 && bullet.x <= SCREEN_WIDTH,
                "enemy bullet stays horizontally inside",
            );
            c.check(bullet.y >= 0.0, "enemy bullet starts below the HUD");
        }
    }
    c.check(saw_enemy_bullet, "enemy fire occurs");

    // A passive round eventually ends when the formation reaches the cannon.
    let mut wave_game = started_game(0x5151);
    let mut saw_game_over = false;
    for _ in 0..40_000 {
        wave_game.tick();
        let events = wave_game.take_events();
        if events.iter().any(|e| matches!(e, GameEvent::GameOver)) {
            saw_game_over = true;
            break;
        }
    }
    c.check(
        saw_game_over,
        "the formation eventually ends a passive round",
    );
    c.eq(
        wave_game.phase(),
        Phase::GameOver,
        "landing reaches game over",
    );
    wave_game.command(Command::Start);
    c.check(
        matches!(wave_game.phase(), Phase::Ready { .. }),
        "Start restarts after game over",
    );
    c.eq(wave_game.score(), 0, "restart clears the score");
    c.eq(wave_game.lives(), 3, "restart restores lives");
    c.eq(wave_game.level(), 1, "restart returns to wave 1");

    // Determinism from an explicit seed.
    let mut a = started_game(0xABCD);
    let mut b = started_game(0xABCD);
    for _ in 0..600 {
        a.tick();
        b.tick();
        let _ = a.take_events();
        let _ = b.take_events();
    }
    c.eq(a.score(), b.score(), "same seed produces the same score");
    c.eq(a.lives(), b.lives(), "same seed produces the same lives");
    c.eq(
        a.aliens_alive(),
        b.aliens_alive(),
        "same seed preserves formation",
    );
    c.eq(a.phase(), b.phase(), "same seed produces the same phase");
    for (left, right) in a.aliens().iter().zip(b.aliens()) {
        c.check(
            (left.x - right.x).abs() < f32::EPSILON && (left.y - right.y).abs() < f32::EPSILON,
            "same seed produces identical alien positions",
        );
    }

    // Long-running soak with periodic restarts. This catches NaN, runaway
    // coordinates, and state-machine panics under sustained input.
    let mut soak = started_game(0x1978_0001);
    for frame in 0..7_200 {
        if matches!(soak.phase(), Phase::GameOver) {
            soak.command(Command::Start);
        }
        if frame % 37 == 0 {
            soak.command(Command::SetLeft(frame % 74 < 37));
            soak.command(Command::SetRight(frame % 74 >= 37));
        }
        if frame % 29 == 0 {
            soak.command(Command::Fire);
        }
        soak.tick();
        let _ = soak.take_events();
        if frame % 60 == 0 {
            for alien in soak.aliens().iter().filter(|alien| alien.alive) {
                c.check(
                    alien.x.is_finite()
                        && alien.y.is_finite()
                        && alien.x >= 0.0
                        && alien.x <= SCREEN_WIDTH,
                    "soak alien position remains finite and on screen",
                );
            }
            c.check(soak.score() <= 100_000, "soak score stays plausible");
            c.check(soak.lives() <= 20, "soak lives stay bounded");
        }
    }
    c.check(soak.ticks() >= 7_200, "soak ran for the requested ticks");

    println!();
    if c.failed == 0 {
        println!("PASS: {} checks", c.passed);
        ExitCode::SUCCESS
    } else {
        println!("FAILED: {} of {} checks", c.failed, c.passed + c.failed);
        ExitCode::FAILURE
    }
}
