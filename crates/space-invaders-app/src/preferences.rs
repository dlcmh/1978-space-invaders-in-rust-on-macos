//! The only two pieces of persistent state, stored in `NSUserDefaults`.
//!
//! Keeping preferences in the platform layer means the simulation never needs
//! filesystem or Foundation access, and the privacy surface stays tiny.

use objc2::rc::Retained;
use objc2_foundation::{NSString, NSUserDefaults};

const HIGH_SCORE_KEY: &str = "SpaceInvadersArcadeHighScore";
const MUTED_KEY: &str = "SpaceInvadersArcadeMuted";

fn key(name: &str) -> Retained<NSString> {
    NSString::from_str(name)
}

/// Reads the persistent high score, clamped to a sane range.
#[must_use]
pub fn load_high_score() -> u32 {
    let defaults = NSUserDefaults::standardUserDefaults();
    u32::try_from(defaults.integerForKey(&key(HIGH_SCORE_KEY)))
        .unwrap_or(0)
        .min(99_999)
}

/// Persists a high score.
pub fn save_high_score(score: u32) {
    let defaults = NSUserDefaults::standardUserDefaults();
    defaults.setInteger_forKey(score as isize, &key(HIGH_SCORE_KEY));
}

/// Reads the persisted mute preference.
#[must_use]
pub fn load_muted() -> bool {
    NSUserDefaults::standardUserDefaults().boolForKey(&key(MUTED_KEY))
}

/// Persists the mute preference.
pub fn save_muted(muted: bool) {
    NSUserDefaults::standardUserDefaults().setBool_forKey(muted, &key(MUTED_KEY));
}
