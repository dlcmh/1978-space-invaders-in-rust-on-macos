//! SPACE INVADERS — a native macOS tribute rendered with Metal.
//!
//! The binary is a thin shell around two deliberate layers: the headless
//! `space-invaders-core` simulation and this crate's AppKit/Metal presentation.

#![deny(unsafe_op_in_unsafe_fn)]

mod app;
mod audio;
mod canvas;
mod metal_shaders;
mod pixel_font;
mod preferences;
mod renderer;
mod scene;
mod sprites;
mod view;

fn main() {
    if let Err(error) = app::run() {
        eprintln!("SPACE INVADERS could not start: {error}");
        std::process::exit(1);
    }
}
