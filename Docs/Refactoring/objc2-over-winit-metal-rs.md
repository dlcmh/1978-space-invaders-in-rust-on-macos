# Direct `objc2` bindings instead of `winit` + `metal-rs`

- **Recorded:** 2026-10-20
- **Last updated:** 2026-10-20
- **Status:** Accepted
- **Decision owner:** the implementer, under the maintainer's commission

## Context

The project is a macOS-only native game. It needs a window that behaves like a
real Mac app (menu bar, first responder, keyboard focus, full-screen, local
preferences), a Metal renderer, and audio output. The two obvious starting points
in the Rust ecosystem are:

1. **`winit` + `metal-rs`**: a cross-platform event loop plus a widely used Metal
   wrapper.
2. **`objc2` + framework crates**: direct, safe-ish bindings to AppKit, Metal,
   Core Animation, and AudioToolbox.

The question is not whether either can render triangles. Both can. The question is
what the rest of the application looks like after the choice.

## Options considered

### A. `winit` + `metal-rs`

**Advantages**

- familiar cross-platform event model;
- large user base and examples;
- easier to add Windows/Linux later;
- `metal-rs` offers a higher-level wrapper over the Objective-C Metal API.

**Costs**

- two abstraction layers between the app and the platform it actually targets;
- the menu bar, About panel, first-responder chain, and `NSUserDefaults` still
  need Objective-C access or another crate;
- `winit` owns the event loop and window, yet AppKit still has to be initialized
  correctly for a polished macOS app;
- another GPU binding crate in the dependency graph, with its own release cadence
  and unsafe surface;
- full-screen and menu-key handling need to be reconciled between the two worlds.

### B. `objc2` + `objc2-app-kit` + `objc2-metal`

**Advantages**

- one platform story: an actual `NSApplication`, `NSWindow`, custom `NSView`,
  real menus, and a `CAMetalLayer`;
- direct access to the exact APIs the game uses, with no hidden event loop;
- framework crates are generated from Apple's headers and maintained together;
- the dependency family is small and shared (`objc2`, `objc2-foundation`,
  `objc2-app-kit`, `objc2-metal`, `objc2-quartz-core`,
  `objc2-audio-toolbox`, `block2`);
- `define_class!` lets the view own a timer, keyboard handlers, and responder-chain
  menu actions without an Objective-C file.

**Costs**

- macOS-only (already a commission requirement);
- more visible unsafe at the Apple-framework boundary;
- fewer Stack Overflow answers than `winit`;
- API names use the Objective-C selector spelling (`setContentView`,
  `addCompletedHandler`), which is verbose in Rust.

## Evidence from this implementation

- [`app.rs`](../../crates/space-invaders-app/src/app.rs) builds the application menu,
  About panel, Hide/Quit items, Game menu, and View menu through
  [`install_menu`](../../crates/space-invaders-app/src/app.rs) (`install_menu`). With
  `winit`, most of
  this would still need AppKit calls, so the cross-platform event loop would not
  remove platform code.
- [`view.rs`](../../crates/space-invaders-app/src/view.rs) subclasses `NSView` with
  [`define_class!`](../../crates/space-invaders-app/src/view.rs) (`GameView`) and receives
  `keyDown:`/`keyUp:` directly. The view is first responder, so menu shortcuts and
  keyboard input share one responder chain.
- [`preferences.rs`](../../crates/space-invaders-app/src/preferences.rs) stores the high
  score and mute flag in `NSUserDefaults` with no extra dependency.
- [`renderer.rs`](../../crates/space-invaders-app/src/renderer.rs) uses
  [`CAMetalLayer`](../../crates/space-invaders-app/src/renderer.rs) and one Metal
  pipeline with a 4× multisample resolve. `metal-rs` would have wrapped the same
  calls but could not remove the AppKit integration.
- [`audio.rs`](../../crates/space-invaders-app/src/audio.rs) uses `AudioQueue` directly
  through [`AudioEngine::new`](../../crates/space-invaders-app/src/audio.rs); no audio
  file, no AVFoundation graph, no extra mixer crate.
- The first release compiles to one native binary with no runtime asset bundle
  beyond the icon and `Info.plist`.

A counterfactual: if this were intended to ship on Windows and Linux too, option
A's cross-platform event loop would become a real advantage and the AppKit
integration would be a platform adapter behind a trait. It is not, so paying for
that abstraction now would be speculative.

## Decision

Use **option B**: direct `objc2` framework bindings, AppKit for the window and
menus, `CAMetalLayer` plus Metal for rendering, and `AudioQueue` for sound.

The boundary is deliberate:

- `space-invaders-core` remains platform-free and `#![forbid(unsafe_code)]`.
- All Objective-C/Metal interop is in `space-invaders-app`.
- Each unsafe block states the invariant that makes it sound.
- The app is one platform target, so platform-specific code is not a portability
  regression.

## Consequences

- The repository cannot be built on Linux. This is an accepted property of a
  native macOS deliverable, not an oversight.
- Adding a second platform would require a new presentation crate. The simulation
  is already ready for it.
- The `objc2` versions are pinned in `Cargo.lock` and updated as a family. A
  partial update is more likely to break type relationships between framework
  crates.
- Reviewers need to be comfortable reading Objective-C selector names in Rust.
  The [Metal crash course](../METAL_CRASH_COURSE.md) is the onboarding path.

## Revisit triggers

Revisit this decision if any of the following becomes true:

- a non-macOS target is commissioned;
- `winit` gains native AppKit menus and first-responder integration that would
  genuinely remove the platform code;
- the `objc2` family becomes unmaintained or cannot track a required macOS SDK;
- the app grows a multi-window or document architecture where a window
  controller abstraction would pay for itself.

Until then, adding `winit` would be a second platform abstraction beside one that
already works, not a simplification.
