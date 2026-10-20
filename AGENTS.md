# AGENTS.md

Operating instructions for AI coding agents working in this repository. These
rules are mandatory unless the maintainer gives a direct instruction that
supersedes them. Update this file when project policy changes.

- **Project:** SPACE INVADERS — Metal Arcade
- **Canonical repository:** <https://github.com/dlcmh/1978-space-invaders-in-rust-on-macos>
- **Maintainer:** [@dlcmh](https://github.com/dlcmh)
- **Governance baseline:** 2026-10-20
- **Current release:** 1.0

## Read first

1. [Docs/PROJECT_ORIGIN.md](Docs/PROJECT_ORIGIN.md) — why this project exists,
   including the original request. Its quoted request is a historical record and
   must not be edited.
2. [GOVERNANCE.md](GOVERNANCE.md) — decision-making, releases, licensing, and
   amendment rules.
3. [CONTRIBUTING.md](CONTRIBUTING.md) — build, test, and contribution workflow.
4. [SECURITY.md](SECURITY.md) — vulnerability reporting and privacy posture.
5. [Docs/RUST_DESIGN.md](Docs/RUST_DESIGN.md) — the architecture and the rationale
   behind the Rust choices.

## Project layout

| Path | Responsibility |
| --- | --- |
| `crates/space-invaders-core/` | Pure Rust simulation. No platform imports, no unsafe, no dependencies. |
| `crates/space-invaders-app/` | macOS app, AppKit window, Metal renderer, input, audio, persistence. |
| `crates/space-invaders-checks/` | Dependency-free behavioral checks over the public core API. |
| `Resources/` | App metadata and generated icon files. |
| `Docs/` | Screenshots, origin record, Rust design notes, Metal crash course, code Q&A under `Docs/QnA/`, and Refactoring records under `Docs/Refactoring/`. |
| `scripts/` | Deterministic app packaging, icon generation, docs checking, and the `make check` gate. |
| `dist/` | Generated, git-ignored app and DMG artifacts. Never commit it. |

## Prerequisites

- macOS 13 or newer on a Metal-capable Mac.
- A stable Rust toolchain, 1.85 or newer, with `rustfmt` and `clippy`.
- Python 3 and Pillow only for the optional icon generator; the game does not
  need Python at runtime.
- No game engine, no Xcode project, and no runtime network services.

The dependency set is intentionally limited to the `objc2` family and `block2`.
Do not add a game engine, a windowing runtime, an async runtime, an ECS, or a
logging framework without explicit maintainer approval.

## Build, run, and verify

Run these from the repository root:

```bash
cargo build                         # compile the workspace
cargo run -p space-invaders-app     # launch the game
cargo test --workspace              # unit, scene, renderer, and audio tests
cargo run -p space-invaders-checks  # behavioral checks over the public API
make app                            # build the release app bundle
make dmg                            # build dist/SPACE-INVADERS-1.0.dmg
make check                          # the full gate: fmt, clippy, tests, checks, release, docs
make checks                         # alias for the checks executable
make docs-check                     # validate doc links and cited symbols
```

`cargo build` is necessary but not sufficient for rendering changes. The Metal
shader is compiled at runtime, so a successful build can still launch into an
error message if the MSL is invalid. After touching shaders, renderer code,
windowing, input, or audio, launch the app and confirm it actually runs.

## Non-negotiable architecture

1. **Keep simulation and presentation separate.** `space-invaders-core` must
   remain headless and platform-free. Do not import `objc2`, AppKit, Metal, or
   audio crates there. Do not add `unsafe` to that crate.
2. **Keep Metal.** Use the existing single-pass, immediate-mode triangle renderer
   and `CAMetalLayer`. Do not replace it with a game engine or a high-level
   drawing crate.
3. **Keep assets procedural.** Sprites, font, icon, starfield, scanlines, and
   audio are generated in code. Do not add ripped arcade sprites, ROM data,
   scans, or copyrighted audio.
4. **Keep the game offline.** No network calls, telemetry, analytics, remote
   configuration, or account systems. The only persisted state is local
   `NSUserDefaults`.
5. **No new dependencies** without explicit maintainer approval. The `objc2`
   framework crates and `block2` are the accepted baseline.
6. **Authoritative simulation is `Game`.** Do not duplicate gameplay rules in the
   renderer or app layer. [`scene.rs`](crates/space-invaders-app/src/scene.rs)
   (`build`) must read the game, never mutate it.
7. **Keep the game loop deterministic and fixed-step.** The app runs simulation
   ticks at 60 Hz with a bounded catch-up loop in
   [`view.rs`](crates/space-invaders-app/src/view.rs) (`frame_tick`). Avoid
   frame-rate-dependent movement, wall-clock timers inside the simulation, or
   non-deterministic random sources.
8. **Preserve public read-only state.** Prefer private fields and read-only
   accessors on [`Game`](crates/space-invaders-core/src/game.rs). Commands drive
   changes; checks and presentation use the public API rather than test hooks.
9. **Keep unsafe at the platform boundary.** `space-invaders-core` forbids
   unsafe code. In the app, every unsafe block needs a `SAFETY` comment that
   names the Objective-C, Metal, or pointer invariant it relies on.
10. **Keep the app macOS-native.** Use AppKit menus, first responder, and
    `NSUserDefaults`. Do not replace the shell with a cross-platform runtime
    without a governance decision.

## Gameplay invariants

Changes that affect formation, collision, or scoring must preserve or deliberately
update these invariants and their checks:

- The formation has 55 invaders: 11 squid, 22 crab, 22 octopus.
- Point values are 30, 20, and 10 respectively.
- The formation steps 2 pixels sideways, drops 8 pixels at a wall, and reverses.
- The step interval shrinks as invaders are destroyed and is bounded below at
  2 ticks.
- One player bullet may be in flight at a time; it moves 4 pixels per tick.
- Enemy bullets move 1.5 pixels per tick and are capped at three at level 1.
- There are four 22 × 16 destructible bunkers, restored at the start of each
  wave, with persistent damage within a wave.
- The mystery ship's value follows the documented fifteen-shot cycle.
- The player has three lives, one bonus life at 1,500 points, and respawns after
  a death animation unless lives are exhausted.
- Pause freezes the simulation; the timer continues to render.
- The attract screen is the initial phase, and Return or Space starts a round.
- The README documents the controls and the current verification counts
  accurately.

## Static checks

`make check` runs [`scripts/check.sh`](scripts/check.sh) (`CARGO`), the gate every
change must pass:

1. `cargo fmt --all -- --check` — formatting from
   [`rustfmt.toml`](rustfmt.toml) (`max_width`, edition 2024).
2. `cargo clippy --workspace --all-targets --locked -- -D warnings` — all and
   pedantic lints, with a short explicit allow-list in the workspace manifest.
3. `cargo test --workspace --locked` — unit tests, scene and renderer tests, and
   the doctest.
4. `cargo run --release -p space-invaders-checks` — public-API behavior checks.
5. `cargo build --release --workspace --locked` — optimized build.
6. `python3 scripts/check_docs.py` — documentation links and cited symbols.

There is no third-party formatter, linter, or test runner beyond Cargo, Clippy,
rustfmt, and the standard library. Apply formatting with:

```bash
cargo fmt --all
```

Do not add `cargo-audit`, `nextest`, `tarpaulin`, or other tools to the required
local gate without a governance decision. They may be useful locally; the gate
must remain runnable with the documented prerequisites.

## Testing standard

The regression suite has three layers:

- Unit tests in `space-invaders-core` may inspect private state to reach edge
  cases such as wave transitions, landing, extra life, and mystery collision.
- [`game.rs`](crates/space-invaders-core/src/game.rs) (`Game::with_seed`)
  provides deterministic replay for tests without exposing mutable state.
- [`main.rs`](crates/space-invaders-checks/src/main.rs) (`main`) uses only the public API
  and must stay dependency-free.

Add checks when changing gameplay, movement, collision, scoring, or state
transitions. A green baseline currently ends with:

```text
PASS: 9460 checks
```

Counts may grow. The important requirement is that all checks pass. For pure
visual or packaging changes, also rebuild and open the resulting `.app`.

The scene, renderer, audio, font, and sprite modules have their own unit tests for
pure geometry and synthesis behavior. Do not rely on those tests as a substitute
for launching the game after a visual change.

## Documentation and code references

A line number in prose is wrong the moment ten lines are inserted above it, and
nothing fails. Documentation therefore cites a **file and a symbol**, never a line
range:

```markdown
[`game.rs`](crates/space-invaders-core/src/game.rs) — `tick_playing`
```

- Link the file, then name the symbol in backticks on the same line or in the
  same sentence.
- Prefer the most specific symbol — a function, method, type, or constant — over
  a general word.
- Link Markdown by heading anchor
  (`../AGENTS.md#non-negotiable-architecture`), never by line.
- When a symbol is renamed or removed, update every document that cites it.

`make docs-check` enforces this: every relative link must resolve, every heading
anchor must exist, every cited symbol must still occur in the linked file, and
`#L` line anchors are rejected.

`make docs-impact` is the answer to "which docs does this code change touch?".
It lists the documents that reference files changed against `HEAD` (or
`--base <rev>`) and warns when the diff removes a symbol one of them cites. Treat
that list as the re-read queue after changing code, and fix any reference the
change invalidated.

[Docs/PROJECT_ORIGIN.md](Docs/PROJECT_ORIGIN.md) is a historical record: its
quotation is immutable, so correct a stale reference there by annotating outside
the quote rather than rewriting it. A single line can opt out of the symbol rule
with `<!-- docs-check: ignore -->`.

## Git and release workflow

- `main` is the canonical trunk.
- `origin` is <git@github.com:dlcmh/1978-space-invaders-in-rust-on-macos.git>.
- Use Conventional Commit messages, for example:
  - `feat: add mystery-ship score popups`
  - `fix: prevent bunker damage from skipping a descending bullet`
  - `docs: record the Rust boundary`
  - `chore: refresh application icon`
- Keep commits coherent and reviewable. Do not mix unrelated refactors with
  gameplay changes.
- Run `make check` before committing; it covers the release build and the checks.
- The maintainer has authorized normal agent commits and pushes in this
  repository.
- Never force-push `main`, rewrite published history, or change remotes without
  explicit maintainer approval.
- Do not commit `target/`, `dist/`, `.DS_Store`, `Resources/AppIcon.iconset/`, or
  other generated intermediates. The generated `Resources/AppIcon.png` and
  `Resources/AppIcon.icns` are intentionally tracked.
- Do not tag or publish a GitHub Release unless the maintainer asks.

## Rendering and audio guidance

- Keep the presentation at the 224 × 256 design resolution and letterbox it
  cleanly when the window aspect changes.
- Maintain 4× MSAA where supported and the existing triple-buffered vertex path.
- Respect the established palette, pixel font, arcade HUD, and scanline look.
- Screenshots in `Docs/` are user-facing documentation. Refresh them when a
  material visual change invalidates them.
- Sound effects are synthesized at launch. Keep the mute setting persisted and do
  not introduce audio files or network-streamed audio.
- Do not add a texture atlas or asset pipeline without a measured need and an
  explicit design record.

## Persistence and privacy

The app stores only two local preferences:

| Key | Purpose |
| --- | --- |
| `SpaceInvadersArcadeHighScore` | Persistent local high score |
| `SpaceInvadersArcadeMuted` | Sound preference |

Do not add analytics, identifiers, crash uploaders, or remote persistence. If a
new stored preference is unavoidable, document it here and in [SECURITY.md](SECURITY.md).

## Legal and licensing

- SPACE INVADERS is a trademark of TAITO CORPORATION. This is an unofficial fan
  tribute and must never be presented as affiliated with or endorsed by Taito.
- No open-source license has been selected for this repository. Do not add a
  license, relicense code, or claim reuse rights without explicit maintainer
  approval.
- Only contribute original code and generated art/audio, or material whose
  license is compatible and documented.
- Do not include arcade ROMs, extracted sprites, fonts, or recorded audio.

## Agent operating protocol

1. Read this file, the origin record, the Rust design notes, and the relevant
   source before editing.
2. Inspect `git status` and preserve unrelated local work.
3. Make the smallest coherent change that satisfies the request.
4. Add or update tests/checks for behavioral changes.
5. Build, run `make check`, and launch the app for presentation changes.
6. Update README/governance docs when behavior, controls, build steps, or policy
   changes, then run `make docs-check` and resolve the docs listed by
   `make docs-impact`.
7. Commit with a Conventional Commit message and push to `origin/main` when
   authorized.
8. Report what changed, what was verified, and any remaining risks or follow-up
   work.

Do not claim to have run something you did not run. If the environment prevents a
verification step, say exactly what was blocked.

## Definition of done

- [ ] `make check` passes: fmt, clippy, tests, checks, release build, docs.
- [ ] `cargo run -p space-invaders-checks` passes.
- [ ] `make docs-check` passes, and docs listed by `make docs-impact` were
      re-read after code changes.
- [ ] The app launches and renders correctly when presentation changed.
- [ ] Gameplay and controls documentation is current.
- [ ] Governance and origin records remain accurate.
- [ ] No generated binaries or secrets are staged beyond the tracked icon files.
- [ ] Change is committed and pushed, if authorized.

## When to stop and ask the maintainer

Ask before:

- changing licensing, repository visibility, or the canonical remote;
- adding third-party dependencies, assets, telemetry, or network access;
- making incompatible gameplay changes or altering the canonical formation;
- cutting a release, tag, or public distribution build;
- force-pushing or rewriting published history;
- adding a second platform target or replacing AppKit/Metal with another stack;
- implementing large optional scope such as a two-player mode or a level editor.
