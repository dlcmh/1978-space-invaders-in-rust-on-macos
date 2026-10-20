//! A tiny immediate-mode triangle canvas in arcade-pixel coordinates.
//!
//! Every visual in the game — sprites, bunker cells, text, scanlines — is
//! emitted as coloured triangles into one vertex array. The renderer uploads
//! that array and issues a single draw call.

/// A GPU vertex: position in arcade pixels, colour in linear RGBA.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vertex {
    /// Arcade-pixel position.
    pub position: [f32; 2],
    /// RGBA, 0.0 to 1.0.
    pub color: [f32; 4],
}

/// Maximum number of vertices the renderer's ring buffers accept.
pub const MAX_VERTICES: usize = 100_000;

/// Builds a frame's worth of triangles.
#[derive(Debug)]
pub struct Canvas {
    vertices: Vec<Vertex>,
}

impl Canvas {
    /// Creates an empty canvas with room for a typical frame.
    #[must_use]
    pub fn new() -> Self {
        Self {
            vertices: Vec::with_capacity(16_384),
        }
    }

    /// The accumulated vertices.
    #[must_use]
    pub fn vertices(&self) -> &[Vertex] {
        &self.vertices
    }

    /// Appends one triangle.
    pub fn triangle(&mut self, a: [f32; 2], b: [f32; 2], c: [f32; 2], color: [f32; 4]) {
        if self.vertices.len() + 3 > MAX_VERTICES {
            return;
        }
        self.vertices.push(Vertex { position: a, color });
        self.vertices.push(Vertex { position: b, color });
        self.vertices.push(Vertex { position: c, color });
    }

    /// Appends an axis-aligned rectangle as two triangles.
    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) {
        if w <= 0.0 || h <= 0.0 {
            return;
        }
        let right = x + w;
        let bottom = y + h;
        self.triangle([x, y], [right, y], [right, bottom], color);
        self.triangle([x, y], [right, bottom], [x, bottom], color);
    }

    /// Appends a rectangular outline one pixel thick.
    pub fn outline(&mut self, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) {
        self.rect(x, y, w, 1.0, color);
        self.rect(x, y + h - 1.0, w, 1.0, color);
        self.rect(x, y, 1.0, h, color);
        self.rect(x + w - 1.0, y, 1.0, h, color);
    }

    /// Appends a sprite whose `#` cells are opaque; other characters are empty.
    ///
    /// Consecutive pixels in a row are merged into one quad, which keeps the
    /// vertex count low without a texture atlas.
    pub fn sprite(&mut self, x: f32, y: f32, rows: &[&str], color: [f32; 4]) {
        for (row_index, row) in rows.iter().enumerate() {
            let mut run_start: Option<usize> = None;
            for (column, cell) in row.bytes().enumerate() {
                let solid = cell == b'#';
                match (solid, run_start) {
                    (true, None) => run_start = Some(column),
                    (false, Some(start)) => {
                        self.rect(
                            x + start as f32,
                            y + row_index as f32,
                            (column - start) as f32,
                            1.0,
                            color,
                        );
                        run_start = None;
                    }
                    _ => {}
                }
            }
            if let Some(start) = run_start {
                self.rect(
                    x + start as f32,
                    y + row_index as f32,
                    (row.len() - start) as f32,
                    1.0,
                    color,
                );
            }
        }
    }
}

impl Default for Canvas {
    fn default() -> Self {
        Self::new()
    }
}

/// Converts 0-255 sRGB components and a 0-1 alpha into a shader colour.
#[must_use]
pub const fn rgba(red: u8, green: u8, blue: u8, alpha: f32) -> [f32; 4] {
    [
        red as f32 / 255.0,
        green as f32 / 255.0,
        blue as f32 / 255.0,
        alpha,
    ]
}

#[cfg(test)]
mod tests {
    use super::Canvas;

    #[test]
    fn rectangles_emit_two_triangles() {
        let mut canvas = Canvas::new();
        canvas.rect(0.0, 0.0, 10.0, 4.0, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(canvas.vertices().len(), 6);
        canvas.rect(0.0, 0.0, 0.0, 4.0, [1.0; 4]);
        assert_eq!(canvas.vertices().len(), 6);
    }

    #[test]
    fn sprite_runs_merge_into_quads() {
        let mut canvas = Canvas::new();
        canvas.sprite(0.0, 0.0, &["###..##", "......."], [1.0; 4]);
        assert_eq!(canvas.vertices().len(), 12, "two runs on the first row");
    }
}
