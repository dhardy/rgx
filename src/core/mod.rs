#![deny(clippy::all, clippy::use_self)]
#![allow(clippy::cast_lossless)]

use std::ops::Range;
use std::{mem, ptr};

use cgmath::Vector2;

///////////////////////////////////////////////////////////////////////////////
/// Rect
///////////////////////////////////////////////////////////////////////////////

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Rect<T> {
    pub x1: T,
    pub y1: T,
    pub x2: T,
    pub y2: T,
}

impl<T> Rect<T> {
    pub fn new(x1: T, y1: T, x2: T, y2: T) -> Self {
        Self { x1, y1, x2, y2 }
    }

    pub fn empty() -> Self
    where
        T: cgmath::Zero,
    {
        Self {
            x1: T::zero(),
            x2: T::zero(),
            y1: T::zero(),
            y2: T::zero(),
        }
    }

    pub fn origin(w: T, h: T) -> Self
    where
        T: cgmath::Zero,
    {
        Self::new(T::zero(), T::zero(), w, h)
    }

    pub fn scale(&self, x: T, y: T) -> Self
    where
        T: std::ops::Mul<Output = T> + Copy,
    {
        Self {
            x1: self.x1,
            y1: self.y1,
            x2: self.x2 * x,
            y2: self.y2 * y,
        }
    }

    pub fn translate(&self, x: T, y: T) -> Self
    where
        T: std::ops::Add<Output = T> + std::ops::Sub<Output = T> + Copy,
    {
        Self {
            x1: x,
            y1: y,
            x2: x + (self.x2 - self.x1),
            y2: y + (self.y2 - self.y1),
        }
    }

    pub fn is_empty(&self) -> bool
    where
        T: PartialEq,
    {
        self.x1 == self.x2 && self.y1 == self.y2
    }

    pub fn is_zero(&self) -> bool
    where
        T: cgmath::Zero,
    {
        self.x1.is_zero() && self.x2.is_zero() && self.y1.is_zero() && self.y2.is_zero()
    }

    pub fn width(&self) -> T
    where
        T: Copy + PartialOrd + std::ops::Sub<Output = T> + std::ops::Neg<Output = T> + cgmath::Zero,
    {
        let w = self.x2 - self.x1;
        if w < T::zero() {
            -w
        } else {
            w
        }
    }

    pub fn height(&self) -> T
    where
        T: Copy + PartialOrd + std::ops::Sub<Output = T> + std::ops::Neg<Output = T> + cgmath::Zero,
    {
        let h = self.y2 - self.y1;
        if h < T::zero() {
            -h
        } else {
            h
        }
    }

    pub fn center(&self) -> Vector2<T>
    where
        T: std::ops::Div<Output = T>
            + Copy
            + From<i16>
            + PartialOrd
            + cgmath::Zero
            + std::ops::Neg<Output = T>
            + std::ops::Sub<Output = T>,
    {
        // TODO: Should be normalized for inverted rectangles.
        Vector2::new(
            self.x1 + self.width() / 2.into(),
            self.y1 + self.height() / 2.into(),
        )
    }

    pub fn radius(&self) -> T
    where
        T: std::ops::Div<Output = T>
            + Copy
            + From<i16>
            + PartialOrd
            + cgmath::Zero
            + std::ops::Neg<Output = T>
            + std::ops::Sub<Output = T>,
    {
        let w = self.width();
        let h = self.height();

        if w > h {
            w / 2.into()
        } else {
            h / 2.into()
        }
    }
}

impl<T> std::ops::Add<Vector2<T>> for Rect<T>
where
    T: std::ops::Add<Output = T> + Copy,
{
    type Output = Self;

    fn add(self, vec: Vector2<T>) -> Self {
        Self {
            x1: self.x1 + vec.x,
            y1: self.y1 + vec.y,
            x2: self.x2 + vec.x,
            y2: self.y2 + vec.y,
        }
    }
}

///////////////////////////////////////////////////////////////////////////////
/// Draw
///////////////////////////////////////////////////////////////////////////////

pub trait Draw {
    fn draw(&self, binding: &BindingGroup, pass: &mut Pass);
}

///////////////////////////////////////////////////////////////////////////////
/// Rgba
///////////////////////////////////////////////////////////////////////////////

#[derive(Copy, Clone, PartialEq)]
pub struct Rgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Rgba {
    pub const WHITE: Self = Self {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };
    pub const TRANSPARENT: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };

    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    fn to_wgpu(&self) -> wgpu::Color {
        wgpu::Color {
            r: self.r,
            g: self.g,
            b: self.b,
            a: self.a,
        }
    }
}

///////////////////////////////////////////////////////////////////////////////
/// Shaders
///////////////////////////////////////////////////////////////////////////////

pub struct Shader {
    module: wgpu::ShaderModule,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ShaderStage {
    Vertex,
    Fragment,
    Compute,
}

impl ShaderStage {
    fn to_wgpu(&self) -> wgpu::ShaderStage {
        match self {
            ShaderStage::Vertex => wgpu::ShaderStage::VERTEX,
            ShaderStage::Fragment => wgpu::ShaderStage::FRAGMENT,
            ShaderStage::Compute => wgpu::ShaderStage::COMPUTE,
        }
    }
}

///////////////////////////////////////////////////////////////////////////////
/// Resource
///////////////////////////////////////////////////////////////////////////////

/// Anything that needs to be submitted to the GPU before the frame starts.
pub trait Resource {
    fn prepare(&self, encoder: &mut wgpu::CommandEncoder);
}

///////////////////////////////////////////////////////////////////////////////
/// BindingGroup
///////////////////////////////////////////////////////////////////////////////

/// A group of bindings.
pub struct BindingGroup {
    wgpu: wgpu::BindGroup,
    set_index: u32,
}

impl BindingGroup {
    fn new(set_index: u32, wgpu: wgpu::BindGroup) -> Self {
        Self { set_index, wgpu }
    }
}

/// The layout of a 'BindingGroup'.
pub struct BindingGroupLayout {
    wgpu: wgpu::BindGroupLayout,
    size: usize,
    set_index: u32,
}

impl BindingGroupLayout {
    fn new(set_index: u32, layout: wgpu::BindGroupLayout, size: usize) -> Self {
        Self {
            wgpu: layout,
            size,
            set_index,
        }
    }
}

/// A trait representing a resource that can be bound.
pub trait Bind {
    fn binding(&self, index: u32) -> wgpu::Binding;
}

///////////////////////////////////////////////////////////////////////////////
/// Uniforms
///////////////////////////////////////////////////////////////////////////////

/// A uniform buffer that can be bound in a 'BindingGroup'.
pub struct UniformBuffer {
    wgpu: wgpu::Buffer,
    size: usize,
}

impl Bind for UniformBuffer {
    fn binding(&self, index: u32) -> wgpu::Binding {
        wgpu::Binding {
            binding: index as u32,
            resource: wgpu::BindingResource::Buffer {
                buffer: &self.wgpu,
                range: 0..(self.size as wgpu::BufferAddress),
            },
        }
    }
}

///////////////////////////////////////////////////////////////////////////////
/// Framebuffer
///////////////////////////////////////////////////////////////////////////////

#[allow(dead_code)]
pub struct Framebuffer {
    texture: wgpu::Texture,
    texture_view: wgpu::TextureView,
    extent: wgpu::Extent3d,

    buffer: Option<wgpu::Buffer>,

    pub w: u32,
    pub h: u32,
}

impl Framebuffer {
    pub fn size(&self) -> usize {
        (self.w * self.h) as usize
    }
}

impl Bind for Framebuffer {
    fn binding(&self, index: u32) -> wgpu::Binding {
        wgpu::Binding {
            binding: index as u32,
            resource: wgpu::BindingResource::TextureView(&self.texture_view),
        }
    }
}

impl Resource for &Framebuffer {
    fn prepare(&self, encoder: &mut wgpu::CommandEncoder) {
        // If we have a buffer to upload, treat the Framebuffer as a Texture.
        if let Some(buffer) = &self.buffer {
            Texture::blit(&self.texture, self.w, self.h, self.extent, buffer, encoder);
        }
    }
}

///////////////////////////////////////////////////////////////////////////////
/// Texturing
///////////////////////////////////////////////////////////////////////////////

#[allow(dead_code)]
pub struct Texture {
    wgpu: wgpu::Texture,
    view: wgpu::TextureView,
    extent: wgpu::Extent3d,
    buffer: wgpu::Buffer,

    pub w: u32,
    pub h: u32,
}

impl Texture {
    pub fn rect(&self) -> Rect<f32> {
        Rect {
            x1: 0.0,
            y1: 0.0,
            x2: self.w as f32,
            y2: self.h as f32,
        }
    }

    fn blit(
        texture: &wgpu::Texture,
        w: u32,
        h: u32,
        extent: wgpu::Extent3d,
        buffer: &wgpu::Buffer,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        encoder.copy_buffer_to_texture(
            wgpu::BufferCopyView {
                buffer,
                offset: 0,
                row_pitch: 4 * w,
                image_height: h,
            },
            wgpu::TextureCopyView {
                texture,
                mip_level: 0,
                array_layer: 0,
                origin: wgpu::Origin3d {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
            },
            extent,
        );
    }
}

impl Bind for Texture {
    fn binding(&self, index: u32) -> wgpu::Binding {
        wgpu::Binding {
            binding: index as u32,
            resource: wgpu::BindingResource::TextureView(&self.view),
        }
    }
}

impl Resource for &Texture {
    fn prepare(&self, encoder: &mut wgpu::CommandEncoder) {
        Texture::blit(
            &self.wgpu,
            self.w,
            self.h,
            self.extent,
            &self.buffer,
            encoder,
        );
    }
}

pub struct Sampler {
    wgpu: wgpu::Sampler,
}

impl Bind for Sampler {
    fn binding(&self, index: u32) -> wgpu::Binding {
        wgpu::Binding {
            binding: index as u32,
            resource: wgpu::BindingResource::Sampler(&self.wgpu),
        }
    }
}

#[derive(Debug)]
pub enum Filter {
    Nearest,
    Linear,
}

impl Filter {
    fn to_wgpu(&self) -> wgpu::FilterMode {
        match self {
            Filter::Nearest => wgpu::FilterMode::Nearest,
            Filter::Linear => wgpu::FilterMode::Linear,
        }
    }
}

///////////////////////////////////////////////////////////////////////////////
/// Vertex/Index Buffers
///////////////////////////////////////////////////////////////////////////////

pub struct VertexBuffer {
    pub size: u32,
    wgpu: wgpu::Buffer,
}

impl Draw for VertexBuffer {
    fn draw(&self, binding: &BindingGroup, pass: &mut Pass) {
        // TODO: If we attempt to draw more vertices than exist in the buffer, because
        // 'size' was guessed wrong, we get a wgpu error. We should somehow try to
        // get the pipeline layout to know here if the buffer we're trying to draw
        // is the right size. Another option is to create buffers from the pipeline,
        // so that we can check at creation time whether the data passed in matches
        // the format.
        pass.apply_binding(binding, &[]);
        pass.set_vertex_buffer(&self);
        pass.draw_buffer(0..self.size, 0..1);
    }
}

pub struct IndexBuffer {
    wgpu: wgpu::Buffer,
}

#[derive(Clone, Copy)]
pub enum VertexFormat {
    Float,
    Float2,
    Float3,
    Float4,
    UByte4,
}

impl VertexFormat {
    // TODO: Use `const fn`
    fn bytesize(self) -> usize {
        match self {
            VertexFormat::Float => 4,
            VertexFormat::Float2 => 8,
            VertexFormat::Float3 => 12,
            VertexFormat::Float4 => 16,
            VertexFormat::UByte4 => 4,
        }
    }
    // TODO: Use `const fn`
    fn to_wgpu(self) -> wgpu::VertexFormat {
        match self {
            VertexFormat::Float => wgpu::VertexFormat::Float,
            VertexFormat::Float2 => wgpu::VertexFormat::Float2,
            VertexFormat::Float3 => wgpu::VertexFormat::Float3,
            VertexFormat::Float4 => wgpu::VertexFormat::Float4,
            VertexFormat::UByte4 => wgpu::VertexFormat::Uchar4Norm,
        }
    }
}

/// Describes a 'VertexBuffer' layout.
#[derive(Default)]
pub struct VertexLayout {
    wgpu_attrs: Vec<wgpu::VertexAttributeDescriptor>,
    size: usize,
}

impl VertexLayout {
    pub fn from(formats: &[VertexFormat]) -> Self {
        let mut vl = Self::default();
        for vf in formats {
            vl.wgpu_attrs.push(wgpu::VertexAttributeDescriptor {
                shader_location: vl.wgpu_attrs.len() as u32,
                offset: vl.size as wgpu::BufferAddress,
                format: vf.to_wgpu(),
            });
            vl.size += vf.bytesize();
        }
        vl
    }

    fn to_wgpu(&self) -> wgpu::VertexBufferDescriptor {
        wgpu::VertexBufferDescriptor {
            stride: self.size as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: self.wgpu_attrs.as_slice(),
        }
    }
}

///////////////////////////////////////////////////////////////////////////////
/// Pipeline Bindings
///////////////////////////////////////////////////////////////////////////////

/// A binding type.
pub enum BindingType {
    UniformBuffer,
    Sampler,
    SampledTexture,
}

impl BindingType {
    fn to_wgpu(&self) -> wgpu::BindingType {
        match self {
            BindingType::UniformBuffer => wgpu::BindingType::UniformBufferDynamic,
            BindingType::SampledTexture => wgpu::BindingType::SampledTexture,
            BindingType::Sampler => wgpu::BindingType::Sampler,
        }
    }
}

pub struct Binding {
    pub binding: BindingType,
    pub stage: ShaderStage,
}

///////////////////////////////////////////////////////////////////////////////
/// Pipeline
///////////////////////////////////////////////////////////////////////////////

pub struct Pipeline {
    wgpu: wgpu::RenderPipeline,

    pub layout: PipelineLayout,
    pub vertex_layout: VertexLayout,
}

impl<'a> AbstractPipeline<'a> for Pipeline {
    type PrepareContext = ();
    type Uniforms = ();

    fn description() -> PipelineDescription<'a> {
        PipelineDescription {
            vertex_layout: &[],
            pipeline_layout: &[],
            vertex_shader: "",
            fragment_shader: "",
        }
    }

    fn setup(pipeline: Self, _dev: &Device, _w: u32, _h: u32) -> Self {
        pipeline
    }

    fn apply(&self, pass: &mut Pass) {
        pass.wgpu.set_pipeline(&self.wgpu);
    }

    fn resize(&mut self, _w: u32, _h: u32) {}

    fn prepare(&'a self, _unused: ()) -> Option<(&'a UniformBuffer, Vec<()>)> {
        None
    }
}

pub struct Set<'a>(pub &'a [Binding]);

pub struct PipelineLayout {
    pub sets: Vec<BindingGroupLayout>,
}

pub trait AbstractPipeline<'a> {
    type PrepareContext;
    type Uniforms: Copy + 'static;

    fn description() -> PipelineDescription<'a>;
    fn setup(pip: Pipeline, dev: &Device, w: u32, h: u32) -> Self;
    fn apply(&self, pass: &mut Pass);
    fn resize(&mut self, w: u32, h: u32);
    fn prepare(
        &'a self,
        t: Self::PrepareContext,
    ) -> Option<(&'a UniformBuffer, Vec<Self::Uniforms>)>;
}

pub struct PipelineDescription<'a> {
    pub vertex_layout: &'a [VertexFormat],
    pub pipeline_layout: &'a [Set<'a>],
    pub vertex_shader: &'static str,
    pub fragment_shader: &'static str,
}

///////////////////////////////////////////////////////////////////////////////
/// Frame
///////////////////////////////////////////////////////////////////////////////

enum OnDrop {
    ReadAsync(wgpu::Buffer, usize, Box<FnMut(&[u32])>),
}

pub struct Frame<'a> {
    encoder: mem::ManuallyDrop<wgpu::CommandEncoder>,
    texture: wgpu::SwapChainOutput<'a>,
    device: &'a mut Device,
    on_drop: Vec<OnDrop>,
}

impl<'a> Drop for Frame<'a> {
    fn drop(&mut self) {
        let e = unsafe { mem::ManuallyDrop::into_inner(ptr::read(&self.encoder)) };
        self.device.submit(&[e.finish()]);

        for a in self.on_drop.drain(..) {
            match a {
                OnDrop::ReadAsync(buf, size, mut f) => {
                    buf.map_read_async(
                        0,
                        size as u64,
                        move |result: wgpu::BufferMapAsyncResult<&[u32]>| match result {
                            Ok(ref mapping) => {
                                f(mapping.data);
                            }
                            Err(ref err) => panic!("{:?}", err),
                        },
                    );
                }
            }
        }
    }
}

impl<'a> Frame<'a> {
    pub fn new(
        encoder: wgpu::CommandEncoder,
        texture: wgpu::SwapChainOutput<'a>,
        device: &'a mut Device,
    ) -> Frame<'a> {
        Frame {
            texture,
            device,
            encoder: mem::ManuallyDrop::new(encoder),
            on_drop: Vec::new(),
        }
    }

    pub fn prepare<T>(&mut self, pip: &'a T, p: T::PrepareContext)
    where
        T: AbstractPipeline<'a>,
    {
        if let Some((buf, unifs)) = pip.prepare(p) {
            self.update_uniform_buffer(buf, unifs.as_slice());
        }
    }

    pub fn offscreen_pass(&mut self, clear: Rgba, fb: &Framebuffer) -> Pass {
        Pass::begin(&mut self.encoder, &fb.texture_view, clear)
    }

    pub fn pass(&mut self, clear: Rgba) -> Pass {
        Pass::begin(&mut self.encoder, &self.texture.view, clear)
    }

    pub fn update_uniform_buffer<T>(&mut self, u: &UniformBuffer, buf: &[T])
    where
        T: 'static + Copy,
    {
        let src = self
            .device
            .device
            .create_buffer_mapped::<T>(
                buf.len(),
                wgpu::BufferUsage::UNIFORM
                    | wgpu::BufferUsage::TRANSFER_SRC
                    | wgpu::BufferUsage::MAP_WRITE,
            )
            .fill_from_slice(buf);

        self.encoder.copy_buffer_to_buffer(
            &src,
            0,
            &u.wgpu,
            0,
            (std::mem::size_of::<T>() * buf.len()) as wgpu::BufferAddress,
        );
    }

    pub fn read_async<F>(&mut self, fb: &Framebuffer, f: F)
    where
        F: 'static + FnMut(&[u32]),
    {
        let bytesize = 4 * fb.size();
        let dst = self.device.device.create_buffer(&wgpu::BufferDescriptor {
            size: bytesize as u64,
            usage: wgpu::BufferUsage::MAP_READ | wgpu::BufferUsage::TRANSFER_DST,
        });

        self.encoder.copy_texture_to_buffer(
            wgpu::TextureCopyView {
                texture: &fb.texture,
                mip_level: 0,
                array_layer: 0,
                origin: wgpu::Origin3d {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
            },
            wgpu::BufferCopyView {
                buffer: &dst,
                offset: 0,
                // TODO: Must be a multiple of 256
                row_pitch: 4 * fb.w,
                image_height: fb.h,
            },
            fb.extent,
        );

        self.on_drop
            .push(OnDrop::ReadAsync(dst, bytesize, Box::new(f)));
    }
}

///////////////////////////////////////////////////////////////////////////////
/// Pass
///////////////////////////////////////////////////////////////////////////////

pub struct Pass<'a> {
    wgpu: wgpu::RenderPass<'a>,
}

impl<'a> Pass<'a> {
    pub fn begin(
        encoder: &'a mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        clear_color: Rgba,
    ) -> Self {
        let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                attachment: &view,
                load_op: wgpu::LoadOp::Clear,
                store_op: wgpu::StoreOp::Store,
                clear_color: clear_color.to_wgpu(),
                resolve_target: None,
            }],
            depth_stencil_attachment: None,
        });
        Pass { wgpu: pass }
    }
    pub fn apply_pipeline<T>(&mut self, pipeline: &T)
    where
        T: AbstractPipeline<'a>,
    {
        pipeline.apply(self);
    }
    pub fn apply_binding(&mut self, group: &BindingGroup, offsets: &[u64]) {
        self.wgpu
            .set_bind_group(group.set_index, &group.wgpu, offsets);
    }
    pub fn set_index_buffer(&mut self, index_buf: &IndexBuffer) {
        self.wgpu.set_index_buffer(&index_buf.wgpu, 0)
    }
    pub fn set_vertex_buffer(&mut self, vertex_buf: &VertexBuffer) {
        self.wgpu.set_vertex_buffers(&[(&vertex_buf.wgpu, 0)])
    }
    pub fn draw<T: Draw>(&mut self, drawable: &T, binding: &BindingGroup) {
        drawable.draw(binding, self);
    }
    pub fn draw_buffer(&mut self, indices: Range<u32>, instances: Range<u32>) {
        self.wgpu.draw(indices, instances)
    }
    pub fn draw_indexed(&mut self, indices: Range<u32>, instances: Range<u32>) {
        self.wgpu.draw_indexed(indices, 0, instances)
    }
}

fn swap_chain_descriptor(width: u32, height: u32) -> wgpu::SwapChainDescriptor {
    wgpu::SwapChainDescriptor {
        usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
        format: wgpu::TextureFormat::Bgra8Unorm,
        width,
        height,
    }
}

///////////////////////////////////////////////////////////////////////////////
/// Renderer
///////////////////////////////////////////////////////////////////////////////

pub struct Renderer {
    pub device: Device,
    swap_chain: wgpu::SwapChain,
}

impl Renderer {
    pub fn new(window: &wgpu::winit::Window) -> Self {
        let size = window
            .get_inner_size()
            .unwrap()
            .to_physical(window.get_hidpi_factor());
        let device = Device::new(window);
        let swap_chain = device.create_swap_chain(size.width as u32, size.height as u32);

        Self { device, swap_chain }
    }

    pub fn texture(&self, texels: &[u8], w: u32, h: u32) -> Texture {
        self.device.create_texture(texels, w, h)
    }

    pub fn framebuffer(&self, texels: &[u8], w: u32, h: u32) -> Framebuffer {
        self.device.create_framebuffer(texels, w, h)
    }

    pub fn vertexbuffer<T>(&self, verts: &[T]) -> VertexBuffer
    where
        T: 'static + Copy,
    {
        self.device.create_buffer(verts)
    }

    pub fn uniform_buffer<T>(&self, buf: &[T]) -> UniformBuffer
    where
        T: 'static + Copy,
    {
        self.device.create_uniform_buffer(buf)
    }

    pub fn binding_group(&self, layout: &BindingGroupLayout, binds: &[&dyn Bind]) -> BindingGroup {
        self.device.create_binding_group(layout, binds)
    }

    pub fn sampler(&self, min_filter: Filter, mag_filter: Filter) -> Sampler {
        self.device.create_sampler(min_filter, mag_filter)
    }

    pub fn pipeline<T>(&self, w: u32, h: u32) -> T
    where
        T: AbstractPipeline<'static>,
    {
        let desc = T::description();
        let pip_layout = self.device.create_pipeline_layout(desc.pipeline_layout);
        let vertex_layout = VertexLayout::from(desc.vertex_layout);
        let vs =
            self.device
                .create_shader("vertex shader", desc.vertex_shader, ShaderStage::Vertex);
        let fs = self.device.create_shader(
            "fragment shader",
            desc.fragment_shader,
            ShaderStage::Fragment,
        );

        T::setup(
            self.device
                .create_pipeline(pip_layout, vertex_layout, &vs, &fs),
            &self.device,
            w,
            h,
        )
    }

    // MUTABLE API ////////////////////////////////////////////////////////////

    pub fn resize(&mut self, w: u32, h: u32) {
        self.swap_chain = self.device.create_swap_chain(w, h);
    }

    pub fn frame(&mut self) -> Frame {
        let texture = self.swap_chain.get_next_texture();
        let encoder = self.device.create_command_encoder();
        Frame::new(encoder, texture, &mut self.device)
    }

    pub fn prepare<T: Resource>(&mut self, resources: &[T]) {
        let mut encoder = self.device.create_command_encoder();
        for r in resources.iter() {
            r.prepare(&mut encoder);
        }
        self.device.submit(&[encoder.finish()]);
    }
}

///////////////////////////////////////////////////////////////////////////////
/// Device
///////////////////////////////////////////////////////////////////////////////

pub struct Device {
    device: wgpu::Device,
    surface: wgpu::Surface,
}

impl Device {
    pub fn new(window: &wgpu::winit::Window) -> Self {
        let instance = wgpu::Instance::new();
        let adapter = instance.get_adapter(&wgpu::AdapterDescriptor {
            power_preference: wgpu::PowerPreference::LowPower,
        });
        let surface = instance.create_surface(&window);

        Self {
            device: adapter.request_device(&wgpu::DeviceDescriptor {
                extensions: wgpu::Extensions {
                    anisotropic_filtering: false,
                },
                limits: wgpu::Limits::default(),
            }),
            surface,
        }
    }

    pub fn create_command_encoder(&self) -> wgpu::CommandEncoder {
        self.device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 })
    }

    pub fn create_swap_chain(&self, w: u32, h: u32) -> wgpu::SwapChain {
        let desc = swap_chain_descriptor(w, h);
        self.device.create_swap_chain(&self.surface, &desc)
    }

    pub fn create_pipeline_layout(&self, ss: &[Set]) -> PipelineLayout {
        let mut sets = Vec::new();
        for (i, s) in ss.iter().enumerate() {
            sets.push(self.create_binding_group_layout(i as u32, s.0))
        }
        PipelineLayout { sets }
    }

    pub fn create_shader(&self, name: &str, source: &str, stage: ShaderStage) -> Shader {
        let ty = match stage {
            ShaderStage::Vertex => shaderc::ShaderKind::Vertex,
            ShaderStage::Fragment => shaderc::ShaderKind::Fragment,
            ShaderStage::Compute => shaderc::ShaderKind::Compute,
        };

        let mut compiler = shaderc::Compiler::new().unwrap();
        let options = shaderc::CompileOptions::new().unwrap();

        let result = compiler.compile_into_spirv(source, ty, name, "main", Some(&options));

        let spv = match result {
            Ok(spv) => spv,
            Err(err) => match err {
                shaderc::Error::CompilationError(_, err) => {
                    panic!(err);
                }
                _ => unimplemented!(),
            },
        };
        Shader {
            module: self.device.create_shader_module(spv.as_binary_u8()),
        }
    }

    pub fn create_encoder(&self) -> wgpu::CommandEncoder {
        self.device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 })
    }

    pub fn create_texture(&self, texels: &[u8], w: u32, h: u32) -> Texture {
        assert_eq!(
            texels.len() as u32,
            w * h * 4,
            "wrong texture width or height given"
        );

        let texture_extent = wgpu::Extent3d {
            width: w,
            height: h,
            depth: 1,
        };
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            size: texture_extent,
            array_layer_count: 1,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::TRANSFER_DST,
        });
        let texture_view = texture.create_default_view();

        let buf = self
            .device
            .create_buffer_mapped(texels.len(), wgpu::BufferUsage::TRANSFER_SRC)
            .fill_from_slice(&texels);

        Texture {
            wgpu: texture,
            view: texture_view,
            extent: texture_extent,
            buffer: buf,
            w,
            h,
        }
    }

    pub fn create_framebuffer(&self, texels: &[u8], w: u32, h: u32) -> Framebuffer {
        let texture_extent = wgpu::Extent3d {
            width: w,
            height: h,
            depth: 1,
        };
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            size: texture_extent,
            array_layer_count: 1,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8Unorm,
            usage: wgpu::TextureUsage::SAMPLED
                | wgpu::TextureUsage::TRANSFER_DST
                | wgpu::TextureUsage::TRANSFER_SRC
                | wgpu::TextureUsage::OUTPUT_ATTACHMENT,
        });
        let texture_view = texture.create_default_view();

        let buffer = if texels.is_empty() {
            None
        } else {
            Some(
                self.device
                    .create_buffer_mapped(texels.len(), wgpu::BufferUsage::TRANSFER_SRC)
                    .fill_from_slice(&texels),
            )
        };

        Framebuffer {
            texture,
            texture_view,
            extent: texture_extent,
            buffer,
            w,
            h,
        }
    }

    pub fn create_binding_group(
        &self,
        layout: &BindingGroupLayout,
        binds: &[&dyn Bind],
    ) -> BindingGroup {
        assert_eq!(
            binds.len(),
            layout.size,
            "layout slot count does not match bindings"
        );

        let mut bindings = Vec::new();

        for (i, b) in binds.iter().enumerate() {
            bindings.push(b.binding(i as u32));
        }

        BindingGroup::new(
            layout.set_index,
            self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &layout.wgpu,
                bindings: bindings.as_slice(),
            }),
        )
    }

    pub fn create_buffer<T>(&self, vertices: &[T]) -> VertexBuffer
    where
        T: 'static + Copy,
    {
        VertexBuffer {
            wgpu: self
                .device
                .create_buffer_mapped(vertices.len(), wgpu::BufferUsage::VERTEX)
                .fill_from_slice(vertices),
            size: vertices.len() as u32,
        }
    }

    pub fn create_uniform_buffer<T>(&self, buf: &[T]) -> UniformBuffer
    where
        T: 'static + Copy,
    {
        UniformBuffer {
            size: std::mem::size_of::<T>(),
            wgpu: self
                .device
                .create_buffer_mapped::<T>(
                    buf.len(),
                    wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::TRANSFER_DST,
                )
                .fill_from_slice(buf),
        }
    }

    pub fn create_index(&self, indices: &[u16]) -> IndexBuffer {
        let index_buf = self
            .device
            .create_buffer_mapped(indices.len(), wgpu::BufferUsage::INDEX)
            .fill_from_slice(indices);
        IndexBuffer { wgpu: index_buf }
    }

    pub fn create_sampler(&self, min_filter: Filter, mag_filter: Filter) -> Sampler {
        Sampler {
            wgpu: self.device.create_sampler(&wgpu::SamplerDescriptor {
                address_mode_u: wgpu::AddressMode::Repeat,
                address_mode_v: wgpu::AddressMode::Repeat,
                address_mode_w: wgpu::AddressMode::Repeat,
                mag_filter: mag_filter.to_wgpu(),
                min_filter: min_filter.to_wgpu(),
                mipmap_filter: wgpu::FilterMode::Nearest,
                lod_min_clamp: -100.0,
                lod_max_clamp: 100.0,
                compare_function: wgpu::CompareFunction::Always,
            }),
        }
    }

    pub fn create_binding_group_layout(&self, index: u32, slots: &[Binding]) -> BindingGroupLayout {
        let mut bindings = Vec::new();

        for s in slots {
            bindings.push(wgpu::BindGroupLayoutBinding {
                binding: bindings.len() as u32,
                visibility: s.stage.to_wgpu(),
                ty: s.binding.to_wgpu(),
            });
        }
        let layout = self
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                bindings: bindings.as_slice(),
            });
        BindingGroupLayout::new(index, layout, bindings.len())
    }

    // MUTABLE API ////////////////////////////////////////////////////////////

    pub fn submit(&mut self, cmds: &[wgpu::CommandBuffer]) {
        self.device.get_queue().submit(cmds);
    }

    // PRIVATE API ////////////////////////////////////////////////////////////

    fn create_pipeline(
        &self,
        pipeline_layout: PipelineLayout,
        vertex_layout: VertexLayout,
        vs: &Shader,
        fs: &Shader,
    ) -> Pipeline {
        let vertex_attrs = vertex_layout.to_wgpu();

        let mut sets = Vec::new();
        for s in pipeline_layout.sets.iter() {
            sets.push(&s.wgpu);
        }
        let layout = &self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                bind_group_layouts: sets.as_slice(),
            });

        let wgpu = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                layout,
                vertex_stage: wgpu::PipelineStageDescriptor {
                    module: &vs.module,
                    entry_point: "main",
                },
                fragment_stage: Some(wgpu::PipelineStageDescriptor {
                    module: &fs.module,
                    entry_point: "main",
                }),
                rasterization_state: wgpu::RasterizationStateDescriptor {
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: wgpu::CullMode::None,
                    depth_bias: 0,
                    depth_bias_slope_scale: 0.0,
                    depth_bias_clamp: 0.0,
                },
                primitive_topology: wgpu::PrimitiveTopology::TriangleList,
                color_states: &[wgpu::ColorStateDescriptor {
                    format: wgpu::TextureFormat::Bgra8Unorm,
                    color_blend: wgpu::BlendDescriptor {
                        src_factor: wgpu::BlendFactor::SrcAlpha,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        operation: wgpu::BlendOperation::Add,
                    },
                    alpha_blend: wgpu::BlendDescriptor {
                        src_factor: wgpu::BlendFactor::SrcAlpha,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        operation: wgpu::BlendOperation::Add,
                    },
                    write_mask: wgpu::ColorWrite::ALL,
                }],
                depth_stencil_state: None,
                index_format: wgpu::IndexFormat::Uint16,
                vertex_buffers: &[vertex_attrs],
                sample_count: 1,
            });

        Pipeline {
            layout: pipeline_layout,
            vertex_layout,
            wgpu,
        }
    }
}
