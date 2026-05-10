use std::collections::HashMap;

use bytemuck::{Pod, Zeroable};
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};

use crate::{Camera2D, RenderError, RenderSize, RenderState, Sprite, TextureData, TextureId};

pub struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    render_pipeline: wgpu::RenderPipeline,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    textures: HashMap<TextureId, GpuTexture>,
    sprite_vertex_buffer: Option<wgpu::Buffer>,
    sprite_vertex_capacity: usize,
    size: RenderSize,
}

impl Renderer {
    /// Creates a renderer from raw platform window handles.
    ///
    /// # Safety
    ///
    /// The raw display and window handles must describe a valid live window and
    /// must remain valid for at least as long as the returned `Renderer` exists.
    /// The runtime upholds this by owning the `winit` window for the full event
    /// loop lifetime.
    pub async unsafe fn new(
        raw_display_handle: RawDisplayHandle,
        raw_window_handle: RawWindowHandle,
        size: RenderSize,
    ) -> Result<Self, RenderError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: platform_backends(),
            ..Default::default()
        });
        let surface = unsafe {
            instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
                raw_display_handle,
                raw_window_handle,
            })
        }
        .map_err(|error| RenderError::SurfaceCreation(error.to_string()))?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or(RenderError::AdapterUnavailable)?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("seishin render device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: platform_limits(),
                },
                None,
            )
            .await
            .map_err(|error| RenderError::DeviceRequest(error.to_string()))?;

        let capabilities = surface.get_capabilities(&adapter);
        let surface_format = capabilities
            .formats
            .iter()
            .copied()
            .find(wgpu::TextureFormat::is_srgb)
            .or_else(|| capabilities.formats.first().copied())
            .ok_or(RenderError::NoSurfaceFormat)?;
        let present_mode = if capabilities
            .present_modes
            .contains(&wgpu::PresentMode::Fifo)
        {
            wgpu::PresentMode::Fifo
        } else {
            capabilities.present_modes[0]
        };
        let alpha_mode = capabilities.alpha_modes[0];
        let config = wgpu_surface_config(surface_format, present_mode, alpha_mode, size);

        if !size.is_zero() {
            surface.configure(&device, &config);
        }

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("seishin sprite shader"),
            source: wgpu::ShaderSource::Wgsl(SPRITE_SHADER.into()),
        });
        let texture_bind_group_layout = create_texture_bind_group_layout(&device);
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("seishin sprite pipeline layout"),
            bind_group_layouts: &[&texture_bind_group_layout],
            push_constant_ranges: &[],
        });
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("seishin sprite pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[SpriteVertex::layout()],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
        });
        let mut renderer = Self {
            surface,
            device,
            queue,
            config,
            render_pipeline,
            texture_bind_group_layout,
            textures: HashMap::new(),
            sprite_vertex_buffer: None,
            sprite_vertex_capacity: 0,
            size,
        };

        renderer.upload_texture(&TextureData::rgba8(TextureId::new(0), 1, 1, vec![255; 4])?)?;

        Ok(renderer)
    }

    pub fn resize(&mut self, size: RenderSize) {
        self.size = size;
        self.config.width = size.width.max(1);
        self.config.height = size.height.max(1);

        if !size.is_zero() {
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn render(&mut self, frame: RenderState<'_>) -> Result<(), RenderError> {
        if self.size.is_zero() {
            return Ok(());
        }

        self.upload_textures(frame.textures)?;

        let sprite_vertices = frame
            .sprites
            .iter()
            .flat_map(|sprite| sprite_vertices(*sprite, frame.camera, self.size))
            .collect::<Vec<_>>();
        self.write_sprite_vertices(&sprite_vertices);

        let surface_texture = match self.surface.get_current_texture() {
            Ok(texture) => texture,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            Err(wgpu::SurfaceError::Timeout) => return Ok(()),
            Err(wgpu::SurfaceError::OutOfMemory) => return Err(RenderError::SurfaceOutOfMemory),
        };

        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("seishin render encoder"),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("seishin render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(frame.clear_color.to_wgpu()),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            pass.set_pipeline(&self.render_pipeline);

            if let Some(sprite_vertex_buffer) = &self.sprite_vertex_buffer {
                pass.set_vertex_buffer(0, sprite_vertex_buffer.slice(..));
            }

            for (index, sprite) in frame.sprites.iter().enumerate() {
                let texture = self
                    .textures
                    .get(&sprite.texture_id)
                    .ok_or(RenderError::MissingTexture(sprite.texture_id))?;
                let vertex_start = (index * 6) as u32;
                let vertex_end = vertex_start + 6;

                pass.set_bind_group(0, &texture.bind_group, &[]);
                pass.draw(vertex_start..vertex_end, 0..1);
            }
        }

        self.queue.submit(Some(encoder.finish()));
        surface_texture.present();

        Ok(())
    }

    fn write_sprite_vertices(&mut self, vertices: &[SpriteVertex]) {
        if vertices.is_empty() {
            return;
        }

        if vertices.len() > self.sprite_vertex_capacity {
            self.sprite_vertex_capacity = vertices.len().next_power_of_two();
            self.sprite_vertex_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("seishin sprite vertex buffer"),
                size: (self.sprite_vertex_capacity * std::mem::size_of::<SpriteVertex>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
        }

        let Some(buffer) = &self.sprite_vertex_buffer else {
            return;
        };

        self.queue
            .write_buffer(buffer, 0, bytemuck::cast_slice(vertices));
    }

    fn upload_textures(&mut self, textures: &[TextureData]) -> Result<(), RenderError> {
        for texture in textures {
            self.upload_texture(texture)?;
        }

        Ok(())
    }

    fn upload_texture(&mut self, texture: &TextureData) -> Result<(), RenderError> {
        if self.textures.contains_key(&texture.id()) {
            return Ok(());
        }

        let size = wgpu::Extent3d {
            width: texture.width(),
            height: texture.height(),
            depth_or_array_layers: 1,
        };
        let gpu_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("seishin sprite texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &gpu_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            texture.pixels_rgba8(),
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(texture.width() * 4),
                rows_per_image: Some(texture.height()),
            },
            size,
        );

        let view = gpu_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("seishin sprite sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("seishin sprite bind group"),
            layout: &self.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        self.textures.insert(
            texture.id(),
            GpuTexture {
                _texture: gpu_texture,
                _view: view,
                _sampler: sampler,
                bind_group,
            },
        );

        Ok(())
    }
}

#[derive(Debug)]
struct GpuTexture {
    _texture: wgpu::Texture,
    _view: wgpu::TextureView,
    _sampler: wgpu::Sampler,
    bind_group: wgpu::BindGroup,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct SpriteVertex {
    position: [f32; 2],
    uv: [f32; 2],
}

impl SpriteVertex {
    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<SpriteVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 0,
                    shader_location: 0,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: std::mem::size_of::<[f32; 2]>() as u64,
                    shader_location: 1,
                },
            ],
        }
    }
}

fn create_texture_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("seishin texture bind group layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}

#[cfg(target_arch = "wasm32")]
fn platform_backends() -> wgpu::Backends {
    wgpu::Backends::GL
}

#[cfg(not(target_arch = "wasm32"))]
fn platform_backends() -> wgpu::Backends {
    wgpu::Backends::PRIMARY
}

#[cfg(target_arch = "wasm32")]
fn platform_limits() -> wgpu::Limits {
    wgpu::Limits::downlevel_webgl2_defaults()
}

#[cfg(not(target_arch = "wasm32"))]
fn platform_limits() -> wgpu::Limits {
    wgpu::Limits::downlevel_defaults()
}

fn wgpu_surface_config(
    format: wgpu::TextureFormat,
    present_mode: wgpu::PresentMode,
    alpha_mode: wgpu::CompositeAlphaMode,
    size: RenderSize,
) -> wgpu::SurfaceConfiguration {
    wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format,
        width: size.width.max(1),
        height: size.height.max(1),
        present_mode,
        desired_maximum_frame_latency: 2,
        alpha_mode,
        view_formats: vec![],
    }
}

fn sprite_vertices(sprite: Sprite, camera: Camera2D, viewport: RenderSize) -> [SpriteVertex; 6] {
    let [top_left, top_right, bottom_right, bottom_left] = sprite.corners();
    let top_left = camera.world_to_ndc(top_left.0, top_left.1, viewport);
    let top_right = camera.world_to_ndc(top_right.0, top_right.1, viewport);
    let bottom_right = camera.world_to_ndc(bottom_right.0, bottom_right.1, viewport);
    let bottom_left = camera.world_to_ndc(bottom_left.0, bottom_left.1, viewport);

    [
        SpriteVertex {
            position: top_left,
            uv: [0.0, 0.0],
        },
        SpriteVertex {
            position: top_right,
            uv: [1.0, 0.0],
        },
        SpriteVertex {
            position: bottom_right,
            uv: [1.0, 1.0],
        },
        SpriteVertex {
            position: top_left,
            uv: [0.0, 0.0],
        },
        SpriteVertex {
            position: bottom_right,
            uv: [1.0, 1.0],
        },
        SpriteVertex {
            position: bottom_left,
            uv: [0.0, 1.0],
        },
    ]
}

const SPRITE_SHADER: &str = r#"
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0)
var sprite_texture: texture_2d<f32>;

@group(0) @binding(1)
var sprite_sampler: sampler;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = vec4<f32>(input.position, 0.0, 1.0);
    output.uv = input.uv;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(sprite_texture, sprite_sampler, input.uv);
}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use seishin_core::Transform2D;

    #[test]
    fn sprite_vertices_reflect_camera_projection() {
        let sprite = Sprite {
            texture_id: TextureId::new(1),
            transform: Transform2D::from_translation(0.0, 0.0),
            width: 100.0,
            height: 50.0,
        };

        let vertices = sprite_vertices(sprite, Camera2D::default(), RenderSize::new(200, 100));

        assert_eq!(vertices[0].position, [-0.5, 0.5]);
        assert_eq!(vertices[2].position, [0.5, -0.5]);
    }
}
