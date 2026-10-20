# How does one draw call render the whole game?

- **Recorded:** 2026-10-20
- **Last updated:** 2026-10-20

## Short answer

Every visible thing is a triangle with a position in arcade pixels and an RGBA
colour. `scene::build` appends all of them to one vertex array. `Renderer::draw`
uploads that array once, sets a pipeline and a 16-byte letterbox uniform, and
issues one `drawPrimitives` call. The GPU rasterises the triangles into a 4×
multisampled texture, resolves it into the layer's drawable, and presents it.

## From game state to triangles

[`scene.rs`](../../crates/space-invaders-app/src/scene.rs) (`build`) reads the game
snapshot and paints in back-to-front order:

1. starfield and horizon glow;
2. bunker cells;
3. invaders, mystery ship, cannon;
4. bullets, explosions, score popups;
5. HUD, lives, wave label;
6. READY, pause, wave-clear, and game-over overlays;
7. scanlines and vignette bands.

The scene is stateless. It creates a fresh
[`Canvas`](../../crates/space-invaders-app/src/canvas.rs) (`Canvas`) each frame and
never touches the game.

A rectangle becomes two triangles in
[`Canvas::rect`](../../crates/space-invaders-app/src/canvas.rs) (`Canvas::rect`). A
sprite is a set of
horizontal runs in
[`Canvas::sprite`](../../crates/space-invaders-app/src/canvas.rs) (`Canvas::sprite`):
adjacent opaque pixels in a row merge into one quad, so a typical invader costs a
few dozen vertices rather than six per pixel. Text uses the same rectangle path
through [`pixel_font.rs`](../../crates/space-invaders-app/src/pixel_font.rs)
(`draw_text`).

## The vertex format

[`Vertex`](../../crates/space-invaders-app/src/canvas.rs) is:

```rust
#[repr(C)]
pub struct Vertex {
    pub position: [f32; 2],
    pub color: [f32; 4],
}
```

Twenty-four bytes, `repr(C)` so the layout is stable. The renderer describes it
to Metal in [`Renderer::new`](../../crates/space-invaders-app/src/renderer.rs)
(`Renderer::new`) with two attributes and a stride of `size_of::<Vertex>()`. The shader declares the matching
`VertexIn` in [`metal_shaders.rs`](../../crates/space-invaders-app/src/metal_shaders.rs)
(`SHADERS`).

## Triple buffering

The renderer keeps three shared-storage vertex buffers. A frame uses
`frame_index % 3`. Before writing into a slot, it ensures at most two command
buffers are still in flight, using a channel signalled by
`addCompletedHandler`:

- [`Renderer::wait_for_capacity`](../../crates/space-invaders-app/src/renderer.rs)
  blocks until a completion arrives;
- [`Renderer::reap_completed`](../../crates/space-invaders-app/src/renderer.rs) drains
  any completions that arrived since the last frame.

The buffers and the two in-flight command buffers also retain their drawables in
a fixed ring, so a drawable cannot be released while the GPU is still presenting
it. The design is deliberately bounded: no unbounded queue, no thread pool, no
channel growth.

## The render pass

A `CAMetalLayer` drawable is single-sampled. For anti-aliasing, the renderer
allocates or reuses a 4× multisample texture with
[`Renderer::ensure_multisample`](../../crates/space-invaders-app/src/renderer.rs),
attaches it as colour attachment zero, and sets the drawable as the resolve
target. The store action is `MultisampleResolve`, so the GPU resolves the samples
into the drawable.

When the window or backing scale changes, the texture is recreated at the new
size. The previous texture is moved to a retired list rather than freed
immediately, because a command buffer from the previous frame might still be
reading it. That list is small and bounded by the number of resizes in a session.

## The uniform

The only uniform is the letterbox transform:

```rust
#[repr(C)]
struct Uniforms {
    scale: [f32; 2],
    offset: [f32; 2],
}
```

[`letterbox_uniforms`](../../crates/space-invaders-app/src/renderer.rs) fits the fixed
224 × 256 design into the drawable with one uniform scale, centres the result, and
flips y for clip space. No world coordinate changes, no per-object matrices, no
projection stack.

The vertex shader applies it:

```metal
out.position = float4(in.position * uniforms.scale + uniforms.offset, 0.0, 1.0);
```

## The draw

After creating the render pass and encoder,
[`Renderer::draw`](../../crates/space-invaders-app/src/renderer.rs) sets:

- the pipeline state;
- the vertex buffer at index 0;
- the uniform bytes at index 1.

Then one call:

```rust
encoder.drawPrimitives_vertexStart_vertexCount(
    MTLPrimitiveType::Triangle,
    0,
    canvas.vertices().len(),
);
```

There is no index buffer, no texture binding, and no second draw. Presentation is
scheduled with `presentDrawable`, a completion handler is installed, and the
command buffer is committed. The main thread is free before the GPU finishes.

## Why this is safe

Metal calls are `unsafe` in `objc2` because the compiler cannot prove buffer
sizes, offsets, or lifetimes. This renderer makes those invariants explicit:

- The canvas is capped at `MAX_VERTICES`, and the buffer is allocated for that
  capacity.
- The copied byte count is bounded by the same constant.
- A buffer is reused only after its command buffer completes.
- The MSAA texture and drawables outlive any command buffer that references them.
- Shader attribute offsets and strides match the Rust `repr(C)` struct.

Each unsafe block in [`renderer.rs`](../../crates/space-invaders-app/src/renderer.rs)
(`Renderer::draw`) carries a comment naming the invariant. If you change the vertex format, change
the descriptor and the shader together, then launch the app; a clean build cannot
validate a runtime-compiled shader.
