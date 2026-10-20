//! Metal renderer: one pipeline, one draw call, triple-buffered vertices.
//!
//! The CPU builds triangles in arcade-pixel space. A vertex shader applies a
//! letterbox transform supplied in a small uniform and the GPU rasterises the
//! whole frame in one `drawPrimitives` call. A 4x multisample texture is
//! resolved into the `CAMetalLayer` drawable.

use std::ffi::c_void;
use std::fmt;
use std::mem::size_of;
use std::ptr::NonNull;
use std::sync::mpsc::{self, Receiver, Sender};

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_foundation::{NSError, ns_string};
use objc2_metal::{
    MTLBuffer, MTLClearColor, MTLCommandBuffer, MTLCommandEncoder, MTLCommandQueue, MTLDevice,
    MTLDrawable, MTLLibrary, MTLLoadAction, MTLPixelFormat, MTLPrimitiveType,
    MTLRenderCommandEncoder, MTLRenderPassDescriptor, MTLRenderPipelineDescriptor,
    MTLRenderPipelineState, MTLResourceOptions, MTLStoreAction, MTLTexture, MTLTextureDescriptor,
    MTLTextureType, MTLTextureUsage, MTLVertexDescriptor, MTLVertexFormat, MTLVertexStepFunction,
};
use objc2_quartz_core::{CAMetalDrawable, CAMetalLayer};

use crate::canvas::{Canvas, MAX_VERTICES, Vertex};
use crate::metal_shaders::SHADERS;

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {}

const SAMPLE_COUNT: usize = 4;
const FRAMES_IN_FLIGHT: usize = 2;
const BUFFER_COUNT: usize = 3;
const PIXEL_FORMAT: MTLPixelFormat = MTLPixelFormat::BGRA8Unorm;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Uniforms {
    scale: [f32; 2],
    offset: [f32; 2],
}

/// A startup or draw failure suitable for showing to the user.
#[derive(Debug)]
pub struct RendererError(String);

impl RendererError {
    fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for RendererError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for RendererError {}

fn describe_error(error: &NSError) -> String {
    error.localizedDescription().to_string()
}

/// Owns the Metal objects needed for the lifetime of the window.
pub struct Renderer {
    device: Retained<ProtocolObject<dyn MTLDevice>>,
    layer: Retained<CAMetalLayer>,
    queue: Retained<ProtocolObject<dyn MTLCommandQueue>>,
    pipeline: Retained<ProtocolObject<dyn MTLRenderPipelineState>>,
    buffers: [Retained<ProtocolObject<dyn objc2_metal::MTLBuffer>>; BUFFER_COUNT],
    drawables: [Option<Retained<ProtocolObject<dyn CAMetalDrawable>>>; BUFFER_COUNT],
    msaa: Option<Retained<ProtocolObject<dyn MTLTexture>>>,
    msaa_size: (usize, usize),
    retired_textures: Vec<Retained<ProtocolObject<dyn MTLTexture>>>,
    frame: usize,
    in_flight: usize,
    completion_tx: Sender<()>,
    completion_rx: Receiver<()>,
}

impl Renderer {
    /// Creates the device, command queue, pipeline, and shared vertex buffers.
    pub fn new(
        device: Retained<ProtocolObject<dyn MTLDevice>>,
        layer: Retained<CAMetalLayer>,
    ) -> Result<Self, RendererError> {
        let queue = device
            .newCommandQueue()
            .ok_or_else(|| RendererError::new("Metal could not create a command queue"))?;

        let library = device
            .newLibraryWithSource_options_error(ns_string!(SHADERS), None)
            .map_err(|error| {
                RendererError::new(format!(
                    "Metal shader compilation failed: {}",
                    describe_error(&error)
                ))
            })?;

        let vertex_function = library
            .newFunctionWithName(ns_string!("vertex_main"))
            .ok_or_else(|| RendererError::new("Metal shader has no vertex_main function"))?;
        let fragment_function = library
            .newFunctionWithName(ns_string!("fragment_main"))
            .ok_or_else(|| RendererError::new("Metal shader has no fragment_main function"))?;

        let vertex_descriptor = MTLVertexDescriptor::vertexDescriptor();
        // SAFETY: the descriptor arrays accept indices 0 and 1, and the
        // attribute layout matches `Vertex` exactly.
        unsafe {
            let attributes = vertex_descriptor.attributes();
            let position = attributes.objectAtIndexedSubscript(0);
            position.setFormat(MTLVertexFormat::Float2);
            position.setOffset(0);
            position.setBufferIndex(0);
            let color = attributes.objectAtIndexedSubscript(1);
            color.setFormat(MTLVertexFormat::Float4);
            color.setOffset(size_of::<[f32; 2]>());
            color.setBufferIndex(0);

            let layout = vertex_descriptor.layouts().objectAtIndexedSubscript(0);
            layout.setStride(size_of::<Vertex>());
            layout.setStepFunction(MTLVertexStepFunction::PerVertex);
        }

        let pipeline_descriptor = MTLRenderPipelineDescriptor::new();
        pipeline_descriptor.setLabel(Some(ns_string!("Space Invaders sprite pipeline")));
        pipeline_descriptor.setVertexFunction(Some(&vertex_function));
        pipeline_descriptor.setFragmentFunction(Some(&fragment_function));
        pipeline_descriptor.setVertexDescriptor(Some(&vertex_descriptor));
        pipeline_descriptor.setRasterSampleCount(SAMPLE_COUNT);
        // SAFETY: colour attachment index 0 exists on a new pipeline descriptor.
        unsafe {
            let attachment = pipeline_descriptor
                .colorAttachments()
                .objectAtIndexedSubscript(0);
            attachment.setPixelFormat(PIXEL_FORMAT);
            attachment.setBlendingEnabled(true);
            attachment.setSourceRGBBlendFactor(objc2_metal::MTLBlendFactor::SourceAlpha);
            attachment
                .setDestinationRGBBlendFactor(objc2_metal::MTLBlendFactor::OneMinusSourceAlpha);
            attachment.setSourceAlphaBlendFactor(objc2_metal::MTLBlendFactor::One);
            attachment
                .setDestinationAlphaBlendFactor(objc2_metal::MTLBlendFactor::OneMinusSourceAlpha);
            attachment.setRgbBlendOperation(objc2_metal::MTLBlendOperation::Add);
            attachment.setAlphaBlendOperation(objc2_metal::MTLBlendOperation::Add);
        }

        let pipeline = device
            .newRenderPipelineStateWithDescriptor_error(&pipeline_descriptor)
            .map_err(|error| {
                RendererError::new(format!(
                    "Metal render pipeline failed: {}",
                    describe_error(&error)
                ))
            })?;

        let buffer_length = MAX_VERTICES * size_of::<Vertex>();
        let mut buffers = Vec::with_capacity(BUFFER_COUNT);
        for _ in 0..BUFFER_COUNT {
            buffers.push(
                device
                    .newBufferWithLength_options(
                        buffer_length,
                        MTLResourceOptions::StorageModeShared,
                    )
                    .ok_or_else(|| {
                        RendererError::new("Metal could not allocate a vertex buffer")
                    })?,
            );
        }
        let buffers: [Retained<ProtocolObject<dyn MTLBuffer>>; BUFFER_COUNT] =
            buffers
                .try_into()
                .map_err(|_| RendererError::new("Metal vertex buffer count mismatch"))?;

        let (completion_tx, completion_rx) = mpsc::channel();

        Ok(Self {
            device,
            layer,
            queue,
            pipeline,
            buffers,
            drawables: [None, None, None],
            msaa: None,
            msaa_size: (0, 0),
            retired_textures: Vec::new(),
            frame: 0,
            in_flight: 0,
            completion_tx,
            completion_rx,
        })
    }

    /// Renders one frame into the layer's next drawable.
    ///
    /// `drawable_width` and `drawable_height` are physical pixels, already
    /// multiplied by the display's backing scale.
    pub fn draw(
        &mut self,
        canvas: &Canvas,
        drawable_width: usize,
        drawable_height: usize,
    ) -> Result<(), RendererError> {
        if drawable_width == 0 || drawable_height == 0 {
            return Ok(());
        }
        self.reap_completed();

        let Some(drawable) = self.layer.nextDrawable() else {
            return Ok(());
        };
        let drawable_texture = drawable.texture();
        let multisample = self.ensure_multisample(drawable_width, drawable_height)?;

        self.wait_for_capacity();

        let buffer_index = self.frame % BUFFER_COUNT;
        let buffer = &self.buffers[buffer_index];
        let vertex_bytes = std::mem::size_of_val(canvas.vertices());
        let capacity = MAX_VERTICES * size_of::<Vertex>();
        if vertex_bytes > capacity {
            return Err(RendererError::new("vertex buffer overflow"));
        }
        if vertex_bytes > 0 {
            let destination = buffer.contents().as_ptr().cast::<u8>();
            // SAFETY: `destination` points at `capacity` writable bytes in a
            // shared-storage buffer, and `vertex_bytes <= capacity`. The buffer
            // is not in use by a previous frame because the in-flight ring
            // guarantees at most two outstanding command buffers.
            unsafe {
                std::ptr::copy_nonoverlapping(
                    canvas.vertices().as_ptr().cast::<u8>(),
                    destination,
                    vertex_bytes,
                );
            }
        }

        let uniforms = letterbox_uniforms(drawable_width, drawable_height);
        let command_buffer = self
            .queue
            .commandBuffer()
            .ok_or_else(|| RendererError::new("Metal could not create a command buffer"))?;

        let pass = MTLRenderPassDescriptor::renderPassDescriptor();
        {
            let attachments = pass.colorAttachments();
            // SAFETY: index 0 is always a valid colour attachment on a default
            // render-pass descriptor.
            let attachment = unsafe { attachments.objectAtIndexedSubscript(0) };
            attachment.setTexture(Some(&multisample));
            attachment.setResolveTexture(Some(&drawable_texture));
            attachment.setLoadAction(MTLLoadAction::Clear);
            attachment.setStoreAction(MTLStoreAction::MultisampleResolve);
            attachment.setClearColor(MTLClearColor {
                red: 0.012,
                green: 0.016,
                blue: 0.027,
                alpha: 1.0,
            });
        }

        let encoder = command_buffer
            .renderCommandEncoderWithDescriptor(&pass)
            .ok_or_else(|| RendererError::new("Metal could not create a render encoder"))?;
        encoder.setRenderPipelineState(&self.pipeline);
        // SAFETY: the vertex buffer and uniform are valid for the duration of
        // encoding, and the encoder owns no reference beyond the command
        // buffer that retains it.
        unsafe {
            encoder.setVertexBuffer_offset_atIndex(Some(&self.buffers[buffer_index]), 0, 0);
            let uniform_ptr = NonNull::from(&uniforms).cast::<c_void>();
            encoder.setVertexBytes_length_atIndex(uniform_ptr, size_of::<Uniforms>(), 1);
            encoder.drawPrimitives_vertexStart_vertexCount(
                MTLPrimitiveType::Triangle,
                0,
                canvas.vertices().len(),
            );
        }
        encoder.endEncoding();

        command_buffer.presentDrawable(ProtocolObject::<dyn MTLDrawable>::from_ref(&*drawable));

        let completion_sender = self.completion_tx.clone();
        let completion = RcBlock::new(
            move |_buffer: NonNull<ProtocolObject<dyn MTLCommandBuffer>>| {
                let _ = completion_sender.send(());
            },
        );
        // SAFETY: the block is a valid command-buffer handler and Metal copies
        // it when it is added. The sender is owned by the closure.
        unsafe {
            command_buffer.addCompletedHandler(RcBlock::as_ptr(&completion));
        }

        command_buffer.commit();
        self.drawables[buffer_index] = Some(drawable);
        self.frame = self.frame.wrapping_add(1);
        self.in_flight += 1;
        Ok(())
    }

    fn ensure_multisample(
        &mut self,
        width: usize,
        height: usize,
    ) -> Result<Retained<ProtocolObject<dyn MTLTexture>>, RendererError> {
        if self.msaa_size == (width, height) {
            if let Some(texture) = &self.msaa {
                return Ok(texture.clone());
            }
        }

        // SAFETY: the dimensions come from the layer's drawable size and are
        // non-zero; Metal validates them when the texture is created.
        let descriptor = unsafe {
            MTLTextureDescriptor::texture2DDescriptorWithPixelFormat_width_height_mipmapped(
                PIXEL_FORMAT,
                width,
                height,
                false,
            )
        };
        descriptor.setTextureType(MTLTextureType::Type2DMultisample);
        // SAFETY: sample count 4 is the value used by the pipeline and is
        // supported by every Metal family targeted by macOS 13+.
        unsafe { descriptor.setSampleCount(SAMPLE_COUNT) };
        descriptor.setUsage(MTLTextureUsage::RenderTarget);
        descriptor.setStorageMode(objc2_metal::MTLStorageMode::Private);

        let texture = self
            .device
            .newTextureWithDescriptor(&descriptor)
            .ok_or_else(|| RendererError::new("Metal could not allocate a multisample texture"))?;

        if let Some(previous) = self.msaa.replace(texture) {
            // A command buffer may still be reading the old texture while a
            // resize arrives. Keep it alive instead of risking a use-after-free;
            // resizes are rare and the memory is reclaimed when the app exits.
            self.retired_textures.push(previous);
        }
        self.msaa_size = (width, height);
        Ok(self.msaa.as_ref().expect("just installed").clone())
    }

    fn reap_completed(&mut self) {
        while let Ok(()) = self.completion_rx.try_recv() {
            self.in_flight = self.in_flight.saturating_sub(1);
        }
    }

    fn wait_for_capacity(&mut self) {
        while self.in_flight >= FRAMES_IN_FLIGHT {
            if self.completion_rx.recv().is_err() {
                self.in_flight = 0;
                break;
            }
            self.in_flight = self.in_flight.saturating_sub(1);
        }
    }
}

fn letterbox_uniforms(drawable_width: usize, drawable_height: usize) -> Uniforms {
    let width = drawable_width as f32;
    let height = drawable_height as f32;
    let design_width = 224.0;
    let design_height = 256.0;
    let content_scale = (width / design_width).min(height / design_height);
    let content_width = design_width * content_scale;
    let content_height = design_height * content_scale;
    let offset_x = (width - content_width) / 2.0;
    let offset_y = (height - content_height) / 2.0;

    Uniforms {
        scale: [
            2.0 * content_width / width / design_width,
            -2.0 * content_height / height / design_height,
        ],
        offset: [2.0 * offset_x / width - 1.0, 1.0 - 2.0 * offset_y / height],
    }
}

#[cfg(test)]
mod tests {
    use super::letterbox_uniforms;

    #[test]
    fn letterbox_fills_a_matching_aspect_ratio() {
        let uniforms = letterbox_uniforms(448, 512);
        assert!((uniforms.scale[0] - 2.0 / 224.0).abs() < 1e-6);
        assert!((uniforms.scale[1] + 2.0 / 256.0).abs() < 1e-6);
        assert!((uniforms.offset[0] + 1.0).abs() < 1e-5);
        assert!((uniforms.offset[1] - 1.0).abs() < 1e-5);
    }

    #[test]
    fn letterbox_centres_a_wider_window() {
        let uniforms = letterbox_uniforms(640, 512);
        assert!((uniforms.offset[1] - 1.0).abs() < 1e-6);
        assert!(uniforms.offset[0] < 0.0);
    }
}
