# Metal Crash Course

**Audience:** someone who writes ordinary CPU code — perhaps for the web — and
has never touched a GPU API. No Metal, shader, or linear-algebra knowledge is
assumed.

This is not a general Metal textbook. It is a guided tour of the Metal concepts
this repository actually uses, in roughly the order a frame encounters them. By
the end you should be able to open the renderer, canvas, scene, and shader source
and follow what every line is doing.

The one-sentence version:

> Sixty times a second, the CPU turns the game state into a flat list of coloured
> triangles, hands that list to the GPU in one buffer, and the GPU runs two tiny
> programs to position and colour those triangles on screen.

---

## 1. What Metal is — and is not

Metal is Apple's low-level API for talking to the GPU. The closest web
equivalents are **WebGL** and **WebGPU**. It is not a game engine, a scene graph,
or a drawing library. Metal knows nothing about invaders, bunkers, sprites,
cameras, or scoring. It knows about buffers of bytes, shader programs, and draw
calls.

Three Apple frameworks appear in this project:

| Framework | Job here |
| --- | --- |
| **AppKit** | Window, application menu, keyboard focus, clock timer, preferences |
| **Core Animation (QuartzCore)** | `CAMetalLayer`, the surface that owns drawable textures |
| **Metal** | GPU device, command queue, buffers, pipeline, render pass, shaders |

The GPU is an extremely wide, fairly dumb workforce. It is brilliant at doing the
same small calculation to millions of vertices or pixels at once. It is bad at
branching, game rules, and doing one thing at a time. So this project gives it
exactly one job: **paint triangles**.

### What the GPU does not do here

- It does not move the cannon, choose invader steps, detect collisions, or keep
  score. Those are CPU-side rules in `space-invaders-core`.
- It does not synthesise audio. `AudioQueue` and a software synth run on the CPU.
- It does not use compute shaders, textures, lighting, or a 3D camera.
- It does not own the game loop. `NSTimer` schedules callbacks; the simulation
  decides how many fixed ticks to consume.

---

## 2. The mental model: a print shop

Metal's object names sound intimidating. They map to a simple workflow:

| Metal object | Print-shop analogy | In this project |
| --- | --- | --- |
| `MTLDevice` | The printing press | Created once from the system GPU |
| `MTLCommandQueue` | The queue of print jobs | One queue, created at startup |
| `MTLBuffer` | A tray of forms | Three shared-storage vertex buffers |
| `MTLRenderPipelineState` | The configured machine: inks, plates, finishing | Shaders, blending, and 4× MSAA, built once |
| Drawable | The sheet of paper being printed | The texture supplied by `CAMetalLayer` |
| `MTLCommandBuffer` | One job ticket | All GPU work for one frame |
| `MTLRenderCommandEncoder` | The operator writing instructions on the ticket | Records pipeline, buffer, uniforms, draw |
| `CAMetalLayer` | The shop front that keeps producing paper | Requests drawables from AppKit's surface |

**Uniforms** are a small packet of values constant for a whole draw call. Here
they hold the letterbox scale and offset that map the 224 × 256 arcade field onto
the actual drawable.

### Web-developer translation table

| Web idea | This project |
| --- | --- |
| `<canvas>` backing store | The drawable texture supplied by `CAMetalLayer` |
| `requestAnimationFrame` | `NSTimer` calling the view's `tick:` method at 60 Hz |
| Canvas 2D paths and fills | `Canvas`, which expands everything into triangles |
| WebGL / WebGPU context | `MTLDevice` plus `MTLCommandQueue` |
| A compiled shader program | `MTLRenderPipelineState` |
| `bufferData` with a typed array | Copying `Vertex` values into an `MTLBuffer` |
| One `drawArrays` call | One `drawPrimitives` call |
| CSS `object-fit: contain` | The scale/offset uniform |
| `devicePixelRatio` | `drawableSize` versus the window's point size |
| `gl.clear` | The render pass's clear colour |
| VSync / present | `presentDrawable` followed by `commit` |

The big difference from Canvas is that the browser's 2D API accepts high-level
shapes. Metal accepts only **vertices** and **draw calls**. Every circle, line,
letter, and scanline in this game has already been converted into triangles
before Metal sees it.

---

## 3. One frame at 30,000 feet

```text
NSTimer fires at ~60 Hz
  └─ GameView::frame_tick
       ├─ accumulate elapsed time, run 0–8 fixed 1/60 s simulation ticks
       ├─ drain gameplay events and trigger audio cues
       ├─ scene::build(game) → Canvas of triangles
       └─ Renderer::draw(canvas, drawable_size)
            ├─ obtain the next drawable
            ├─ ensure the matching multisample texture
            ├─ copy vertices into the next ring buffer
            ├─ build the letterbox uniforms
            ├─ create command buffer and render pass
            ├─ set pipeline, vertex buffer, uniforms
            ├─ ONE drawPrimitives call for the whole frame
            └─ presentDrawable + addCompletedHandler + commit
  └─ GPU asynchronously: vertex shader → rasterise → fragment shader → blend → screen
```

The central split is deliberate: the view advances the game, then the renderer
reads the finished snapshot and draws it. The renderer never writes gameplay
state. That rule is why the simulation can be tested headlessly.

The frame entry point is
[`view.rs`](../crates/space-invaders-app/src/view.rs) (`frame_tick`). The GPU work is
in [`renderer.rs`](../crates/space-invaders-app/src/renderer.rs) (`Renderer::draw`).

---

## 4. Everything is a triangle

Look at [`canvas.rs`](../crates/space-invaders-app/src/canvas.rs) (`Vertex`). A
vertex is four floats of colour and two floats of position:

```text
x, y, red, green, blue, alpha
```

That is 24 bytes. There are no texture coordinates, normals, indices, or
matrices. A triangle is three consecutive vertices. A rectangle is two triangles;
a sprite is a set of merged horizontal pixel runs; text is a collection of small
rectangles. The font, the invaders, the shields, and the scanlines all use the
same format.

The layout is a contract between two files:

- CPU side: [`canvas.rs`](../crates/space-invaders-app/src/canvas.rs) (`Vertex`) and
  the [`Canvas::sprite`](../crates/space-invaders-app/src/canvas.rs) emitter.
- GPU side: [`metal_shaders.rs`](../crates/space-invaders-app/src/metal_shaders.rs)
  (`SHADERS`), where `VertexIn` declares the same two attributes.

If you change one, change the other. The renderer's vertex descriptor in
[`renderer.rs`](../crates/space-invaders-app/src/renderer.rs) (`Renderer::new`)
connects the two with `MTLVertexFormat` offsets and a stride of 24 bytes.

### Why there is only one draw call

All triangles share the same vertex format, shaders, and pipeline state. Nothing
switches textures or materials mid-frame, so the renderer can put the entire
frame in one buffer and issue a single draw. The call is in
[`renderer.rs`](../crates/space-invaders-app/src/renderer.rs) (`Renderer::draw`).

Drawing order still matters. The scene is composed back-to-front in
[`scene.rs`](../crates/space-invaders-app/src/scene.rs) (`build`):

1. starfield and horizon glow;
2. bunkers;
3. invaders;
4. mystery ship;
5. cannon, bullets, explosions, score popups;
6. HUD and lives;
7. READY, pause, wave-clear, and game-over overlays;
8. scanlines and vignette bands.

There is no depth buffer. Later triangles paint over earlier ones — the painter's
algorithm. The scanlines are last because they darken everything beneath them.

### A concrete example: one invader

A squid is 8 × 8 pixels and its solid runs merge into roughly 25 quads. Each quad
is two triangles, so one squid is about 150 vertices, or 3,600 bytes. All 55
invaders, four bunkers, text, and effects fit comfortably inside the 100,000-vertex
ring buffers.

This immediate-mode model would be a poor trade for a large 3D world with tens of
thousands of objects. For a fixed 2D arcade screen it keeps the code small and
makes the scene trivial to reason about.

---

## 5. A frame in slow motion

### 5.1 Set-up happens once

At startup, [`view.rs`](../crates/space-invaders-app/src/view.rs) (`GameView::new`)
creates the Metal device, configures a `CAMetalLayer`, and installs it as the
view's backing layer:

- `MTLCreateSystemDefaultDevice()` gets the GPU.
- `setPixelFormat(MTLPixelFormat::BGRA8Unorm)` chooses the drawable format.
- `setFramebufferOnly(true)` says the drawable is only a render target.
- `setMaximumDrawableCount(3)` allows a small ring of drawables.
- `setContentsScale(2.0)` is refreshed every frame from the window's backing
  scale, so Retina displays render at native resolution.

[`renderer.rs`](../crates/space-invaders-app/src/renderer.rs) (`Renderer::new`) then
builds the long-lived objects:

1. a command queue;
2. a Metal library compiled from the MSL source at launch;
3. the vertex and fragment functions;
4. a vertex descriptor matching `Vertex`;
5. a render pipeline with alpha blending and a sample count of four;
6. three shared-storage vertex buffers.

Compiling MSL at launch keeps the Cargo build free of an Xcode Metal toolchain
step. The cost is that shader errors surface at runtime, which is why startup
returns a readable error instead of panicking.

### 5.2 Each frame chooses a drawable and a buffer

`CAMetalLayer` owns a small pool of textures. `nextDrawable()` blocks briefly
until one is free. The renderer writes vertices into the next of three buffers.
Before reusing a slot it waits until at most two command buffers are in flight,
using a channel fed by `addCompletedHandler`.

That ring is in
[`renderer.rs`](../crates/space-invaders-app/src/renderer.rs)
(`Renderer::wait_for_capacity`). It prevents the CPU from overwriting a buffer the
GPU is still reading. The drawable is kept alive in the renderer's ring as well,
so presentation cannot outlive the texture it references.

### 5.3 The render pass and MSAA

A `CAMetalLayer` drawable is usually not multisampled. The renderer creates a
separate 4× multisample texture that matches the drawable size. It attaches that
texture to colour attachment zero, sets the drawable as the resolve target, and
uses `MTLStoreAction::MultisampleResolve`. The GPU rasterises four samples per
pixel, then resolves into the drawable. The texture is recreated when the window
or display scale changes; the old one is retired rather than freed while a frame
might still reference it.

This is in
[`renderer.rs`](../crates/space-invaders-app/src/renderer.rs)
(`Renderer::ensure_multisample`).

### 5.4 Encoding the draw

The render encoder receives:

- the pipeline;
- the vertex buffer at index 0;
- the sixteen-byte uniform at index 1;
- the primitive type and vertex count.

Then it calls `drawPrimitives(MTLPrimitiveType::Triangle, 0, count)`. There are
no index buffers and no per-object draws.

### 5.5 Presenting

`presentDrawable` schedules presentation when the command buffer completes.
`commit` hands the whole ticket to the GPU. The CPU immediately returns to AppKit
and the next timer callback, while the GPU works on the frame in parallel.

---

## 6. The shaders

The complete MSL source is [`metal_shaders.rs`](../crates/space-invaders-app/src/metal_shaders.rs)
(`SHADERS`). It contains two functions.

### Vertex shader

```metal
struct VertexIn {
    float2 position [[attribute(0)]];
    float4 color    [[attribute(1)]];
};

struct Uniforms {
    float2 scale;
    float2 offset;
};

vertex VertexOut vertex_main(VertexIn in [[stage_in]],
                             constant Uniforms &uniforms [[buffer(1)]]) {
    out.position = float4(in.position * uniforms.scale + uniforms.offset, 0.0, 1.0);
    out.color = in.color;
}
```

`[[attribute(n)]]` indexes into the vertex descriptor. `[[buffer(1)]]` is the
uniform slot bound with `setVertexBytes`. Positions arrive in arcade pixels and
leave in Metal's clip space, where x and y range from −1 to 1. The scale is
negative on y because the game uses a top-left origin while clip space has y
pointing up.

### Fragment shader

```metal
fragment float4 fragment_main(VertexOut in [[stage_in]]) {
    return in.color;
}
```

The fragment shader does no lighting or texturing. It returns the interpolated
colour, and the fixed-function blend unit mixes it with the destination using
source-alpha blending.

### Letterboxing

The CPU computes the uniform in
[`renderer.rs`](../crates/space-invaders-app/src/renderer.rs)
(`letterbox_uniforms`). It fits 224 × 256 inside the drawable with the same scale
on both axes and centres the result. Wider windows get black bars on the left and
right; taller windows get bars on the top and bottom. The game world never
changes.

A useful exercise is to set `scale` to a constant `2/224` and `offset` to `-1`
and watch the game pin to the left edge. That is exactly what the uniform is
doing, with a little arithmetic to keep it centred.

---

## 7. Colour, blending, and the CRT look

The layer and pipeline use `BGRA8Unorm`, not an sRGB format. The palette is
authored directly in display values and no gamma conversion is applied, which
keeps the neon colours predictable.

Alpha blending is enabled on the colour attachment with:

- source RGB × source alpha;
- destination RGB × (1 − source alpha);
- alpha = source + destination × (1 − source alpha).

The translucent parts of the game — scanlines, pause dimming, explosion fades,
starfield twinkle — rely on that blend. There are no textures or sprite sheets;
the CRT look is a stack of one-pixel translucent rectangles drawn after the
scene.

The black bars outside the playfield come from the render pass clear colour. The
scene then draws a slightly blue-black starfield over the game area.

---

## 8. Why not use a texture atlas?

An atlas would be the right choice for hundreds of animated sprites or detailed
art. Here:

- every sprite is 5–16 pixels wide;
- the palette is a handful of colours;
- the entire scene is rebuilt in a few hundred microseconds;
- there are no external assets to load.

Immediate-mode triangles avoid texture coordinates, samplers, atlas packing,
bleeding at atlas edges, and a binary asset pipeline. The code is longer in
`Canvas` but shorter everywhere else. That trade is documented in
[`RUST_DESIGN.md`](RUST_DESIGN.md) as well.

---

## 9. Common failure modes

- **Shader compiles but nothing is visible.** Check the clip-space transform,
  the vertex count, and whether the pipeline's pixel format matches the layer.
  A build alone cannot catch this.
- **A black window with a valid pipeline.** Check the drawable size. A zero-sized
  layer (before the window is shown) produces no drawable.
- **Torn or corrupt frames after resize.** The MSAA texture or drawable size is
  stale. This renderer recreates the texture and retires the old one.
- **A hang after a few seconds.** The in-flight ring is not being signalled.
  Completion handlers must run; do not block the main thread on GPU work.
- **Ghosting or flicker.** Presentation happened before the command buffer
  completed, or a drawable was released too early.
- **A too-dark or washed-out frame.** The pixel format and the authored palette
  are in different colour spaces.

For visual changes, run the app. Metal errors are runtime errors; `cargo build`
cannot prove that a shader compiled or that a frame reached the screen.

---

## 10. Where to go next

- Read [`RUST_DESIGN.md`](RUST_DESIGN.md) for the Rust architecture and the
  boundary between simulation and presentation.
- Read the [Q&A entry on the renderer](QnA/metal-renderer.md) for a shorter,
  source-grounded walkthrough.
- Change one thing at a time: a colour, then a sprite, then a uniform. Keep
  `make check` green and relaunch after every visual edit.
