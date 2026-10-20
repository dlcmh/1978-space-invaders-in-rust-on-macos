//! The fixed arcade coordinate system.
//!
//! Everything in the simulation is expressed in the 224 x 256 logical-pixel
//! space of the 1978 cabinet. The renderer scales that space to the window and
//! never feeds scale information back into gameplay.

/// Logical screen width, in arcade pixels.
pub const SCREEN_WIDTH: f32 = 224.0;
/// Logical screen height, in arcade pixels.
pub const SCREEN_HEIGHT: f32 = 256.0;

/// Horizontal playfield limit for moving actors.
pub const PLAY_LEFT: f32 = 8.0;
/// Right-hand playfield limit for moving actors.
pub const PLAY_RIGHT: f32 = 216.0;
/// The horizontal green ground line.
pub const GROUND_Y: f32 = 232.0;
/// The top of the HUD; nothing gameplay-related may rise above it.
pub const HUD_BOTTOM: f32 = 30.0;

/// Player cannon width.
pub const PLAYER_WIDTH: f32 = 13.0;
/// Player cannon height.
pub const PLAYER_HEIGHT: f32 = 8.0;
/// The cannon's resting top edge.
pub const PLAYER_Y: f32 = 216.0;
/// Cannon horizontal movement per 60 Hz tick.
pub const PLAYER_SPEED: f32 = 2.0;

/// Invader columns in the canonical 55-alien formation.
pub const FORMATION_COLUMNS: usize = 11;
/// Invader rows in the canonical 55-alien formation.
pub const FORMATION_ROWS: usize = 5;
/// Horizontal distance between invader cell centres.
pub const FORMATION_SPACING_X: f32 = 16.0;
/// Vertical distance between invader cell centres.
pub const FORMATION_SPACING_Y: f32 = 16.0;
/// Left edge of the initial formation.
pub const FORMATION_X: f32 = 24.0;
/// Top edge of the initial formation.
pub const FORMATION_Y: f32 = 48.0;
/// Distance the formation drops after touching a side.
pub const FORMATION_DROP: f32 = 8.0;

/// Number of destructible defence bunkers.
pub const BUNKER_COUNT: usize = 4;
/// Left edges of the four bunkers.
pub const BUNKER_X: [f32; BUNKER_COUNT] = [24.0, 72.0, 120.0, 168.0];
/// Top edge shared by every bunker.
pub const BUNKER_Y: f32 = 176.0;
/// Bunker width in destructible cells (one logical pixel per cell).
pub const BUNKER_WIDTH: usize = 22;
/// Bunker height in destructible cells.
pub const BUNKER_HEIGHT: usize = 16;

/// Player bullet upward speed per tick.
pub const PLAYER_BULLET_SPEED: f32 = 4.0;
/// Enemy bullet downward speed per tick.
pub const ENEMY_BULLET_SPEED: f32 = 1.5;
/// Maximum simultaneous enemy bullets at level 1.
pub const BASE_ENEMY_BULLETS: usize = 3;

/// Mystery-ship cruising height.
pub const MYSTERY_Y: f32 = 36.0;
/// Mystery-ship width.
pub const MYSTERY_WIDTH: f32 = 16.0;
/// Mystery-ship height.
pub const MYSTERY_HEIGHT: f32 = 7.0;
/// Mystery-ship speed per tick.
pub const MYSTERY_SPEED: f32 = 1.0;

/// Starting lives.
pub const STARTING_LIVES: u32 = 3;
/// Score at which the single bonus life is awarded.
pub const BONUS_LIFE_SCORE: u32 = 1_500;
