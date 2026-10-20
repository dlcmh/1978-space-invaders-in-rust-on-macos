//! Axis-aligned geometry shared by the simulation and collision checks.

/// A rectangle in arcade pixels, with the origin at the top left.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    /// Left edge.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Width.
    pub w: f32,
    /// Height.
    pub h: f32,
}

impl Rect {
    /// Creates a rectangle from its top-left corner and size.
    #[must_use]
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    /// The right edge (exclusive).
    #[must_use]
    pub const fn right(self) -> f32 {
        self.x + self.w
    }

    /// The bottom edge (exclusive).
    #[must_use]
    pub const fn bottom(self) -> f32 {
        self.y + self.h
    }

    /// Returns `true` when the two rectangles share any positive area.
    #[must_use]
    pub fn overlaps(self, other: Self) -> bool {
        self.x < other.right()
            && self.right() > other.x
            && self.y < other.bottom()
            && self.bottom() > other.y
    }

    /// Returns this rectangle inset by `amount` on every side.
    #[must_use]
    pub fn inset(self, amount: f32) -> Self {
        Self::new(
            self.x + amount,
            self.y + amount,
            (self.w - amount * 2.0).max(0.0),
            (self.h - amount * 2.0).max(0.0),
        )
    }

    /// Converts the rectangle to the integer cell range it touches.
    #[must_use]
    pub fn cell_bounds(self) -> (i32, i32, i32, i32) {
        (
            self.x.floor() as i32,
            self.y.floor() as i32,
            self.right().ceil() as i32,
            self.bottom().ceil() as i32,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::Rect;

    #[test]
    fn overlaps_respects_exclusive_edges() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0);
        assert!(a.overlaps(Rect::new(9.0, 9.0, 2.0, 2.0)));
        assert!(!a.overlaps(Rect::new(10.0, 0.0, 2.0, 2.0)));
        assert!(!a.overlaps(Rect::new(-2.0, 0.0, 2.0, 2.0)));
    }

    #[test]
    fn cell_bounds_cover_fractional_pixels() {
        assert_eq!(Rect::new(1.2, 2.0, 1.0, 4.0).cell_bounds(), (1, 2, 3, 6));
    }
}
