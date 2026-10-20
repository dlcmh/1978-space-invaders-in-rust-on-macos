# Contributing to SPACE INVADERS — Metal Arcade

Thanks for your interest. This is a compact, native macOS game with a deliberately
small dependency surface, and focused contributions are welcome.

Before changing anything, read [AGENTS.md](AGENTS.md),
[Docs/PROJECT_ORIGIN.md](Docs/PROJECT_ORIGIN.md),
[Docs/RUST_DESIGN.md](Docs/RUST_DESIGN.md), and [GOVERNANCE.md](GOVERNANCE.md).

## Prerequisites

- macOS 13 or newer on a Metal-capable Mac
- A stable Rust toolchain, 1.85 or newer, with `rustfmt` and `clippy`
- Git

No game engine, Xcode project, or runtime package manager is required. Python 3
and Pillow are optional and only used by `make icon`.

## Build and run

```bash
cargo build
cargo run -p space-invaders-app
```

Build the distributable app and disk image:

```bash
make app
make dmg
open dist/SPACE-INVADERS.app
```

Generated artifacts in `target/` and `dist/` are deliberately not versioned.
`Resources/AppIcon.png` and `Resources/AppIcon.icns` are generated but tracked so
the app bundle builds without Python.

## Verify changes

```bash
make check
```

That runs the full gate:

1. `cargo fmt --all -- --check`
2. `cargo clippy --workspace --all-targets --locked -- -D warnings`
3. `cargo test --workspace --locked`
4. `cargo run --release -p space-invaders-checks`
5. `cargo build --release --workspace --locked`
6. `python3 scripts/check_docs.py`

Run it before every commit. Individual stages are available as `make fmt`,
`make lint`, `make test`, `make checks`, and `make docs-check`.

Style is defined by [`rustfmt.toml`](rustfmt.toml) (`max_width`): edition 2024, 100 columns.
Apply it with:

```bash
cargo fmt --all
```

Clippy runs with `all` and `pedantic` enabled at warn level and warnings denied.
The workspace allow-list in [`Cargo.toml`](Cargo.toml) (`workspace.lints.clippy`) exists for lints that fight
the chosen pixel-math style; do not add broad blanket allows to individual files
just to silence a warning that points at a real simplification.

The standalone `SpaceInvadersChecks` executable is a regression suite over the
public simulation API. It covers formation geometry, bunker damage, scoring,
movement clamping, pause, enemy fire, passive game over, restart, deterministic
replay, and a 7,200-tick soak. Add checks whenever you change gameplay logic. Use
`Command`, `tick`, and the read-only accessors; do not add test-only mutation
hooks to production code.

For rendering, input, shader, windowing, or audio changes, launching the app is
mandatory: the Metal shader compiles at runtime, so a build alone cannot validate
it.

Documentation links and symbol references are checked too:

```bash
make docs-check
```

Prose cites a file and a symbol — never a line number — and the checker verifies
that every cited symbol still exists in the linked file. See
[AGENTS.md](AGENTS.md#documentation-and-code-references). After changing code,
`make docs-impact` lists the documents that reference the files you touched.

## Where to make changes

- **Simulation:** `crates/space-invaders-core/` — no platform imports, no unsafe,
  no dependencies.
- **Presentation and platform:** `crates/space-invaders-app/`.
- **Checks:** `crates/space-invaders-checks/src/main.rs`.
- **Packaging:** `scripts/`, `Makefile`, and `Resources/`.
- **User docs:** `README.md`.
- **Design and Q&A docs:** `Docs/RUST_DESIGN.md`, `Docs/METAL_CRASH_COURSE.md`,
  `Docs/QnA/`, and `Docs/Refactoring/`.
- **Governance docs:** `AGENTS.md`, `GOVERNANCE.md`, this file, `SECURITY.md`,
  and `Docs/PROJECT_ORIGIN.md`.

## Quality bar

A contribution is ready when:

- `cargo fmt --all -- --check` passes;
- `cargo clippy --workspace --all-targets -- -D warnings` passes;
- `cargo test --workspace` passes;
- `cargo run -p space-invaders-checks` passes;
- `make docs-check` passes, and docs affected by the change were re-read;
- rendering changes were visually checked by running the app;
- gameplay invariants documented in [AGENTS.md](AGENTS.md#gameplay-invariants)
  still hold, or the change explicitly updates them along with checks and docs;
- no new runtime dependency, telemetry, network access, or non-original asset
  was introduced; and
- user-facing documentation and screenshots are current.

## Style

- Follow the existing Rust formatting and naming.
- Keep functions and types focused; the core should stay readable and testable.
- Prefer private fields, explicit commands, and read-only snapshots over public
  mutable state.
- Preserve the fixed-step 60 Hz simulation. Do not make movement or timing depend
  on display refresh rate.
- Keep the simulation free of `unsafe`. At the platform boundary, every unsafe
  block needs a `SAFETY` comment explaining the invariant.
- Keep procedural art and audio procedural. Do not add arcade ROM content, ripped
  sprites, recorded arcade audio, or copyrighted fonts.
- Avoid adding dependencies. Discuss a proposed dependency with the maintainer
  before writing code around it.

## Workflow

1. Open an issue for a substantial behavior change or new scope.
2. Branch from `main` using a descriptive name, for example
   `feat/mystery-score-popups` or `fix/bunker-sweep`.
3. Make a coherent change with tests, checks, and docs.
4. Run the verification above.
5. Commit using a Conventional Commit message:
   - `feat: ...`
   - `fix: ...`
   - `docs: ...`
   - `perf: ...`
   - `refactor: ...`
   - `chore: ...`
6. Open a pull request describing the change, verification performed, and any
   known limitations.

Allowed direct pushes by repository agents are defined in `AGENTS.md`. Human
contributors should use pull requests unless the maintainer says otherwise.

## Reporting bugs

Use the repository issue tracker and include:

- macOS and Rust versions;
- steps to reproduce;
- expected and observed behavior;
- a screenshot or short recording for visual issues; and
- whether the issue reproduces after `make app` and a clean launch.

Please do not include secrets, personal information, or unredacted local paths in
public reports.

## Security

Do not open a public issue for a suspected vulnerability. Follow
[SECURITY.md](SECURITY.md).

## Conduct

Be constructive, specific, and respectful. Harassment, discrimination, personal
attacks, and deliberately disruptive behavior are not welcome. The maintainer may
moderate, close, or block participation to keep the project healthy.

## Licensing and intellectual property

SPACE INVADERS is a trademark of TAITO CORPORATION. This is an unofficial fan
project and must not be presented as affiliated with or endorsed by Taito.

No open-source license has been selected for this repository. Do not assume that
code, art, or audio may be reused or redistributed beyond this project. Only
submit original work or material with a documented compatible license. Adding a
license or changing the project's licensing requires explicit maintainer
approval.

For decisions about scope, releases, and repository policy, see
[GOVERNANCE.md](GOVERNANCE.md).
