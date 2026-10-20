# How is the Rust workspace split between simulation and presentation?

- **Recorded:** 2026-10-20
- **Last updated:** 2026-10-20

## Short answer

Three Cargo crates enforce the boundary. `space-invaders-core` owns every game
rule and has no platform dependencies. `space-invaders-app` owns AppKit, Metal,
audio, and persistence. `space-invaders-checks` consumes only the public core API.
The renderer reads a finished game snapshot and never writes gameplay state.

## The dependency graph

```text
space-invaders-core  (no dependencies, forbids unsafe)
        ▲
        ├────────────── space-invaders-app
        │                   objc2, objc2-app-kit, objc2-metal,
        │                   objc2-quartz-core, objc2-audio-toolbox, block2
        └────────────── space-invaders-checks
                            (core only)
```

The direction matters. The app depends on the core; the core cannot depend on the
app. It does not import AppKit, Metal, or audio, and a future headless replay tool
could use the same crate without a window.

The core starts with `#![forbid(unsafe_code)]` in
[`lib.rs`](../../crates/space-invaders-core/src/lib.rs) (`forbid`). That guarantee is
the strongest expression of the split: no simulation rule can reach a raw pointer.

## Commands in, snapshots out

Input never mutates fields. The view translates a key code into a
[`Command`](../../crates/space-invaders-core/src/game.rs) and calls
[`Game::command`](../../crates/space-invaders-core/src/game.rs). Time advances with
[`Game::tick`](../../crates/space-invaders-core/src/game.rs), which is total and takes no
arguments.

Presentation reads immutable slices:

- [`Game::aliens`](../../crates/space-invaders-core/src/game.rs)
- [`Game::bunkers`](../../crates/space-invaders-core/src/game.rs)
- [`Game::enemy_bullets`](../../crates/space-invaders-core/src/game.rs)

Sound and persistence are driven by data, not callbacks. The core queues
[`GameEvent`](../../crates/space-invaders-core/src/game.rs) values and the app drains
them with [`Game::take_events`](../../crates/space-invaders-core/src/game.rs). This is
why the core can be tested without mocking an audio device.

## The fixed-step loop

[`view.rs`](../../crates/space-invaders-app/src/view.rs) (`frame_tick`) is the only
clock. It measures elapsed wall time, adds it to an accumulator, and runs at most
eight fixed 1/60-second ticks. If the app stalls during a window drag, the
accumulator is capped instead of trying to replay an unbounded backlog.

The simulation has no access to `Instant`, `SystemTime`, or a random device. That
is why `Game::with_seed` can reproduce a round exactly and why the checks can run
7,200 ticks in milliseconds.

## Why not put everything in one crate?

It would compile, but the boundary would become a matter of discipline. Someone
could read a window size inside collision code "just this once", and the
simulation would stop being headless. Cargo's dependency graph makes that a
compile error rather than a review comment.

The cost is a little module ceremony and a second crate manifest. The benefit is
a deterministic, testable core and a presentation layer that can be replaced
without touching the game.

## Where the presentation lives

- [`view.rs`](../../crates/space-invaders-app/src/view.rs) (`GameView`) owns the timer,
  keyboard, audio dispatch, and persistence.
- [`renderer.rs`](../../crates/space-invaders-app/src/renderer.rs) (`Renderer`) owns
  Metal objects and one frame's GPU work.
- [`scene.rs`](../../crates/space-invaders-app/src/scene.rs) (`build`) turns the
  snapshot into triangles.
- [`audio.rs`](../../crates/space-invaders-app/src/audio.rs) (`AudioEngine`) turns cues
  into samples.

None of those files are visible to the core, and none of them can change an
invader's position.
