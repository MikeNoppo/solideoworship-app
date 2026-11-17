use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use iced::widget::shader::{Pipeline, Primitive, Program, Viewport};
use iced::{Element, Fill, Rectangle, mouse};
use sdw_core::MediaFit;
use sdw_media::{PixelFormat, VideoFrame};

pub fn view<'a, Message: 'a>(
    stream_id: u64,
    instance_id: u64,
    frame: Arc<VideoFrame>,
    fit: MediaFit,
    opacity: f32,
) -> Element<'a, Message> {
    iced::widget::shader(VideoProgram {
        stream_id,
        instance_id,
        frame,
        fit,
        opacity,
    })
    .width(Fill)
    .height(Fill)
    .into()
}

#[derive(Debug)]
struct VideoProgram {
    stream_id: u64,
    instance_id: u64,
    frame: Arc<VideoFrame>,
    fit: MediaFit,
    opacity: f32,
}

impl<Message> Program<Message> for VideoProgram {
    type State = ();
    type Primitive = VideoPrimitive;

    fn draw(
        &self,
        _state: &Self::State,
        _cursor: mouse::Cursor,
        _bounds: Rectangle,
    ) -> Self::Primitive {
        VideoPrimitive {
            stream_id: self.stream_id,
            instance_id: self.instance_id,
            frame: Arc::clone(&self.frame),
            fit: self.fit,
            opacity: self.opacity,
        }
    }
}

#[derive(Debug)]
pub struct VideoPrimitive {
    stream_id: u64,
    instance_id: u64,
    frame: Arc<VideoFrame>,
    fit: MediaFit,
    opacity: f32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
struct Uniforms {
    pos_scale: [f32; 2],
    uv_offset: [f32; 2],
    uv_scale: [f32; 2],
    is_nv12: u32,
    is_srgb: u32,
    opacity: f32,
    _pad: [f32; 3],
}

impl Uniforms {
    fn compute(
        bounds: &Rectangle,
        frame_w: u32,
        frame_h: u32,
        fit: MediaFit,
        format: PixelFormat,
        is_srgb: bool,
        opacity: f32,
    ) -> Self {
        let box_aspect = if bounds.height > 0.0 {
            bounds.width / bounds.height
        } else {
            16.0 / 9.0
        };
        let frame_aspect = if frame_h > 0 {
            frame_w as f32 / frame_h as f32
        } else {
            16.0 / 9.0
        };

        let (pos_scale, uv_offset, uv_scale) = match fit {
            MediaFit::Fill => ([1.0, 1.0], [0.0, 0.0], [1.0, 1.0]),
            MediaFit::Cover => {
                if frame_aspect > box_aspect {
                    let sx = (box_aspect / frame_aspect).clamp(0.0, 1.0);
                    ([1.0, 1.0], [(1.0 - sx) * 0.5, 0.0], [sx, 1.0])
                } else {
                    let sy = (frame_aspect / box_aspect).clamp(0.0, 1.0);
                    ([1.0, 1.0], [0.0, (1.0 - sy) * 0.5], [1.0, sy])
                }
            }
            MediaFit::Contain => {
                if frame_aspect > box_aspect {
                    let py = (box_aspect / frame_aspect).clamp(0.0, 1.0);
                    ([1.0, py], [0.0, 0.0], [1.0, 1.0])
                } else {
                    let px = (frame_aspect / box_aspect).clamp(0.0, 1.0);
                    ([px, 1.0], [0.0, 0.0], [1.0, 1.0])
                }
            }
        };

        Self {
            pos_scale,
            uv_offset,
            uv_scale,
            is_nv12: u32::from(format == PixelFormat::Nv12),
            is_srgb: u32::from(is_srgb),
            opacity: if opacity.is_finite() {
                opacity.clamp(0.0, 1.0)
            } else {
                0.0
            },
            _pad: [0.0; 3],
        }
    }
}

struct StreamEntry {
    last_frame_id: u64,
    width: u32,
    height: u32,
    format: PixelFormat,
    y_texture: wgpu::Texture,
    u_texture: wgpu::Texture,
    v_texture: Option<wgpu::Texture>,
    bind_group: wgpu::BindGroup,
    last_epoch: u64,
}

struct InstanceEntry {
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    last_uniforms: Uniforms,
    last_epoch: u64,
}

pub struct VideoPipeline {
    render_pipeline: wgpu::RenderPipeline,
    texture_layout: wgpu::BindGroupLayout,
    uniform_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    dummy_v_view: wgpu::TextureView,
    is_srgb: bool,
    epoch: u64,
    streams: HashMap<u64, StreamEntry>,
    instances: HashMap<u64, InstanceEntry>,
}

impl Pipeline for VideoPipeline {
    fn new(device: &wgpu::Device, _queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sdw-video-shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(WGSL)),
        });

        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sdw-video-texture-layout"),
            entries: &[
                texture_entry(0),
                texture_entry(1),
                texture_entry(2),
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sdw-video-uniform-layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sdw-video-pipeline-layout"),
            bind_group_layouts: &[&texture_layout, &uniform_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sdw-video-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("sdw-video-sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let dummy_v = create_plane_texture(device, 1, 1, wgpu::TextureFormat::R8Unorm, "dummy-v");
        let dummy_v_view = dummy_v.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            render_pipeline,
            texture_layout,
            uniform_layout,
            sampler,
            dummy_v_view,
            is_srgb: format.is_srgb(),
            epoch: 0,
            streams: HashMap::new(),
            instances: HashMap::new(),
        }
    }

    fn trim(&mut self) {
        self.epoch = self.epoch.wrapping_add(1);
        let cutoff = self.epoch.saturating_sub(120);
        self.streams.retain(|_, entry| entry.last_epoch >= cutoff);
        self.instances.retain(|_, entry| entry.last_epoch >= cutoff);
    }
}

impl Primitive for VideoPrimitive {
    type Pipeline = VideoPipeline;

    fn prepare(
        &self,
        pipeline: &mut Self::Pipeline,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bounds: &Rectangle,
        _viewport: &Viewport,
    ) {
        let w = self.frame.width();
        let h = self.frame.height();
        let format = self.frame.format();
        if w == 0 || h == 0 {
            return;
        }

        let current_epoch = pipeline.epoch;
        let recreate_stream = pipeline
            .streams
            .get(&self.stream_id)
            .is_none_or(|entry| entry.width != w || entry.height != h || entry.format != format);

        if recreate_stream {
            let y_texture = create_plane_texture(device, w, h, wgpu::TextureFormat::R8Unorm, "y");
            let chroma_w = w.div_ceil(2);
            let chroma_h = h.div_ceil(2);
            let (u_texture, v_texture) = match format {
                PixelFormat::Nv12 => (
                    create_plane_texture(
                        device,
                        chroma_w,
                        chroma_h,
                        wgpu::TextureFormat::Rg8Unorm,
                        "uv",
                    ),
                    None,
                ),
                PixelFormat::Yuv420p => (
                    create_plane_texture(
                        device,
                        chroma_w,
                        chroma_h,
                        wgpu::TextureFormat::R8Unorm,
                        "u",
                    ),
                    Some(create_plane_texture(
                        device,
                        chroma_w,
                        chroma_h,
                        wgpu::TextureFormat::R8Unorm,
                        "v",
                    )),
                ),
            };

            let y_view = y_texture.create_view(&wgpu::TextureViewDescriptor::default());
            let u_view = u_texture.create_view(&wgpu::TextureViewDescriptor::default());
            let v_view_owned = v_texture
                .as_ref()
                .map(|tex| tex.create_view(&wgpu::TextureViewDescriptor::default()));
            let v_view = v_view_owned.as_ref().unwrap_or(&pipeline.dummy_v_view);

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("sdw-video-texture-bind-group"),
                layout: &pipeline.texture_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&y_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&u_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(v_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::Sampler(&pipeline.sampler),
                    },
                ],
            });

            pipeline.streams.insert(
                self.stream_id,
                StreamEntry {
                    last_frame_id: 0,
                    width: w,
                    height: h,
                    format,
                    y_texture,
                    u_texture,
                    v_texture,
                    bind_group,
                    last_epoch: current_epoch,
                },
            );
        }

        if let Some(stream) = pipeline.streams.get_mut(&self.stream_id) {
            stream.last_epoch = current_epoch;
            if stream.last_frame_id != self.frame.id() {
                upload_plane(queue, &stream.y_texture, self.frame.y_plane(), w, h, 1);
                let chroma_w = w.div_ceil(2);
                let chroma_h = h.div_ceil(2);
                match format {
                    PixelFormat::Nv12 => {
                        upload_plane(
                            queue,
                            &stream.u_texture,
                            self.frame.u_or_uv_plane(),
                            chroma_w,
                            chroma_h,
                            2,
                        );
                    }
                    PixelFormat::Yuv420p => {
                        upload_plane(
                            queue,
                            &stream.u_texture,
                            self.frame.u_or_uv_plane(),
                            chroma_w,
                            chroma_h,
                            1,
                        );
                        if let Some(v_tex) = &stream.v_texture {
                            upload_plane(queue, v_tex, self.frame.v_plane(), chroma_w, chroma_h, 1);
                        }
                    }
                }
                stream.last_frame_id = self.frame.id();
            }
        }

        let uniforms = Uniforms::compute(
            bounds,
            w,
            h,
            self.fit,
            format,
            pipeline.is_srgb,
            self.opacity,
        );

        let entry = pipeline
            .instances
            .entry(self.instance_id)
            .or_insert_with(|| {
                let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("sdw-video-uniforms"),
                    size: std::mem::size_of::<Uniforms>() as u64,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                queue.write_buffer(&buffer, 0, bytemuck::bytes_of(&uniforms));
                let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("sdw-video-uniform-bind-group"),
                    layout: &pipeline.uniform_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: buffer.as_entire_binding(),
                    }],
                });
                InstanceEntry {
                    buffer,
                    bind_group,
                    last_uniforms: uniforms,
                    last_epoch: current_epoch,
                }
            });

        entry.last_epoch = current_epoch;
        if entry.last_uniforms != uniforms {
            queue.write_buffer(&entry.buffer, 0, bytemuck::bytes_of(&uniforms));
            entry.last_uniforms = uniforms;
        }
    }

    fn draw(&self, pipeline: &Self::Pipeline, render_pass: &mut wgpu::RenderPass<'_>) -> bool {
        let Some(stream) = pipeline.streams.get(&self.stream_id) else {
            return false;
        };
        let Some(instance) = pipeline.instances.get(&self.instance_id) else {
            return false;
        };

        render_pass.set_pipeline(&pipeline.render_pipeline);
        render_pass.set_bind_group(0, &stream.bind_group, &[]);
        render_pass.set_bind_group(1, &instance.bind_group, &[]);
        render_pass.draw(0..4, 0..1);
        true
    }
}

fn texture_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

fn create_plane_texture(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
    label: &str,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    })
}

fn upload_plane(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    data: &[u8],
    width: u32,
    height: u32,
    bytes_per_pixel: u32,
) {
    let row_bytes = width.saturating_mul(bytes_per_pixel);
    let expected = (row_bytes as usize).saturating_mul(height as usize);
    if data.len() < expected || width == 0 || height == 0 {
        return;
    }

    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &data[..expected],
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(row_bytes),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
}

const WGSL: &str = "
struct Uniforms {
    pos_scale: vec2<f32>,
    uv_offset: vec2<f32>,
    uv_scale: vec2<f32>,
    is_nv12: u32,
    is_srgb: u32,
    opacity: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
};

@group(0) @binding(0) var y_tex: texture_2d<f32>;
@group(0) @binding(1) var u_tex: texture_2d<f32>;
@group(0) @binding(2) var v_tex: texture_2d<f32>;
@group(0) @binding(3) var video_sampler: sampler;
@group(1) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VertexOut {
    var corners = array<vec2<f32>, 4>(
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
    );
    var uvs = array<vec2<f32>, 4>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0),
    );
    var out: VertexOut;
    let pos = corners[idx] * uniforms.pos_scale;
    out.position = vec4<f32>(pos, 0.0, 1.0);
    out.uv = uniforms.uv_offset + uvs[idx] * uniforms.uv_scale;
    return out;
}

fn srgb_channel_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        return c / 12.92;
    }
    return pow((c + 0.055) / 1.055, 2.4);
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let y_sample = textureSample(y_tex, video_sampler, in.uv).r;
    var u_sample: f32;
    var v_sample: f32;
    if uniforms.is_nv12 == 1u {
        let uv = textureSample(u_tex, video_sampler, in.uv).rg;
        u_sample = uv.r;
        v_sample = uv.g;
    } else {
        u_sample = textureSample(u_tex, video_sampler, in.uv).r;
        v_sample = textureSample(v_tex, video_sampler, in.uv).r;
    }

    let y = (y_sample - 0.0627451) * 1.1643836;
    let u = u_sample - 0.5019608;
    let v = v_sample - 0.5019608;

    let r = y + 1.5748 * v;
    let g = y - 0.187324 * u - 0.468124 * v;
    let b = y + 1.8556 * u;
    var rgb = clamp(vec3<f32>(r, g, b), vec3<f32>(0.0), vec3<f32>(1.0));

    if uniforms.is_srgb == 1u {
        rgb = vec3<f32>(
            srgb_channel_to_linear(rgb.r),
            srgb_channel_to_linear(rgb.g),
            srgb_channel_to_linear(rgb.b),
        );
    }

    let a = uniforms.opacity;
    return vec4<f32>(rgb * a, a);
}
";
