//! The Metal Shading Language source compiled at launch.
//!
//! Compiling MSL at runtime keeps the Cargo build free of an Xcode Metal
//! toolchain step. The trade-off is that shader errors surface at launch, which
//! is why the app reports pipeline failures instead of panicking.

/// Vertex and fragment shaders for flat-coloured, alpha-blended triangles.
pub const SHADERS: &str = r"
#include <metal_stdlib>
using namespace metal;

struct VertexIn {
    float2 position [[attribute(0)]];
    float4 color    [[attribute(1)]];
};

struct VertexOut {
    float4 position [[position]];
    float4 color;
};

struct Uniforms {
    float2 scale;
    float2 offset;
};

vertex VertexOut vertex_main(VertexIn in [[stage_in]],
                             constant Uniforms &uniforms [[buffer(1)]]) {
    VertexOut out;
    // Convert arcade pixels into Metal's clip space, preserving the 224:256
    // letterboxed presentation computed on the CPU.
    out.position = float4(in.position * uniforms.scale + uniforms.offset, 0.0, 1.0);
    out.color = in.color;
    return out;
}

fragment float4 fragment_main(VertexOut in [[stage_in]]) {
    return in.color;
}
";
