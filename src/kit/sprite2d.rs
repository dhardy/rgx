#![deny(clippy::all, clippy::use_self)]
#![allow(clippy::new_without_default)]

use crate::core;
use crate::core::{Binding, BindingType, Rect, Rgba, Set, ShaderStage};

use crate::math::*;

use crate::kit;
use crate::kit::{Model, Repeat, Rgba8};

use crate::nonempty::NonEmpty;

///////////////////////////////////////////////////////////////////////////
// Uniforms
///////////////////////////////////////////////////////////////////////////

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Uniforms {
    pub ortho: Matrix4<f32>,
    pub transform: Matrix4<f32>,
}

///////////////////////////////////////////////////////////////////////////
// Vertex
///////////////////////////////////////////////////////////////////////////

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Vertex {
    position: Vector2<f32>,
    uv: Vector2<f32>,
    color: Rgba8,
    opacity: f32,
}

impl Vertex {
    fn new(x: f32, y: f32, u: f32, v: f32, color: Rgba8, opacity: f32) -> Self {
        Self {
            position: Vector2::new(x, y),
            uv: Vector2::new(u, v),
            color,
            opacity,
        }
    }
}

///////////////////////////////////////////////////////////////////////////
// Pipeline
///////////////////////////////////////////////////////////////////////////

pub struct Pipeline {
    pipeline: core::Pipeline,
    bindings: core::BindingGroup,
    buf: core::UniformBuffer,
    width: u32,
    height: u32,
    ortho: Matrix4<f32>,
    model: Model,
}

impl Pipeline {
    pub fn binding(
        &self,
        renderer: &core::Renderer,
        texture: &core::Texture,
        sampler: &core::Sampler,
    ) -> core::BindingGroup {
        renderer
            .device
            .create_binding_group(&self.pipeline.layout.sets[2], &[texture, sampler])
    }
}

//////////////////////////////////////////////////////////////////////////

pub struct Command<'a>(&'a core::VertexBuffer, &'a core::BindingGroup, Matrix4<f32>);

pub struct Frame<'a> {
    commands: Vec<Command<'a>>,
    transforms: NonEmpty<Matrix4<f32>>,
}

impl<'a> Frame<'a> {
    pub fn draw(&mut self, buffer: &'a core::VertexBuffer, binding: &'a core::BindingGroup) {
        self.commands
            .push(Command(buffer, binding, *self.transforms.last()));
    }

    pub fn transform<F>(&mut self, t: Matrix4<f32>, inner: F)
    where
        F: FnOnce(&mut Self),
    {
        self.transforms.push(*self.transforms.last() * t);
        inner(self);
        self.transforms.pop();
    }

    pub fn translate<F>(&mut self, x: f32, y: f32, inner: F)
    where
        F: FnOnce(&mut Self),
    {
        self.transform(Matrix4::from_translation(Vector3::new(x, y, 0.)), inner);
    }

    pub fn scale<F>(&mut self, s: f32, inner: F)
    where
        F: FnOnce(&mut Self),
    {
        self.transform(Matrix4::from_scale(s), inner);
    }
}

//////////////////////////////////////////////////////////////////////////

impl<'a> core::AbstractPipeline<'a> for Pipeline {
    type PrepareContext = Matrix4<f32>;
    type Uniforms = self::Uniforms;

    fn description() -> core::PipelineDescription<'a> {
        core::PipelineDescription {
            vertex_layout: &[
                core::VertexFormat::Float2,
                core::VertexFormat::Float2,
                core::VertexFormat::UByte4,
                core::VertexFormat::Float,
            ],
            pipeline_layout: &[
                Set(&[Binding {
                    binding: BindingType::UniformBuffer,
                    stage: ShaderStage::Vertex,
                }]),
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
            vertex_shader: include_bytes!("data/sprite.vert.spv"),
            fragment_shader: include_bytes!("data/sprite.frag.spv"),
        }
    }

    fn setup(pipeline: core::Pipeline, dev: &core::Device, width: u32, height: u32) -> Self {
        let ortho = kit::ortho(width, height);
        let transform = Matrix4::identity();
        let model = Model::new(&pipeline.layout.sets[1], &[Matrix4::identity()], dev);
        let buf = dev.create_uniform_buffer(&[self::Uniforms { ortho, transform }]);
        let bindings = dev.create_binding_group(&pipeline.layout.sets[0], &[&buf]);

        Self {
            pipeline,
            buf,
            bindings,
            model,
            ortho,
            width,
            height,
        }
    }

    fn resize(&mut self, w: u32, h: u32) {
        self.width = w;
        self.height = h;
        self.ortho = kit::ortho(w, h);
    }

    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn apply(&self, pass: &mut core::Pass) {
        pass.set_pipeline(&self.pipeline);
        pass.set_binding(&self.bindings, &[]);
        pass.set_binding(&self.model.binding, &[]);
    }

    fn prepare(
        &'a self,
        transform: Matrix4<f32>,
    ) -> Option<(&'a core::UniformBuffer, Vec<self::Uniforms>)> {
        Some((
            &self.buf,
            vec![self::Uniforms {
                transform,
                ortho: self.ortho,
            }],
        ))
    }
}

///////////////////////////////////////////////////////////////////////////////////////////////////
/// Batch
///////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Debug)]
pub struct Batch {
    pub w: u32,
    pub h: u32,
    pub size: usize,

    items: Vec<(Rect<f32>, Rect<f32>, Rgba, f32, Repeat)>,
}

impl Batch {
    pub fn new(w: u32, h: u32) -> Self {
        Self {
            w,
            h,
            items: Vec::new(),
            size: 0,
        }
    }

    pub fn singleton(
        w: u32,
        h: u32,
        src: Rect<f32>,
        dst: Rect<f32>,
        rgba: Rgba,
        opa: f32,
        rep: Repeat,
    ) -> Self {
        let mut view = Self::new(w, h);
        view.add(src, dst, rgba, opa, rep);
        view
    }

    pub fn add(&mut self, src: Rect<f32>, dst: Rect<f32>, rgba: Rgba, opacity: f32, rep: Repeat) {
        if rep != Repeat::default() {
            assert!(
                src == Rect::origin(self.w as f32, self.h as f32),
                "using texture repeat is only valid when using the entire {}x{} texture",
                self.w,
                self.h
            );
        }
        self.items.push((src, dst, rgba, opacity, rep));
        self.size += 1;
    }

    pub fn vertices(&self) -> Vec<Vertex> {
        let mut buf = Vec::with_capacity(6 * self.items.len());

        for (src, dst, rgba, o, rep) in self.items.iter() {
            // Relative texture coordinates
            let rx1: f32 = src.x1 / self.w as f32;
            let ry1: f32 = src.y1 / self.h as f32;
            let rx2: f32 = src.x2 / self.w as f32;
            let ry2: f32 = src.y2 / self.h as f32;

            let c: Rgba8 = (*rgba).into();

            // TODO: Use an index buffer
            buf.extend_from_slice(&[
                Vertex::new(dst.x1, dst.y1, rx1 * rep.x, ry2 * rep.y, c, *o),
                Vertex::new(dst.x2, dst.y1, rx2 * rep.x, ry2 * rep.y, c, *o),
                Vertex::new(dst.x2, dst.y2, rx2 * rep.x, ry1 * rep.y, c, *o),
                Vertex::new(dst.x1, dst.y1, rx1 * rep.x, ry2 * rep.y, c, *o),
                Vertex::new(dst.x1, dst.y2, rx1 * rep.x, ry1 * rep.y, c, *o),
                Vertex::new(dst.x2, dst.y2, rx2 * rep.x, ry1 * rep.y, c, *o),
            ]);
        }
        buf
    }

    pub fn finish(self, r: &core::Renderer) -> core::VertexBuffer {
        let buf = self.vertices();
        r.device.create_buffer(buf.as_slice())
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.size = 0;
    }

    pub fn offset(&mut self, x: f32, y: f32) {
        for (_, dst, _, _, _) in self.items.iter_mut() {
            *dst = *dst + Vector2::new(x, y);
        }
    }
}
