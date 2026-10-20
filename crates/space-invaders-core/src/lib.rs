//! # Space Invaders Core
//!
//! A headless, deterministic implementation of the 1978 Taito arcade game.
//!
//! The crate owns every gameplay rule: the 55-invader formation, its accelerating
//! march, player and enemy fire, destructible bunkers, the mystery ship, scoring,
//! lives, waves, and the attract/ready/death/game-over state machine. It has no
//! platform dependencies, no unsafe code, and no notion of real time. A caller
//! advances it with [`Game::tick`] at a fixed 60 Hz and feeds [`Command`]s from
//! the input layer.
//!
//! ```no_run
//! use space_invaders_core::{Command, Game, Phase};
//!
//! let mut game = Game::new(0);
//! game.command(Command::Start);
//! while !matches!(game.phase(), Phase::GameOver) {
//!     game.tick();
//! }
//! ```
//!
//! Presentation code reads immutable snapshots and drains [`GameEvent`]s. It
//! must never duplicate gameplay rules or mutate the simulation directly.

#![forbid(unsafe_code)]

pub mod bunker;
pub mod game;
pub mod geometry;
pub mod layout;
pub mod rng;

pub use bunker::Bunker;
pub use game::{
    Alien, AlienKind, Bullet, Command, Explosion, ExplosionKind, Game, GameEvent, MysteryShip,
    Phase, Player, ScorePopup,
};
pub use geometry::Rect;
pub use layout::{SCREEN_HEIGHT, SCREEN_WIDTH};
