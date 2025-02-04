#![deny(clippy::all)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::single_match)]

use rgx::core;
use rgx::core::*;
use rgx::kit;
use rgx::kit::sprite2d;
use rgx::kit::*;
use rgx::math::*;

use image::ImageDecoder;

use raw_window_handle::HasRawWindowHandle;
use winit::{
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

pub struct Framebuffer {
    target: core::Framebuffer,
    vertices: core::VertexBuffer,
}

impl Framebuffer {
    fn new(w: u32, h: u32, r: &core::Renderer) -> Self {
        #[rustfmt::skip]
        let vertices: &[(f32, f32, f32, f32)] = &[
            (-1.0, -1.0, 0.0, 1.0),
            ( 1.0, -1.0, 1.0, 1.0),
            ( 1.0,  1.0, 1.0, 0.0),
            (-1.0, -1.0, 0.0, 1.0),
            (-1.0,  1.0, 0.0, 0.0),
            ( 1.0,  1.0, 1.0, 0.0),
        ];

        Self {
            target: r.framebuffer(w, h),
            vertices: r.vertex_buffer(vertices),
        }
    }
}

pub struct FramebufferPipeline {
    pipeline: core::Pipeline,
    bindings: core::BindingGroup,
    buf: core::UniformBuffer,
    width: u32,
    height: u32,
}

impl<'a> core::AbstractPipeline<'a> for FramebufferPipeline {
    type PrepareContext = core::Rgba;
    type Uniforms = core::Rgba;

    fn description() -> core::PipelineDescription<'a> {
        core::PipelineDescription {
            vertex_layout: &[core::VertexFormat::Float2, core::VertexFormat::Float2],
            pipeline_layout: &[
                Set(&[Binding {
                    binding: BindingType::UniformBuffer,
                    stage: ShaderStage::Vertex,
                }]),
                Set(&[
                    Binding {
                        binding: BindingType::SampledTexture,
                        stage: ShaderStage::Fragment,
                    },
                    Binding {
                        binding: BindingType::Sampler,
                        stage: ShaderStage::Fragment,
                    },
                ]),
            ],
            // TODO: Use `env("CARGO_MANIFEST_DIR")`
            vertex_shader: include_bytes!("data/framebuffer.vert.spv"),
            fragment_shader: include_bytes!("data/framebuffer.frag.spv"),
        }
    }

    fn setup(pipeline: core::Pipeline, dev: &core::Device, width: u32, height: u32) -> Self {
        let buf = dev.create_uniform_buffer(&[core::Rgba::TRANSPARENT]);
        let bindings = dev.create_binding_group(&pipeline.layout.sets[0], &[&buf]);

        FramebufferPipeline {
            pipeline,
            buf,
            bindings,
            width,
            height,
        }
    }

    fn apply(&self, pass: &mut core::Pass) {
        pass.set_pipeline(&self.pipeline);
        pass.set_binding(&self.bindings, &[]);
    }

    fn prepare(&'a self, color: core::Rgba) -> Option<(&'a core::UniformBuffer, Vec<core::Rgba>)> {
        Some((&self.buf, vec![color]))
    }

    fn resize(&mut self, w: u32, h: u32) {
        self.width = w;
        self.height = h;
    }

    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }
}

impl FramebufferPipeline {
    pub fn binding(
        &self,
        renderer: &core::Renderer,
        framebuffer: &Framebuffer,
        sampler: &core::Sampler,
    ) -> core::BindingGroup {
        renderer.device.create_binding_group(
            &self.pipeline.layout.sets[1],
            &[&framebuffer.target, sampler],
        )
    }
}

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new();
    let window = Window::new(&event_loop).unwrap();

    ///////////////////////////////////////////////////////////////////////////
    // Setup renderer
    ///////////////////////////////////////////////////////////////////////////

    let mut r = Renderer::new(window.raw_window_handle());
    let size = window.inner_size().to_physical(window.hidpi_factor());

    let (sw, sh) = (size.width as u32, size.height as u32);
    let mut offscreen: kit::sprite2d::Pipeline = r.pipeline(sw, sh, Blending::default());
    let mut onscreen: FramebufferPipeline = r.pipeline(sw, sh, Blending::default());
    let framebuffer = Framebuffer::new(sw, sh, &r);

    ///////////////////////////////////////////////////////////////////////////
    // Setup sampler & load texture
    ///////////////////////////////////////////////////////////////////////////

    let sampler = r.sampler(Filter::Nearest, Filter::Nearest);

    let (texture, pixels) = {
        let bytes = include_bytes!("data/sprite.tga");
        let tga = std::io::Cursor::new(bytes.as_ref());
        let decoder = image::tga::TGADecoder::new(tga).unwrap();
        let (w, h) = decoder.dimensions();
        let pixels = decoder.read_image().unwrap();
        let pixels = Rgba8::align(&pixels);

        (r.texture(w as u32, h as u32), pixels.to_owned())
    };

    let offscreen_binding = offscreen.binding(&r, &texture, &sampler); // Texture binding
    let onscreen_binding = onscreen.binding(&r, &framebuffer, &sampler);

    let w = 50.0;
    let rect = Rect::new(w * 1.0, 0.0, w * 2.0, texture.h as f32);
    let batch = sprite2d::Batch::singleton(
        texture.w,
        texture.h,
        rect,
        Rect::origin(sw as f32, sh as f32),
        Rgba::TRANSPARENT,
        1.0,
        Repeat::default(),
    );
    let buffer = batch.finish(&r);

    ///////////////////////////////////////////////////////////////////////////
    // Prepare resources
    ///////////////////////////////////////////////////////////////////////////

    r.prepare(&[Op::Fill(&texture, pixels.as_slice())]);

    ///////////////////////////////////////////////////////////////////////////
    // Render loop
    ///////////////////////////////////////////////////////////////////////////

    let mut textures = r.swap_chain(sw, sh, PresentMode::default());

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent { event, .. } => match event {
            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        virtual_keycode: Some(VirtualKeyCode::Escape),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            WindowEvent::CloseRequested => {
                *control_flow = ControlFlow::Exit;
            }
            WindowEvent::Resized(size) => {
                let physical = size.to_physical(window.hidpi_factor());
                let (w, h) = (physical.width as u32, physical.height as u32);

                offscreen.resize(w, h);
                onscreen.resize(w, h);
                textures = r.swap_chain(w, h, PresentMode::default());
            }
            _ => {}
        },
        Event::EventsCleared => {
            *control_flow = ControlFlow::Wait;

            ///////////////////////////////////////////////////////////////////////////
            // Create frame
            ///////////////////////////////////////////////////////////////////////////

            let mut frame = r.frame();

            ///////////////////////////////////////////////////////////////////////////
            // Prepare pipeline
            ///////////////////////////////////////////////////////////////////////////

            r.update_pipeline(&offscreen, Matrix4::identity(), &mut frame);
            r.update_pipeline(&onscreen, Rgba::new(0.2, 0.2, 0.0, 1.0), &mut frame);

            ///////////////////////////////////////////////////////////////////////////
            // Draw frame
            ///////////////////////////////////////////////////////////////////////////

            let out = textures.next();

            {
                let pass = &mut frame.pass(PassOp::Clear(Rgba::TRANSPARENT), &framebuffer.target);
                pass.set_pipeline(&offscreen);
                pass.draw(&buffer, &offscreen_binding);
            }

            {
                let pass = &mut frame.pass(PassOp::Clear(Rgba::TRANSPARENT), &out);
                pass.set_pipeline(&onscreen);
                pass.draw(&framebuffer.vertices, &onscreen_binding);
            }

            r.submit(frame);
        }
        _ => {}
    });
}
