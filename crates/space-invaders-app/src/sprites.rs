//! Original procedural pixel art for every actor in the game.
//!
//! The sprites are authored here as ASCII masks rather than loaded from image
//! files. They evoke the cabinet's silhouettes without copying any ROM or
//! sprite-sheet data.

/// Immutable ASCII pixel art.
#[derive(Clone, Copy, Debug)]
pub struct Sprite {
    /// Rows of `#` (opaque) and `.` (transparent) cells.
    pub rows: &'static [&'static str],
}

/// The player's laser cannon.
pub const PLAYER: Sprite = Sprite {
    rows: &[
        "......#......",
        ".....###.....",
        ".....###.....",
        ".###########.",
        "#############",
        "#############",
        "#############",
        "#############",
    ],
};

/// Top-row invader, frame A.
pub const SQUID_A: Sprite = Sprite {
    rows: &[
        "...##...", "..####..", ".######.", "##.##.##", "########", "..####..", ".#.##.#.",
        "#.#..#.#",
    ],
};

/// Top-row invader, frame B.
pub const SQUID_B: Sprite = Sprite {
    rows: &[
        "...##...", "..####..", ".######.", "##.##.##", "########", ".#.##.#.", "#......#",
        ".#....#.",
    ],
};

/// Middle-row invader, frame A.
pub const CRAB_A: Sprite = Sprite {
    rows: &[
        "..#.....#..",
        "...#...#...",
        "..#######..",
        ".##.###.##.",
        "###########",
        "#.#######.#",
        "#.#.....#.#",
        "...##.##...",
    ],
};

/// Middle-row invader, frame B.
pub const CRAB_B: Sprite = Sprite {
    rows: &[
        "..#.....#..",
        "#..#...#..#",
        "#.#######.#",
        "###.###.###",
        "###########",
        ".#########.",
        "..#.....#..",
        ".#.......#.",
    ],
};

/// Bottom-row invader, frame A.
pub const OCTOPUS_A: Sprite = Sprite {
    rows: &[
        "....####....",
        ".##########.",
        "############",
        "###..##..###",
        "############",
        "...##..##...",
        "..##.##.##..",
        "##........##",
    ],
};

/// Bottom-row invader, frame B.
pub const OCTOPUS_B: Sprite = Sprite {
    rows: &[
        "....####....",
        ".##########.",
        "############",
        "###..##..###",
        "############",
        "..###..###..",
        ".##..##..##.",
        "..##....##..",
    ],
};

/// The mystery ship.
pub const MYSTERY: Sprite = Sprite {
    rows: &[
        "....########....",
        "..############..",
        ".##############.",
        "##.##.##.##.##.#",
        "################",
        "..###..##..###..",
        "...#........#...",
    ],
};

/// Burst shown when an invader or the mystery ship is destroyed.
pub const INVADER_BURST: Sprite = Sprite {
    rows: &[
        "..#...#...#..",
        "...#..#..#...",
        "....#...#....",
        "...#.###.#...",
        "..#.#####.#..",
        "...#.###.#...",
        "....#...#....",
        "...#..#..#...",
    ],
};

/// Burst shown when the cannon is destroyed.
pub const PLAYER_BURST: Sprite = Sprite {
    rows: &[
        "..#...#...#..",
        "#..#..#..#..#",
        ".#..#...#..#.",
        "..#..###..#..",
        "##..#####..##",
        "..#..###..#..",
        ".#..#...#..#.",
        "#..#..#..#..#",
    ],
};

/// Tiny impact spark for a bullet hitting cover.
pub const SPARK: Sprite = Sprite {
    rows: &["..#..", "..#..", "#####", "..#..", "..#.."],
};

/// Frame A or B for an invader.
#[must_use]
pub const fn frame_a(frame: u8) -> bool {
    frame & 1 == 0
}

#[cfg(test)]
mod tests {
    use super::{
        CRAB_A, CRAB_B, INVADER_BURST, MYSTERY, OCTOPUS_A, OCTOPUS_B, PLAYER, PLAYER_BURST, SPARK,
        SQUID_A, SQUID_B,
    };

    fn assert_rectangular(rows: &[&str]) {
        let width = rows[0].len();
        assert!(width > 0);
        for row in rows {
            assert_eq!(row.len(), width, "sprite rows must be rectangular");
            assert!(row.bytes().all(|cell| cell == b'#' || cell == b'.'));
        }
    }

    #[test]
    fn every_sprite_is_rectangular() {
        for sprite in [
            PLAYER,
            SQUID_A,
            SQUID_B,
            CRAB_A,
            CRAB_B,
            OCTOPUS_A,
            OCTOPUS_B,
            MYSTERY,
            INVADER_BURST,
            PLAYER_BURST,
            SPARK,
        ] {
            assert_rectangular(sprite.rows);
        }
    }

    #[test]
    fn canonical_sprite_sizes() {
        assert_eq!(PLAYER.rows[0].len(), 13);
        assert_eq!(SQUID_A.rows[0].len(), 8);
        assert_eq!(CRAB_A.rows[0].len(), 11);
        assert_eq!(OCTOPUS_A.rows[0].len(), 12);
        assert_eq!(MYSTERY.rows[0].len(), 16);
    }
}
