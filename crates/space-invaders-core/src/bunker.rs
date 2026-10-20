//! Destructible defence bunkers.
//!
//! A bunker is a 22 x 16 grid of one-pixel cells. Bullets carve small,
//! jagged holes out of that grid; the renderer simply draws the cells that are
//! still alive. Keeping the damage model in the headless crate means bunker
//! behaviour is reproducible and testable without a GPU.

use crate::geometry::Rect;
use crate::layout::{BUNKER_HEIGHT, BUNKER_WIDTH, BUNKER_X, BUNKER_Y};
use crate::rng::Rng;

const CELLS: usize = BUNKER_WIDTH * BUNKER_HEIGHT;

/// One destructible defence bunker.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bunker {
    x: f32,
    cells: [bool; CELLS],
}

impl Bunker {
    /// Builds a bunker with the classic rounded roof and arched doorway.
    #[must_use]
    pub fn new(x: f32) -> Self {
        let mut cells = [true; CELLS];
        for row in 0..BUNKER_HEIGHT {
            for col in 0..BUNKER_WIDTH {
                let rounded_roof = (row == 0 && !(2..=19).contains(&col))
                    || (row == 1 && !(1..=20).contains(&col));
                let delta = (11_i32 - col as i32).unsigned_abs() as usize;
                let doorway = row >= 9 && delta <= (row - 8) / 2;
                let clipped_base = row >= 15 && !(1..=20).contains(&col);
                if rounded_roof || doorway || clipped_base {
                    cells[row * BUNKER_WIDTH + col] = false;
                }
            }
        }
        Self { x, cells }
    }

    /// The four bunkers in their canonical positions.
    #[must_use]
    pub fn standard_set() -> [Self; 4] {
        BUNKER_X.map(Self::new)
    }

    /// The bunker's left edge.
    #[must_use]
    pub const fn x(&self) -> f32 {
        self.x
    }

    /// The bunker's top edge.
    #[must_use]
    pub const fn y(&self) -> f32 {
        BUNKER_Y
    }

    /// Width in cells.
    #[must_use]
    pub const fn width(&self) -> usize {
        BUNKER_WIDTH
    }

    /// Height in cells.
    #[must_use]
    pub const fn height(&self) -> usize {
        BUNKER_HEIGHT
    }

    /// Returns whether the cell at `(col, row)` still exists.
    #[must_use]
    pub fn cell(&self, col: usize, row: usize) -> bool {
        col < BUNKER_WIDTH && row < BUNKER_HEIGHT && self.cells[row * BUNKER_WIDTH + col]
    }

    /// Returns whether a world-space point is inside intact bunker material.
    #[must_use]
    pub fn solid_at(&self, x: f32, y: f32) -> bool {
        if x < self.x || y < BUNKER_Y {
            return false;
        }
        let col = (x - self.x) as usize;
        let row = (y - BUNKER_Y) as usize;
        self.cell(col, row)
    }

    /// Number of intact cells. Useful for checks and for a damage percentage.
    #[must_use]
    pub fn intact_cells(&self) -> usize {
        self.cells.iter().filter(|cell| **cell).count()
    }

    /// Finds the intact cell a projectile collides with.
    ///
    /// `rising` selects the lowest hit cell for an upward projectile and the
    /// highest hit cell for a descending one, which keeps impact craters on the
    /// side the bullet actually struck.
    #[must_use]
    pub fn impact(&self, projectile: Rect, rising: bool) -> Option<(usize, usize)> {
        let (left, top, right, bottom) = projectile.cell_bounds();
        let local_left = left - self.x as i32;
        let local_top = top - BUNKER_Y as i32;
        let local_right = right - self.x as i32;
        let local_bottom = bottom - BUNKER_Y as i32;

        let span = (local_bottom - local_top).max(0);
        for index in 0..span {
            let row = if rising {
                local_bottom - 1 - index
            } else {
                local_top + index
            };
            if !(0..BUNKER_HEIGHT as i32).contains(&row) {
                continue;
            }
            for col in local_left..local_right {
                if (0..BUNKER_WIDTH as i32).contains(&col) && self.cell(col as usize, row as usize)
                {
                    return Some((col as usize, row as usize));
                }
            }
        }
        None
    }

    /// Removes a small, irregular crater centred on `(col, row)`.
    ///
    /// The shape is deliberately asymmetric: a perfectly circular crater looks
    /// mechanical, while the arcade original chewed ragged holes in the shield.
    pub fn carve(&mut self, col: usize, row: usize, rng: &mut Rng) {
        for dy in -2_i32..=2 {
            for dx in -3_i32..=3 {
                let distance = dx * dx + dy * dy * 2;
                let ragged = distance <= 5 || (distance <= 9 && rng.one_in(2));
                if !ragged {
                    continue;
                }
                let c = col as i32 + dx;
                let r = row as i32 + dy;
                if (0..BUNKER_WIDTH as i32).contains(&c) && (0..BUNKER_HEIGHT as i32).contains(&r) {
                    self.cells[r as usize * BUNKER_WIDTH + c as usize] = false;
                }
            }
        }
    }

    /// Erases the material under an invader that has descended into the shield.
    pub fn crush(&mut self, area: Rect) {
        let (left, top, right, bottom) = area.cell_bounds();
        for row in top - BUNKER_Y as i32..bottom - BUNKER_Y as i32 {
            for col in left - self.x as i32..right - self.x as i32 {
                if (0..BUNKER_WIDTH as i32).contains(&col)
                    && (0..BUNKER_HEIGHT as i32).contains(&row)
                {
                    self.cells[row as usize * BUNKER_WIDTH + col as usize] = false;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Bunker;
    use crate::geometry::Rect;
    use crate::rng::Rng;

    #[test]
    fn roof_and_doorway_are_open() {
        let bunker = Bunker::new(24.0);
        assert!(!bunker.cell(0, 0));
        assert!(bunker.cell(10, 0));
        assert!(!bunker.cell(11, 15));
        assert!(bunker.cell(2, 8));
        assert!(bunker.intact_cells() > 200);
    }

    #[test]
    fn carving_removes_cells_and_stops_at_edges() {
        let mut bunker = Bunker::new(24.0);
        let before = bunker.intact_cells();
        bunker.carve(11, 4, &mut Rng::new(9));
        assert!(bunker.intact_cells() < before);
        bunker.carve(0, 0, &mut Rng::new(9));
        bunker.carve(21, 15, &mut Rng::new(9));
        assert!(!bunker.cell(21, 15) || bunker.intact_cells() < before);
    }

    #[test]
    fn impact_picks_struck_side() {
        let bunker = Bunker::new(24.0);
        let rising = bunker.impact(Rect::new(30.0, 190.8, 1.0, 2.0), true);
        let falling = bunker.impact(Rect::new(30.0, 190.8, 1.0, 2.0), false);
        assert!(rising.is_some());
        assert!(falling.is_some());
        assert!(rising.unwrap().1 >= falling.unwrap().1);
    }
}
