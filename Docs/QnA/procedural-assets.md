# When would sprite sheets, a game engine, texture assets, or runtime downloads be needed?

- **Recorded:** 2026-10-20
- **Last updated:** 2026-10-20

## Short answer

Each of those four things solves a problem this game does not have yet. A sprite
sheet or texture atlas pays off when the art volume outgrows hand-written ASCII
masks. A game engine pays off when the game needs tools, a second platform, or
content that a team of non-Rust authors must edit. Runtime downloads pay off only
when content lives on a server. Today every one of those would add a dependency,
a failure mode, and a licensing obligation in exchange for nothing measurable, so
the project holds the line in
[`AGENTS.md`](../../AGENTS.md#non-negotiable-architecture) (rule 3 and rule 4).

The useful question is not "is this professional?" but "what specifically is
blocking me?" If the answer is "I want a third invader frame", add ASCII rows to
[`sprites.rs`](../../crates/space-invaders-app/src/sprites.rs) (`Sprite`). If the
answer is "I have 400 hand-painted animation frames from an artist", you have
found a real reason to talk to the maintainer about a texture pipeline.

## What "asset-free" actually means here

It does not mean the game has no art. It means the art is **source code**, so it
is reviewed, diffed, and versioned like any other logic.

| Visual or sound | How it is produced | Symbol |
| --- | --- | --- |
| Invaders, cannon, mystery ship, bursts, spark | ASCII `#`/`.` masks | [`Sprite`](../../crates/space-invaders-app/src/sprites.rs) |
| Masks → geometry | Row runs merged into quads, no textures | [`Canvas::sprite`](../../crates/space-invaders-app/src/canvas.rs) |
| HUD text | 5 × 7 bitmaps, one `match` arm per character | [`glyph`](../../crates/space-invaders-app/src/pixel_font.rs) |
| Starfield and horizon glow | Closed-form arithmetic on the tick count | [`draw_starfield`](../../crates/space-invaders-app/src/scene.rs) |
| Scanlines | Thin translucent quads | [`draw_scanlines`](../../crates/space-invaders-app/src/scene.rs) |
| Music and effects | Square, triangle, saw, and noise voices synthesised into the audio buffer | [`Synth`](../../crates/space-invaders-app/src/audio.rs) |
| Shaders | MSL text compiled at launch | [`SHADERS`](../../crates/space-invaders-app/src/metal_shaders.rs) |
| App icon | Script run by `make icon`, output committed | [`make_icon.py`](../../scripts/make_icon.py) (`main`) |

Two things look like texture assets but are not. The MSAA colour target created by
[`ensure_multisample`](../../crates/space-invaders-app/src/renderer.rs) is a
render target the GPU draws into, not art sampled by the fragment shader. And the
fragment shader in [`metal_shaders.rs`](../../crates/space-invaders-app/src/metal_shaders.rs)
(`fragment_main`) returns the interpolated vertex colour, so there is no sampler
in the pipeline at all. That is why one draw call can render the whole frame: the
vertex buffer already contains the finished picture.

The icon is the one place where a generator produces a file. Note the shape of
it: Python and Pillow run on the developer's machine, never at launch, and the
resulting `Resources/AppIcon.icns` is tracked in git. That is **build-time
generation**, not an asset pipeline, and it is the pattern to copy if you ever
need a generated binary blob.

## Sprite sheets and texture atlases

A sprite sheet is a single image holding many frames; an atlas is the same idea
packed for efficient GPU sampling. Adopt one when at least one of these is true:

- **Frame volume.** Dozens of frames per actor, or many actors. Each new sprite
  here is currently a small diff to one struct; a sheet would be a binary blob
  whose contents a reviewer cannot see in a pull request.
- **Art that is not on the pixel grid.** The mask format stores one bit per
  pixel: opaque or empty, one colour per sprite, drawn with hard edges. Soft
  edges, gradients, glows, or per-pixel colour need a source image.
- **An artist's actual toolchain.** If the art arrives as PNGs from Aseprite or
  Photoshop, forcing it back into ASCII is wasted work.
- **A vertex budget problem.** [`Canvas::sprite`](../../crates/space-invaders-app/src/canvas.rs)
  emits one quad per horizontal run, and [`MAX_VERTICES`](../../crates/space-invaders-app/src/canvas.rs)
  caps the ring buffer at 100,000. If a frame ever approaches that ceiling, a
  textured quad per sprite is the fix. At today's sprite sizes that ceiling is
  not close, and the run-merging keeps the count low.

What it costs: a `MTLTexture` and sampler in the pipeline, UV coordinates in
[`Vertex`](../../crates/space-invaders-app/src/canvas.rs), an atlas packer or a
build step, filtering and pixel-snapping decisions so the art stays crisp at
non-integer window scales, and a licensing record proving every frame is
original. [`AGENTS.md`](../../AGENTS.md#rendering-and-audio-guidance) explicitly
requires "a measured need and an explicit design record" before any asset
pipeline lands. "It would look nicer" is not a measurement.

## A game engine

Engines earn their keep by supplying an editor, a scene graph, a renderer, a
physics system, UI, input mapping, and multi-platform export. This game needs a
fixed 224 × 256 playfield, two keys and a mouse-free menu, one draw call, and a
deterministic 60 Hz tick that already fits in one file
([`view.rs`](../../crates/space-invaders-app/src/view.rs) — `frame_tick`).

Reconsider an engine when the project genuinely changes shape:

- a second platform (Windows, Linux, web) becomes a requirement;
- the game becomes 3D, or needs skeletal animation, lighting, or a particle
  system;
- non-programmers must build levels or tune gameplay in a visual editor;
- the team stops being comfortable in Rust;
- online multiplayer or matchmaking becomes a real feature.

The cost is not just the dependency. The engine would want to own the frame
loop, which breaks the fixed-step accumulator; it would own the renderer, which
retires the Metal path and the
[`Docs/METAL_CRASH_COURSE.md`](../METAL_CRASH_COURSE.md) knowledge with it; and it
would tempt gameplay rules out of the headless core, undoing the boundary
described in
[the architecture entry](architecture.md). The deterministic checks in
[`main.rs`](../../crates/space-invaders-checks/src/main.rs) (`main`) exist because
the simulation has no engine beneath it.

## Texture assets, more broadly

Beyond sprites, "texture assets" usually means background art, a title screen
image, a bigger font, particle sprites, or Retina-resolution art. Ask what each
buys:

- **A richer title screen?** The attract screen is composed by
  [`build`](../../crates/space-invaders-app/src/scene.rs) from the same
  primitives. A painted background would be the first true texture, and the
  first time the game needs a file that can be missing or corrupt at launch.
- **A bigger font?** [`glyph`](../../crates/space-invaders-app/src/pixel_font.rs)
  covers A–Z and digits at 5 × 7. Adding a glyph is a few lines; adding a
  licensed typeface is a licensing question, not a rendering question.
- **Higher-resolution art?** The renderer draws vector triangles at the design
  resolution and letterboxes, so it scales without a texture budget. A bitmap
  would need 1×/2× variants and a selection rule.

Adopt texture files when the content is genuinely unmaintainable as code, and
bring an original-art provenance note plus a design record when you do.

## Runtime downloads

This is the easiest line to hold: nothing may be fetched at launch or during
play. The game is offline by design, stated in
[`SECURITY.md`](../../SECURITY.md) and in
[`AGENTS.md`](../../AGENTS.md#non-negotiable-architecture) (rule 4 and the
persistence rules). The only persisted state is two `NSUserDefaults` keys,
`SpaceInvadersArcadeHighScore` and `SpaceInvadersArcadeMuted`, read and written by
[`load_high_score`](../../crates/space-invaders-app/src/preferences.rs) and
[`save_muted`](../../crates/space-invaders-app/src/preferences.rs).

Downloads would only become necessary if the product changed into something else:

- a live service with rotating content or remote configuration;
- online leaderboards, accounts, or cloud saves;
- content too large to ship in the bundle, streamed on demand;
- server-authoritative multiplayer;
- telemetry or crash reporting.

Each of those turns a self-contained `.app` into a client with a backend, a
privacy policy, an availability budget, and a failure path for "no network". It
also breaks a property worth keeping: the game launches and is fully playable on
a plane, and `make check` never needs the internet. If a feature seems to require
a download, the usual answer is to ship the data in the bundle or generate it, and
then to ask the maintainer before doing either.

## The escalation ladder

Try these in order; stop at the first rung that solves the problem.

1. **Add data to the existing tables.** A new invader frame is rows in
   [`sprites.rs`](../../crates/space-invaders-app/src/sprites.rs) — `Sprite`. A
   new character is an arm in
   [`pixel_font.rs`](../../crates/space-invaders-app/src/pixel_font.rs) — `glyph`.
2. **Generate it from a formula.** Stars, glow, and scanlines are a handful of
   lines in [`draw_starfield`](../../crates/space-invaders-app/src/scene.rs); a
   procedural pattern often beats a hand-drawn one for this art style.
3. **Generate it at build time and commit the output**, following the
   [`main`](../../scripts/make_icon.py) entry point of the icon generator. The
   game still opens no art files at launch.
4. **Embed a file you authored** with `include_bytes!` and draw it as a texture.
   This needs the design record and maintainer approval.
5. **Atlas, engine, or network.** These are architectural changes, not features.
   Bring a measured problem, not a preference.

## The one-question test

Before adding any of the four, answer this: *does the change make the game open a
file it did not author, or contact a machine it does not control?* If yes, stop
and ask. If no, the change is probably just code, and code is what this project
is made of.
