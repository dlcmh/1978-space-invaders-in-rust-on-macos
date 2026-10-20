# Project Origin and Decision Record

This document preserves the request that created **SPACE INVADERS — Metal
Arcade** and records the decisions that shaped the delivered v1.0. It is a
historical and governance document. The **Original request** block below is
immutable.

- **Project:** SPACE INVADERS — Metal Arcade
- **Repository:** <https://github.com/dlcmh/1978-space-invaders-in-rust-on-macos>
- **Commissioned by:** David Chin ([@dlcmh](https://github.com/dlcmh))
- **Commissioned:** 2026-10-20 21:06:43 (+08:00, Asia/Kuala_Lumpur)
- **Delivery agent:** DeepSeek Harness (deepseek-flash)
- **Initial public version:** 1.0
- **Reference implementation for repository shape:** PAC-MAN — Metal Arcade
  (Swift), at the time under `/Users/dlcmh/dev/pac-man-arcade-swift-macos`

## Original request

The following is the human-authored request verbatim. Harness-supplied runtime
metadata that followed it — the working directory, file policy, approval policy,
GUI URL, and checkout path — is summarized separately and is not part of the
quote.

```text
reckon you can one-shot this game in clean Rust with Metal? Game description: "The original Space Invaders is a landmark fixed-shooter arcade game developed by Japanese designer Tomohiro Nishikado and released by Taito in 1978.

Key Details

• Developer & Designer: Tomohiro Nishikado
• Release Year: 1978 (released in Japan by Taito and distributed overseas by Midway Games)
• Genre: Fixed shooter / Shoot 'em up
• Core Gameplay:
	• Players control a horizontal laser cannon at the bottom of the screen to shoot descending waves of 55 alien ships.
	• The aliens move side-to-side in a zig-zag pattern and step closer to the bottom with each pass.
	• Four destructible defense bunkers provide temporary cover from enemy fire.
	• As you destroy more aliens, the remaining ships speed up (originally caused by processor load decreasing as sprites were cleared).
	• A bonus "Mystery Ship" occasionally crosses the top of the screen for extra points."

Do use /Users/dlcmh/dev/pac-man-arcade-swift-macos as a reference for how i'd like this repo and game artifacts to be shaped, developed and governed; additionally for game artifacts, kindly use modern rust swe best practices for architecture and coding style, document your rust choices

set this repo's upstream to https://github.com/dlcmh/1978-space-invaders-in-rust-on-macos.git
```

## Requirements captured from the request

1. Deliver a playable, original-style Space Invaders arcade game in **clean
   Rust** with **Metal** rendering on macOS.
2. Implement the brief's canonical loop: a horizontal cannon, 55 descending
   invaders, side-to-side formation movement with a downward step, four
   destructible bunkers, acceleration as invaders are destroyed, and an
   occasional mystery ship.
3. Shape the repository and its artifacts after the referenced Pac-Man project:
   a native app bundle, procedural assets, a headless simulation, a standalone
   checks harness, a documentation set, and governance documents.
4. Apply modern Rust software-engineering practice and **document the Rust
   choices**, not merely the feature list.
5. Set the repository's upstream to
   <https://github.com/dlcmh/1978-space-invaders-in-rust-on-macos.git>.
6. Deliver the result in one implementation pass rather than a mock-up or a
   partial prototype.

The historical details in the brief were treated as context. The exact 1978
cabinet's hardware quirks, two-player alternating mode, and ROM-accurate timing
tables were not required; the described gameplay loop was.

## Runtime context at commissioning

- Workspace: `/Users/dlcmh/dev/1978-space-invaders-in-rust-on-macos`
- Host platform: macOS 26.6.2 on Apple silicon (M5), Metal 4
- Toolchain: rustc 1.97.1, cargo 1.97.1, Command Line Tools
- Interaction surface: DeepSeek Harness Web GUI
- File policy: danger-full-access; approvals disabled
- Repository state at commissioning: empty local directory, not yet a Git
  repository

These facts explain implementation choices and are not ongoing requirements.

## Delivered implementation

- Native macOS application written in Rust, with a Cargo workspace split into a
  headless simulation crate, an AppKit/Metal executable, and a checks executable.
- The canonical 55-invader formation: 11 squid, 22 crab, 22 octopus, with 30/20/10
  point values.
- Formation movement that steps sideways, reverses and drops at the walls,
  alternates animation frames, and accelerates as invaders are destroyed.
- Player movement and fire, enemy fire from the lowest invader in a column,
  bullet cancellation, damaged bunkers, and invader crushing of cover.
- Four 22 × 16 destructible bunkers with persistent, ragged craters.
- Mystery ship with a deterministic fifteen-shot scoring cycle.
- Lives, extra life, waves, high score, attract, ready, pause, death, wave-clear,
  and game-over states.
- Immediate-mode Metal renderer: a single pipeline, one draw call, triple-buffered
  vertex data, 4× MSAA, letterboxing, procedural sprites, a 5 × 7 pixel font,
  starfield, explosions, and CRT scanlines.
- Procedural audio through `AudioQueue` with a sixteen-voice square/triangle/saw/
  noise synthesiser. No audio files.
- Local persistence for high score and mute preference only.
- App bundle, DMG target, generated application icon, documentation checker, and
  a `make check` gate.
- Governance and project documentation adapted from the reference repository.

## Explicit scope decisions

| Topic | Decision |
| --- | --- |
| Platform | Native macOS; no web, iOS, or Windows build in v1.0. |
| Language | Rust throughout, with no Swift/C/Objective-C source outside framework bindings. |
| Windowing | AppKit directly through `objc2`; no `winit` or other cross-platform shell. |
| Rendering | Metal through `objc2-metal` and a `CAMetalLayer`; no game engine, sprite kit, or MetalKit. |
| Assets | No original arcade sprites, ROM data, fonts, or audio. Everything is procedural and original. |
| Audio | Synthesised at runtime through `AudioToolbox`; no audio files. |
| Simulation | Deterministic 60 Hz fixed-step; platform-free; `#![forbid(unsafe_code)]`. |
| Persistence | Local `NSUserDefaults` only (high score and mute). |
| Two-player mode | Not included; single-player cabinet loop only. |
| Historical quirks | Arcade speed table approximated with a deterministic formation interval; not cycle-accurate. |
| Network | Fully offline. No leaderboards, telemetry, or downloads. |
| Licensing | No open-source license selected. See [GOVERNANCE.md](../GOVERNANCE.md). |
| Trademark | Unofficial fan tribute; not affiliated with or endorsed by Taito. |

## Verification record

The v1.0 baseline was built and launched on the commissioning host. At the time
of writing:

- `cargo test --workspace` passes 33 tests, including the doctest.
- `SpaceInvadersChecks` passes **9,460** behavioral checks.
- `cargo clippy --workspace --all-targets -- -D warnings` is clean.
- `cargo fmt --all -- --check` is clean.
- `scripts/check_docs.py` validates every link, heading anchor, and cited Rust
  symbol.
- The release `.app` bundle was built, ad-hoc signed, launched, played, quit, and
  relaunched. High-score persistence across launches was verified with
  `defaults read dev.dlcmh.space-invaders-rust-metal`.
- Window captures are recorded as [gameplay.png](gameplay.png) and
  [title-screen.png](title-screen.png).

The checks cover the invader roster and geometry, bunker construction and damage,
player movement and clamping, firing and scoring, pause, enemy fire, the passive
round that ends in game over, restart, deterministic replay, and a 7,200-tick
input soak. Unit tests additionally cover wave progression, extra life, mystery
ship collision, and the three-life death sequence.

## Governance directive

The same request asked for the reference project's repository shape, development
practice, and governance, and for the upstream remote to be set. That established
[AGENTS.md](../AGENTS.md), [CONTRIBUTING.md](../CONTRIBUTING.md),
[GOVERNANCE.md](../GOVERNANCE.md), [SECURITY.md](../SECURITY.md), this record,
and the [Q&A](QnA/README.md) and [Refactoring](Refactoring/README.md) indexes.
The canonical remote is `origin`:
<https://github.com/dlcmh/1978-space-invaders-in-rust-on-macos.git>.

## Record maintenance

- Do not edit the **Original request** quotation.
- Append material decisions rather than silently rewriting history.
- Update the verification section when the release baseline changes.
- Changes to governance itself follow [GOVERNANCE.md](../GOVERNANCE.md).
