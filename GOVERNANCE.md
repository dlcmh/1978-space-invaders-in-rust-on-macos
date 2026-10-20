# Governance

This document defines how **SPACE INVADERS — Metal Arcade** is maintained, how
decisions are made, and how releases and legal obligations are handled. It is
intentionally lightweight for a small fan project.

- **Canonical repository:** <https://github.com/dlcmh/1978-space-invaders-in-rust-on-macos>
- **Canonical branch:** `main`
- **Maintainer:** David Chin ([@dlcmh](https://github.com/dlcmh))
- **Governance baseline:** 2026-10-20
- **Version covered:** 1.0

## Mission and scope

The project is a native, offline, procedurally generated macOS tribute to the
1978 Space Invaders arcade game. Its quality priorities are faithful arcade
gameplay, clean Rust architecture, a dependency-light Metal renderer,
deterministic behavior, and reproducible builds.

### In scope

- The classic fixed-shooter loop: cannon movement, 55 invaders, accelerating
  formation, four destructible bunkers, enemy fire, mystery ship, lives, waves,
  scoring, and high scores.
- Native macOS presentation, input, synthesized sound, and local persistence.
- Procedural art, audio, font, icons, and app packaging.
- Headless simulation checks, documentation, and release tooling.
- The Rust workspace structure and the AppKit/Metal platform boundary.

### Explicitly excluded unless re-scoped

- Network play, online leaderboards, accounts, telemetry, and remote content.
- Web, iOS, Windows, or Linux ports.
- Original arcade ROMs, sprites, fonts, audio, or other proprietary assets.
- Two-player alternating mode in v1.0.
- ROM-cycle-accurate timing tables or undocumented hardware quirks.
- Paid distribution or any claim of official affiliation with Taito.

Scope changes require the process described under **Decision making**.

## Roles

### Maintainer

The maintainer has final responsibility for:

- repository direction, roadmap, and scope;
- merging and publishing changes;
- releases, tags, signing, and distribution;
- licensing and trademark decisions;
- security response; and
- amendments to governance documents.

The current sole maintainer is [@dlcmh](https://github.com/dlcmh).

### Contributors

Contributors propose fixes, features, documentation, and tests through issues,
pull requests, or maintainer-authorized direct pushes. Contributions are reviewed
against the quality bar in [CONTRIBUTING.md](CONTRIBUTING.md) and the
architectural rules in [AGENTS.md](AGENTS.md).

### Users

Users may report bugs and request features through the GitHub issue tracker.
Security reports must follow [SECURITY.md](SECURITY.md).

## Decision making

The project uses a **maintainer-led** model:

- Day-to-day fixes and documentation improvements proceed through normal review.
- Substantial gameplay changes, new dependencies, platform changes, licensing
  changes, and releases require explicit maintainer approval.
- Discussion happens in the relevant GitHub issue or pull request so the record
  remains public where appropriate.
- When consensus is unclear, the maintainer makes the final decision and records
  the rationale.

Architectural invariants in [AGENTS.md](AGENTS.md) are considered settled policy
until changed through the governance amendment process. Design alternatives that
were evaluated are recorded under [Docs/Refactoring/](Docs/Refactoring/README.md).

## Quality gates

Changes to `main` are expected to pass:

1. `cargo fmt --all -- --check`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
4. `cargo run -p space-invaders-checks`
5. `cargo build --release --workspace`
6. `python3 scripts/check_docs.py`
7. A manual launch for any rendering, input, audio, shader, or windowing change
8. Review of user-facing documentation and governance accuracy

`make check` runs the automated stages in order. Generated binaries are not
committed to source control. Distribution artifacts are produced through
`make app` and `make dmg` and published only when the maintainer authorizes a
release.

## Releases

Release versions follow semantic versioning for the application and its public
documentation.

A release requires the maintainer to:

1. confirm the checks and manual verification pass;
2. update the version in `Resources/Info.plist` and any `dist/` artifact naming;
3. confirm README, governance, and provenance records are current;
4. commit and tag the release if a tag is requested;
5. attach any `.app`/`.dmg` artifacts to a GitHub Release when distribution is
   intended; and
6. record notable changes and any compatibility notes.

Version 1.0 corresponds to the initial implementation and governance baseline.
No Git tag or public GitHub Release was created as part of the setup.

## Licensing and intellectual property

- **No open-source license has been selected for this repository.** Absent a
  separate license, default copyright applies and no permission to reuse or
  redistribute is granted.
- The maintainer reserves all licensing decisions. A license must not be added,
  changed, or implied by an agent or contributor without explicit approval.
- All committed code and generated art/audio must be original or properly
  licensed with attribution.
- Ripped or extracted assets from Space Invaders arcade hardware, ROMs, or
  official products are prohibited.
- SPACE INVADERS is a trademark of TAITO CORPORATION. This project is an
  unofficial fan tribute and is not affiliated with or endorsed by Taito.
- Public descriptions must retain this disclaimer and must not imply official
  status.

See [Docs/PROJECT_ORIGIN.md](Docs/PROJECT_ORIGIN.md) for the commissioning record
and scope decisions.

## Security and privacy

The game is offline, has no accounts, and does not collect telemetry. Only a
local high score and mute preference are stored. Security handling is defined in
[SECURITY.md](SECURITY.md).

## Repository and remote policy

- `origin` is the canonical GitHub remote:
  <https://github.com/dlcmh/1978-space-invaders-in-rust-on-macos.git>
- `main` tracks `origin/main`.
- Force-pushing, rewriting published history, changing the canonical remote, or
  adding another release remote requires maintainer approval.
- Agents follow the commit and push rules in [AGENTS.md](AGENTS.md).

## Dependency policy

The accepted dependency baseline is the `objc2` family and `block2` for Apple
framework access. These are bindings, not a game engine or runtime. A proposed
dependency requires maintainer approval and a review of its security, licensing,
maintenance, and binary-size impact. The lockfile is committed and release builds
use `--locked`.

The choice between `objc2` and `winit` + `metal-rs` is recorded in
[Docs/Refactoring/objc2-over-winit-metal-rs.md](Docs/Refactoring/objc2-over-winit-metal-rs.md).

## Amendments

Governance changes are proposed as a pull request or explicit maintainer
instruction and require maintainer approval. The change should state why the
current rule is insufficient, what replaces it, and any migration impact.

The **Original request** quotation in
[Docs/PROJECT_ORIGIN.md](Docs/PROJECT_ORIGIN.md) is immutable. Historical
decisions may be annotated but must not be silently rewritten.

## Contact

Use the GitHub issue tracker for project questions and
[SECURITY.md](SECURITY.md) for vulnerabilities.
