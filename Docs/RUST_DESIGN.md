# Rust Design Notes

This document records the engineering choices behind **SPACE INVADERS — Metal
Arcade**. It is meant to be read before changing the architecture, and it
explains why the repository looks the way it does rather than merely describing
what each file contains.

The short version: a **deterministic, platform-free simulation** owns every game
rule; a **thin macOS shell** translates AppKit input into commands and Metal
draws a snapshot. Everything else follows from that boundary.

## 1. Workspace shape

The repository is a Cargo workspace with three crates:

| Crate | Role | Dependencies |
| --- | --- | --- |
| `space-invaders-core` | Headless simulation | None |
| `space-invaders-app` | AppKit window, Metal renderer, audio, persistence | `objc2` family, `block2` |
| `space-invaders-checks` | Public-API behavioral harness | `space-invaders-core` |

Why a workspace instead of one crate?

- The game rules compile and test on a machine with no window server and no GPU.
- The `Checks` harness cannot accidentally reach into AppKit to make a test pass.
- The boundary is enforced by Cargo rather than by discipline: the core crate
  literally cannot import `objc2_app_kit` because it does not depend on it.
- A future front end (a different windowing layer, a replay viewer, a server)
  can depend on the simulation without inheriting the macOS shell.

The core crate begins with `#![forbid(unsafe_code)]` in
[`lib.rs`](../crates/space-invaders-core/src/lib.rs) (`forbid`). That is a
deliberate guarantee: no gameplay rule can depend on an unchecked pointer.

## 2. State, commands, snapshots

The app never mutates an alien or a bullet directly. Input becomes a
[`Command`](../crates/space-invaders-core/src/game.rs) value and is applied by
[`Game::command`](../crates/space-invaders-core/src/game.rs). The simulation then
advances through [`Game::tick`](../crates/space-invaders-core/src/game.rs).

Presentation reads immutable snapshots:

- [`Game::aliens`](../crates/space-invaders-core/src/game.rs)
- [`Game::bunkers`](../crates/space-invaders-core/src/game.rs)
- [`Game::player_bullet`](../crates/space-invaders-core/src/game.rs)
- [`Game::mystery_ship`](../crates/space-invaders-core/src/game.rs)

Events are drained separately with
[`Game::take_events`](../crates/space-invaders-core/src/game.rs). This is the
Rust analogue of a command/query split: writes go through intent, reads go
through snapshots, and side effects that the platform cares about (sound,
persistence) are represented as data rather than callbacks from the core.

The alternative — `pub` mutable fields and ad-hoc setters — is easy to write and
hard to reason about. It also makes deterministic tests impossible to protect:
any caller could move an alien between ticks. Keeping the fields private and
exposing read-only slices has a small ergonomic cost and a large correctness
payoff.

## 3. Determinism and the fixed step

Gameplay advances in fixed 1/60-second ticks. The AppKit timer accumulates real
elapsed time and runs a bounded number of ticks, up to eight, per callback in
[`GameView`](../crates/space-invaders-app/src/view.rs) (`frame_tick`). This gives:

- identical physics on a 60 Hz and a 120 Hz display;
- a bounded catch-up after a window drag or debugger pause;
- a test harness that can replay thousands of ticks in milliseconds.

Randomness is an explicit xorshift64* generator in
[`rng.rs`](../crates/space-invaders-core/src/rng.rs) (`Rng`). It is seeded once per
game. There is no `rand` crate, no thread-local entropy, and no hidden clock.
Two games created with
[`Game::with_seed`](../crates/space-invaders-core/src/game.rs) produce identical
alien positions, bullet streams, and scores. The checks executable verifies this.

All movement is expressed in **arcade pixels** on a fixed 224 × 256 field, defined
in [`layout.rs`](../crates/space-invaders-core/src/layout.rs) (`SCREEN_WIDTH`).
Display scaling never feeds back into gameplay: a Retina display and a resized
window exercise exactly the same numbers.

## 4. Data modelling

The simulation uses small `Copy` value types (`Alien`, `Bullet`, `Player`,
`MysteryShip`) and narrow enums (`Phase`, `AlienKind`, `GameEvent`) rather than a
trait-object hierarchy. There is no ECS. For 55 aliens, four bunkers, and a
handful of projectiles, an ECS adds indirection and dependency weight without
improving locality or clarity.

A few deliberate choices:

- **`Rect` is a value**, not a reference to an entity. Collision code is a pure
  overlap test and cannot mutate state through its geometry.
- **Bunkers are fixed arrays of cells.** Damage is part of the simulation, so a
  returning player sees the same holes in a replay. The renderer only draws the
  cells that survive.
- **`Phase` owns its timers** (`Ready { ticks_left }`, `PlayerDeath { .. }`,
  `WaveClear { .. }`). Invalid combinations, such as a ready screen with no
  countdown, are unrepresentable.
- **Scoring is centralised** in `Game::award`, so the high score and the single
  bonus life cannot drift apart from the score display.
- **Events are semantic**, not audio commands. The core emits
  `GameEvent::InvaderKilled`; the app decides which synthesizer cue that maps to.

Numeric types are intentional: world positions are `f32` because they move in
sub-pixel steps; scores and timers are unsigned integers; indices are `usize`.
There are no newtypes around `u32` scores, because they would add ceremony
without preventing a real bug at this scale.

## 5. Error handling

There is no `anyhow` or `thiserror` dependency. Errors are handled at the
boundary where the app can do something useful:

- [`Renderer::new`](../crates/space-invaders-app/src/renderer.rs) returns a private
  `RendererError` with a human-readable message from Metal or an allocation
  failure.
- [`AudioEngine::new`](../crates/space-invaders-app/src/audio.rs) returns
  `Result<Self, String>` and the app treats audio as optional: a Mac with no
  available output device still plays the game silently.
- [`app::run`](../crates/space-invaders-app/src/app.rs) returns
  `Result<(), Box<dyn std::error::Error>>`; `main` prints a clear message and
  exits non-zero.

Inside the simulation there are no recoverable errors: a tick is total. Panics
indicate a programming error, not a player-action failure. Every arithmetic
operation that could overflow with a long session uses saturating arithmetic
(score, lives, level, timers, tick count).

The `unsafe` policy is equally explicit. The core forbids unsafe code. The app
uses unsafe only where Objective-C or Metal requires it, and every block carries
a `SAFETY` comment. `#![deny(unsafe_op_in_unsafe_fn)]` in
[`main.rs`](../crates/space-invaders-app/src/main.rs) (`main`) forces new unsafe operations
inside unsafe functions to be marked individually.

## 6. Why `objc2` instead of `winit` + `metal-rs`

The obvious alternative stack is `winit` for the window and `metal-rs` for the
GPU. It works, but it adds a cross-platform event-loop abstraction and a second
set of Metal bindings for a macOS-only game that already has to speak AppKit for
menus, preferences, and keyboard focus.

The chosen stack is:

- **`objc2` + `objc2-app-kit`**: a real `NSApplication`, `NSWindow`, NSView
  subclass, menu bar, first-responder chain, and `NSUserDefaults`.
- **`objc2-metal` + `objc2-quartz-core`**: `MTLDevice`, `MTLCommandQueue`,
  pipeline objects, buffers, and a `CAMetalLayer` on the custom view.
- **`objc2-audio-toolbox`**: a low-level `AudioQueue` output callback.
- **`block2`**: the completion handler for command-buffer pacing.

The result is one platform story instead of two, no hidden windowing runtime, and
direct access to the APIs the app actually needs. The trade-off is that the shell
is macOS-only and that the Objective-C interop is visible in the code. That is
acceptable for a native macOS artifact.

The full decision record, including rejected alternatives and the measured
trade-offs, is in
[`objc2-over-winit-metal-rs.md`](Refactoring/objc2-over-winit-metal-rs.md).

## 7. Rendering architecture

The renderer is intentionally immediate-mode:

1. [`scene::build`](../crates/space-invaders-app/src/scene.rs) walks the immutable
   game snapshot and appends coloured triangles to a
   [`Canvas`](../crates/space-invaders-app/src/canvas.rs).
2. Every sprite, glyph, bunker cell, star, and scanline is a quad made of two
   triangles. Runs of adjacent pixels in a sprite row are merged into one quad.
3. The whole frame is uploaded to one of three shared-storage vertex buffers.
4. A single `drawPrimitives` call renders it through one pipeline with alpha
   blending and 4× multisampling.
5. Two frames may be in flight; a completion handler recycles the third buffer.

The letterbox transform lives entirely in a four-float uniform. The vertex shader
in [`metal_shaders.rs`](../crates/space-invaders-app/src/metal_shaders.rs) (`SHADERS`)
converts arcade pixels to clip space, so the CPU never changes world coordinates
for presentation.

There is no texture atlas because there are no textures. There is no depth buffer
because a fixed 2D scene can use painter's order. There is no scene graph because
the scene is rebuilt from a small snapshot in microseconds. These are scope
decisions, not claims that immediate mode scales to a 3D engine.

## 8. Audio architecture

Sound effects are synthesized, not sampled. A fixed bank of sixteen voices in
[`audio.rs`](../crates/space-invaders-app/src/audio.rs) (`Voice`) renders square,
triangle, saw, and noise waveforms with a short attack and a linear release. The
game thread only calls [`AudioEngine::play`](../crates/space-invaders-app/src/audio.rs)
with a [`SoundCue`](../crates/space-invaders-app/src/audio.rs).

The audio callback:

- runs on an `AudioQueue` thread;
- locks a `Mutex<Synth>` only for the duration of one buffer fill;
- writes interleaved 16-bit mono samples at 44.1 kHz;
- re-enqueues the buffer.

The mutex is acceptable here because the critical section is short and the game
thread only holds it briefly when triggering a cue. A lock-free ring buffer would
be the next step if the synthesiser became heavier; it would not make the current
one faster in any way a player could hear. No allocation happens in the render
path except the fixed stack scratch buffer.

Audio failure is non-fatal. The app keeps the game loop and the mute preference
working, and logs no error spam.

## 9. Testing strategy

Three layers, each catching a different class of mistake:

1. **Unit tests inside the core** run private-state scenarios (wave transitions,
   landing, extra life, mystery-ship collision) without polluting the public API
   with test hooks.
2. **Integration-style checks in `space-invaders-checks`** use only the public
   command/tick/read surface, exactly like the app. They also run a 7,200-tick
   soak and a deterministic replay comparison.
3. **Renderer, scene, audio, font, and sprite tests** in the app cover pure
   functions and geometry invariants that do not need a window.

There is no `unsafe` test backdoor and no feature flag that changes gameplay for
tests. The checks executable is deliberately dependency-free so it can run in a
minimal environment.

`make check` adds formatting, Clippy with `-D warnings`, a release build, and the
documentation checker. Clippy runs with `all` and `pedantic` at warn level and
the workspace denies warnings, with a short explicit allow-list for lints that
fight the project's chosen style (for example integer-to-float casts in pixel
math).

## 10. Dependencies and supply chain

The whole workspace has one dependency family: the `objc2` bindings and
`block2`. These are thin, maintained bindings to Apple frameworks, not a game
engine or a runtime. The lockfile is committed, builds use `--locked` in the
gate, and the version set is intentionally pinned to versions that were compiled
and tested together.

No asset files, fonts, audio samples, or network services are fetched at build or
run time. The app has no network entitlement, no telemetry, and no updater.

## 11. What was deliberately not built

- **No ECS**, because the entity count and behaviour set are tiny.
- **No physics engine**, because everything is axis-aligned rectangles on a fixed
  grid.
- **No texture pipeline**, because all art is generated at runtime.
- **No save-game system**, because an arcade round is short and the only useful
  persistence is a high score.
- **No two-player mode**, because the commission described the single-player
  cabinet loop; the door is open but it is a scope change.
- **No level-256-style kill screen**, because that is Pac-Man history, not Space
  Invaders.
- **No web or iOS build**, because the deliverable is a native macOS artifact.

## 12. How to extend it without breaking the boundary

1. Add rules and state to `space-invaders-core`, with commands and events. Keep
   it free of `unsafe`, platform imports, and wall-clock time.
2. Add a unit test for the rule and, where the behavior is observable through the
   public API, an assertion in `space-invaders-checks`.
3. Add the visual representation in `scene.rs` and the art in `sprites.rs` or the
   font. The scene may read the new snapshot; it may not advance the game.
4. If the change is audible, add a `SoundCue`, map the event in
   `dispatch_event`, and keep the synth allocation-free.
5. Update `README.md`, `AGENTS.md`'s invariants, and the relevant Q&A entry.
6. Run `make check`, then launch the app. A clean build does not prove that a
   shader compiled or that a frame is visible.
