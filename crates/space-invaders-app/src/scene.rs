//! Frame composition: turns a simulation snapshot into triangles.
//!
//! The scene layer is deliberately stateless. It may read the game and its
//! event-driven effects, but it never advances the simulation or caches
//! gameplay state. All art is procedural and all text uses the built-in 5 x 7
//! font.

use space_invaders_core::{
    AlienKind, Bullet, Explosion, ExplosionKind, Game, MysteryShip, Phase, SCREEN_HEIGHT,
    SCREEN_WIDTH,
};

use crate::canvas::{Canvas, rgba};
use crate::pixel_font::{draw_text, draw_text_centred, draw_text_right};
use crate::sprites;

const WHITE: [f32; 4] = rgba(232, 242, 255, 1.0);
const DIM_WHITE: [f32; 4] = rgba(126, 145, 162, 1.0);
const GREEN: [f32; 4] = rgba(64, 255, 128, 1.0);
const BUNKER: [f32; 4] = rgba(46, 230, 104, 1.0);
const BUNKER_DARK: [f32; 4] = rgba(26, 156, 70, 1.0);
const PLAYER_COLOR: [f32; 4] = rgba(120, 255, 190, 1.0);
const SQUID_COLOR: [f32; 4] = rgba(122, 238, 255, 1.0);
const CRAB_COLOR: [f32; 4] = rgba(255, 92, 210, 1.0);
const OCTOPUS_COLOR: [f32; 4] = rgba(196, 255, 74, 1.0);
const MYSTERY_COLOR: [f32; 4] = rgba(255, 72, 84, 1.0);
const BULLET_COLOR: [f32; 4] = rgba(235, 255, 210, 1.0);
const ENEMY_BULLET_COLOR: [f32; 4] = rgba(255, 110, 96, 1.0);
const YELLOW: [f32; 4] = rgba(255, 214, 74, 1.0);

/// Builds a complete frame.
#[must_use]
pub fn build(game: &Game, muted: bool) -> Canvas {
    let mut canvas = Canvas::new();
    draw_starfield(&mut canvas, game.ticks());

    if game.phase() == Phase::Attract {
        draw_attract(&mut canvas, game, muted);
        draw_scanlines(&mut canvas);
        return canvas;
    }

    draw_playfield(&mut canvas, game);
    draw_hud(&mut canvas, game, muted);

    match game.phase() {
        Phase::Ready { .. } => {
            if blink(game.ticks(), 24) {
                draw_text_centred(&mut canvas, 112.0, 154.0, "READY", 2, GREEN);
            }
        }
        Phase::WaveClear { .. } => {
            draw_text_centred(&mut canvas, 112.0, 154.0, "WAVE CLEAR", 2, YELLOW);
        }
        Phase::GameOver => {
            canvas.rect(0.0, 0.0, SCREEN_WIDTH, SCREEN_HEIGHT, rgba(0, 0, 0, 0.58));
            canvas.rect(14.0, 80.0, 196.0, 110.0, rgba(2, 5, 10, 0.92));
            canvas.outline(14.0, 80.0, 196.0, 110.0, rgba(255, 72, 84, 0.55));
            draw_text_centred(&mut canvas, 112.0, 92.0, "GAME OVER", 3, MYSTERY_COLOR);
            let score = format!("SCORE {:05}", game.score());
            draw_text_centred(&mut canvas, 112.0, 132.0, &score, 1, WHITE);
            draw_text_centred(
                &mut canvas,
                112.0,
                154.0,
                "PRESS ENTER TO PLAY AGAIN",
                1,
                GREEN,
            );
            draw_text_centred(&mut canvas, 112.0, 170.0, "DEFEND THE EARTH", 1, DIM_WHITE);
        }
        _ => {}
    }

    if game.is_paused() {
        canvas.rect(0.0, 0.0, SCREEN_WIDTH, SCREEN_HEIGHT, rgba(0, 0, 0, 0.55));
        draw_text_centred(&mut canvas, 112.0, 108.0, "PAUSED", 3, WHITE);
        draw_text_centred(&mut canvas, 112.0, 146.0, "PRESS P TO RESUME", 1, GREEN);
        draw_text_centred(
            &mut canvas,
            112.0,
            160.0,
            if muted { "M SOUND OFF" } else { "M SOUND ON" },
            1,
            DIM_WHITE,
        );
    }

    draw_scanlines(&mut canvas);
    canvas
}

fn draw_starfield(canvas: &mut Canvas, ticks: u64) {
    canvas.rect(0.0, 0.0, SCREEN_WIDTH, SCREEN_HEIGHT, rgba(3, 5, 11, 1.0));
    for index in 0..26_u64 {
        let x = ((index * 53 + 17) % 224) as f32;
        let y = ((index * 97 + 31) % 230) as f32 + 22.0;
        let phase = (ticks / 6 + index * 3) % 12;
        let (alpha, size) = if phase < 2 {
            (0.6, 1.0)
        } else if phase < 5 {
            (0.35, 1.0)
        } else {
            (0.16, 1.0)
        };
        let color = if index % 5 == 0 {
            rgba(122, 180, 255, alpha)
        } else {
            rgba(210, 225, 255, alpha)
        };
        canvas.rect(x, y, size, size, color);
    }
    // A faint horizon glow anchors the playfield without competing with actors.
    for band in 0..8 {
        let alpha = 0.012 * (8 - band) as f32;
        canvas.rect(
            0.0,
            band as f32 * 4.0,
            SCREEN_WIDTH,
            4.0,
            rgba(40, 110, 150, alpha),
        );
    }
}

fn draw_attract(canvas: &mut Canvas, game: &Game, muted: bool) {
    let ticks = game.ticks();

    draw_text(canvas, 8.0, 7.0, "HI-SCORE", 1, DIM_WHITE);
    draw_text_right(
        canvas,
        216.0,
        7.0,
        &format!("{:05}", game.high_score()),
        1,
        WHITE,
    );

    draw_text_centred(canvas, 112.0, 28.0, "SPACE", 3, GREEN);
    draw_text_centred(canvas, 112.0, 56.0, "INVADERS", 3, WHITE);
    canvas.rect(58.0, 88.0, 108.0, 1.0, rgba(64, 255, 128, 0.5));

    // A compact score table, with sprites drawn at their native pixel size.
    let squid = if sprites::frame_a(0) {
        sprites::SQUID_A.rows
    } else {
        sprites::SQUID_B.rows
    };
    canvas.sprite(53.0, 101.0, squid, SQUID_COLOR);
    draw_text(canvas, 70.0, 102.0, "= 30 PTS", 1, WHITE);
    canvas.sprite(51.0, 116.0, sprites::CRAB_A.rows, CRAB_COLOR);
    draw_text(canvas, 70.0, 117.0, "= 20 PTS", 1, WHITE);
    canvas.sprite(50.0, 131.0, sprites::OCTOPUS_A.rows, OCTOPUS_COLOR);
    draw_text(canvas, 70.0, 132.0, "= 10 PTS", 1, WHITE);
    canvas.sprite(48.0, 147.0, sprites::MYSTERY.rows, MYSTERY_COLOR);
    draw_text(canvas, 70.0, 148.0, "= ???", 1, MYSTERY_COLOR);

    // A little marching demo formation. It only exists in the renderer; the
    // simulation stays authoritative for real gameplay.
    let step = ((ticks / 8) % 14) as f32;
    let bounce = step.min(13.0 - step);
    for row in 0..3 {
        for column in 0..5 {
            let kind = match row {
                0 => AlienKind::Squid,
                1 => AlienKind::Crab,
                _ => AlienKind::Octopus,
            };
            let sprite = alien_sprite(kind, ((ticks / 16) % 2) as u8);
            let width = sprite_width(sprite);
            let x = 48.0 + column as f32 * 22.0 - width / 2.0 + bounce;
            let y = 168.0 + row as f32 * 12.0;
            canvas.sprite(x, y, sprite, alien_color(kind));
        }
    }

    if blink(ticks, 30) {
        draw_text_centred(canvas, 112.0, 208.0, "PRESS ENTER", 1, GREEN);
    }
    draw_text_centred(
        canvas,
        112.0,
        222.0,
        "ARROWS MOVE   SPACE FIRE",
        1,
        DIM_WHITE,
    );
    draw_text_centred(
        canvas,
        112.0,
        234.0,
        "P PAUSE   M MUTE   ESC PAUSE",
        1,
        DIM_WHITE,
    );
    draw_text_centred(
        canvas,
        112.0,
        246.0,
        "A 1978 TAITO ARCADE TRIBUTE",
        1,
        rgba(80, 110, 130, 1.0),
    );
    if muted {
        draw_text_right(canvas, 216.0, 18.0, "MUTE", 1, DIM_WHITE);
    }
}

fn draw_playfield(canvas: &mut Canvas, game: &Game) {
    // Ground line and side rails.
    canvas.rect(0.0, 232.0, SCREEN_WIDTH, 1.0, rgba(64, 255, 128, 0.55));
    canvas.rect(0.0, 233.0, SCREEN_WIDTH, 1.0, rgba(20, 100, 50, 0.5));
    canvas.rect(2.0, 30.0, 1.0, 202.0, rgba(40, 120, 90, 0.16));
    canvas.rect(221.0, 30.0, 1.0, 202.0, rgba(40, 120, 90, 0.16));

    for bunker in game.bunkers() {
        for row in 0..bunker.height() {
            let color = if row < 3 { BUNKER } else { BUNKER_DARK };
            for column in 0..bunker.width() {
                if bunker.cell(column, row) {
                    canvas.rect(
                        bunker.x() + column as f32,
                        bunker.y() + row as f32,
                        1.0,
                        1.0,
                        color,
                    );
                }
            }
        }
    }

    for alien in game.aliens().iter().filter(|alien| alien.alive) {
        let sprite = alien_sprite(alien.kind, game.animation_frame());
        canvas.sprite(
            alien.x.round(),
            alien.y.round(),
            sprite,
            alien_color(alien.kind),
        );
    }

    if let Some(ship) = game.mystery_ship() {
        draw_mystery_ship(canvas, ship, game.ticks());
    }

    let player = game.player();
    if player.alive {
        canvas.sprite(
            player.x.round(),
            player.y.round(),
            sprites::PLAYER.rows,
            PLAYER_COLOR,
        );
    }

    if let Some(bullet) = game.player_bullet() {
        draw_bullet(canvas, bullet, BULLET_COLOR);
    }
    for bullet in game.enemy_bullets() {
        draw_bullet(canvas, *bullet, ENEMY_BULLET_COLOR);
    }

    for explosion in game.explosions() {
        draw_explosion(canvas, *explosion, game.ticks());
    }

    for popup in game.score_popups() {
        draw_text(
            canvas,
            popup.x.round(),
            popup.y.round(),
            &format!("{}", popup.points),
            1,
            YELLOW,
        );
    }

    let lives = game.lives().min(5);
    for index in 0..lives {
        canvas.sprite(
            8.0 + index as f32 * 16.0,
            239.0,
            sprites::PLAYER.rows,
            rgba(64, 255, 128, 0.85),
        );
    }
    if game.lives() > 5 {
        draw_text(
            canvas,
            8.0 + 5.0 * 16.0,
            240.0,
            &format!("X{}", game.lives()),
            1,
            GREEN,
        );
    }
}

fn draw_hud(canvas: &mut Canvas, game: &Game, muted: bool) {
    draw_text(canvas, 8.0, 6.0, "SCORE", 1, DIM_WHITE);
    draw_text(canvas, 8.0, 15.0, &format!("{:05}", game.score()), 1, WHITE);
    draw_text(canvas, 132.0, 6.0, "HI-SCORE", 1, DIM_WHITE);
    draw_text_right(
        canvas,
        216.0,
        15.0,
        &format!("{:05}", game.high_score()),
        1,
        WHITE,
    );
    let wave = format!("WAVE {}", game.level());
    draw_text_right(canvas, 216.0, 240.0, &wave, 1, GREEN);
    if muted {
        draw_text_centred(canvas, 112.0, 249.0, "MUTE", 1, DIM_WHITE);
    }
}

fn draw_mystery_ship(canvas: &mut Canvas, ship: MysteryShip, ticks: u64) {
    let color = if (ticks / 4) % 2 == 0 {
        MYSTERY_COLOR
    } else {
        rgba(255, 160, 120, 1.0)
    };
    canvas.sprite(ship.x.round(), 36.0, sprites::MYSTERY.rows, color);
}

fn draw_bullet(canvas: &mut Canvas, bullet: Bullet, color: [f32; 4]) {
    canvas.rect(
        bullet.x.round(),
        bullet.y.round(),
        bullet.w,
        bullet.h,
        color,
    );
}

fn draw_explosion(canvas: &mut Canvas, explosion: Explosion, ticks: u64) {
    let elapsed = explosion.duration.saturating_sub(explosion.ticks_left);
    if (ticks / 2 + u64::from(explosion.duration)) % 2 == 0 && elapsed > explosion.duration / 2 {
        return;
    }
    let (rows, color) = match explosion.kind {
        ExplosionKind::Invader => (sprites::INVADER_BURST.rows, WHITE),
        ExplosionKind::Player => (sprites::PLAYER_BURST.rows, PLAYER_COLOR),
        ExplosionKind::Spark => (sprites::SPARK.rows, YELLOW),
        ExplosionKind::Mystery => (sprites::INVADER_BURST.rows, MYSTERY_COLOR),
    };
    let width = rows.first().map_or(0, |row| row.len()) as f32;
    let height = rows.len() as f32;
    canvas.sprite(
        explosion.x.round() - width / 2.0,
        explosion.y.round() - height / 2.0,
        rows,
        color,
    );

    if matches!(explosion.kind, ExplosionKind::Player) && elapsed < 30 {
        for ring in 0..3 {
            let radius = 5.0 + ring as f32 * 4.0 + elapsed as f32 * 0.25;
            canvas.outline(
                explosion.x - radius,
                explosion.y - radius,
                radius * 2.0,
                radius * 2.0,
                rgba(255, 200, 120, 0.55 - ring as f32 * 0.15),
            );
        }
    }
}

fn draw_scanlines(canvas: &mut Canvas) {
    for y in (0..SCREEN_HEIGHT as usize).step_by(2) {
        canvas.rect(0.0, y as f32, SCREEN_WIDTH, 1.0, rgba(0, 0, 0, 0.09));
    }
    for band in 0..6 {
        let alpha = 0.025 * (6 - band) as f32;
        canvas.rect(
            0.0,
            band as f32 * 2.0,
            SCREEN_WIDTH,
            2.0,
            rgba(0, 0, 0, alpha),
        );
        canvas.rect(
            0.0,
            SCREEN_HEIGHT - (band + 1) as f32 * 2.0,
            SCREEN_WIDTH,
            2.0,
            rgba(0, 0, 0, alpha),
        );
    }
}

fn alien_sprite(kind: AlienKind, frame: u8) -> &'static [&'static str] {
    match (kind, sprites::frame_a(frame)) {
        (AlienKind::Squid, true) => sprites::SQUID_A.rows,
        (AlienKind::Squid, false) => sprites::SQUID_B.rows,
        (AlienKind::Crab, true) => sprites::CRAB_A.rows,
        (AlienKind::Crab, false) => sprites::CRAB_B.rows,
        (AlienKind::Octopus, true) => sprites::OCTOPUS_A.rows,
        (AlienKind::Octopus, false) => sprites::OCTOPUS_B.rows,
    }
}

fn alien_color(kind: AlienKind) -> [f32; 4] {
    match kind {
        AlienKind::Squid => SQUID_COLOR,
        AlienKind::Crab => CRAB_COLOR,
        AlienKind::Octopus => OCTOPUS_COLOR,
    }
}

fn sprite_width(rows: &[&str]) -> f32 {
    rows.first().map_or(0, |row| row.len()) as f32
}

fn blink(ticks: u64, period: u64) -> bool {
    (ticks / period) % 2 == 0
}

#[cfg(test)]
mod tests {
    use super::build;
    use crate::pixel_font::text_width;
    use space_invaders_core::{Command, Game, Phase};

    #[test]
    fn attract_scene_has_geometry() {
        let game = Game::with_seed(0, 1);
        let canvas = build(&game, false);
        assert!(canvas.vertices().len() > 2_000);
        assert!(text_width("SPACE INVADERS", 1) <= 224);
    }

    #[test]
    fn gameplay_scene_changes_with_lives() {
        let mut game = Game::with_seed(0, 2);
        game.command(Command::Start);
        for _ in 0..100 {
            game.tick();
        }
        assert_eq!(game.phase(), Phase::Playing);
        let canvas = build(&game, true);
        assert!(canvas.vertices().len() > 4_000);
    }
}
