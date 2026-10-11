//! Fixed CRT-Lottes pass using WGSL generated from the unchanged upstream shader.
//! Translation tools are development-only; the shipped app needs only wgpu.
use wgpu::util::DeviceExt;
pub struct Lottes {
    pipeline: wgpu::RenderPipeline,
    globals: wgpu::Buffer,
    parameters: wgpu::Buffer,
    sampler: wgpu::BindGroup,
}
impl Lottes {
    pub fn new(device: &wgpu::Device) -> Self {
        let vertex = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Lottes triangle"),
            source: wgpu::ShaderSource::Wgsl(include_str!("present.wgsl").into()),
        });
        let fragment = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("upstream Lottes, translated at development time"),
            source: wgpu::ShaderSource::Wgsl(include_str!("lottes.frag.wgsl").into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("CRT-Lottes"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &vertex,
                entry_point: Some("vertex"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &fragment,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        let globals = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Lottes sizes"),
            size: 112,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        // Upstream defaults, in reflected Push member order; padded to 64 bytes.
        let values: [f32; 16] = [
            -8.0, -3.0, 0.031, 0.041, 0.5, 1.5, 1.0, 3.0, 1.0, -2.0, -1.5, 0.15, 2.0, 0.0, 0.0, 0.0,
        ];
        let bytes: Vec<u8> = values.iter().flat_map(|v| v.to_ne_bytes()).collect();
        let parameters = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Lottes defaults"),
            contents: &bytes,
            usage: wgpu::BufferUsages::UNIFORM,
        });
        // Matches librashader's nearest sampling / clamp-to-edge fallback on our device.
        let sample = device.create_sampler(&wgpu::SamplerDescriptor::default());
        let sampler = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &pipeline.get_bind_group_layout(1),
            entries: &[wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&sample),
            }],
        });
        Self {
            pipeline,
            globals,
            parameters,
            sampler,
        }
    }
    pub fn render(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        input: &wgpu::Texture,
        output: &wgpu::Texture,
    ) {
        let mut values = vec![0.0f32; 16];
        for (w, h) in [
            (output.width(), output.height()),
            (input.width(), input.height()),
            (input.width(), input.height()),
        ] {
            values.extend([w as f32, h as f32, 1.0 / w as f32, 1.0 / h as f32]);
        }
        let bytes: Vec<u8> = values.iter().flat_map(|v| v.to_ne_bytes()).collect();
        queue.write_buffer(&self.globals, 0, &bytes);
        let input_view = input.create_view(&Default::default());
        let binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.globals.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&input_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.parameters.as_entire_binding(),
                },
            ],
        });
        let view = output.create_view(&Default::default());
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Lottes"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &binding, &[]);
        pass.set_bind_group(1, &self.sampler, &[]);
        pass.draw(0..3, 0..1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // GPU integration test: retain the old engine only as a development reference.
    #[test]
    #[ignore = "requires a GPU; run explicitly when changing renderer or generated shader"]
    fn matches_upstream_lottes() {
        use librashader::{
            presets::ShaderFeatures,
            runtime::{
                Viewport,
                wgpu::{FilterChain, WgpuOutputView},
            },
        };
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        let (device, queue) =
            pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let direct = Lottes::new(&device);
        let mut reference = FilterChain::load_from_path(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../assets/shaders/crt/crt-lottes.slangp"),
            ShaderFeatures::empty(),
            &device,
            &queue,
            None,
        )
        .unwrap();
        let texture = |w, h| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_DST
                    | wgpu::TextureUsages::COPY_SRC
                    | wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
        };
        // Native SNES, high-resolution SNES and an uneven window size.
        for (iw, ih, ow, oh) in [
            (256, 224, 768, 672),
            (512, 448, 1024, 896),
            (256, 239, 853, 641),
        ] {
            let input = texture(iw, ih);
            let actual = texture(ow, oh);
            let expected = texture(ow, oh);
            let data: Vec<u8> = (0..ih)
                .flat_map(|y| {
                    (0..iw).flat_map(move |x| {
                        [
                            ((x * 13 + y * 7) % 256) as u8,
                            ((x / 8 * 43) % 256) as u8,
                            ((y / 7 * 31) % 256) as u8,
                            255,
                        ]
                    })
                })
                .collect();
            queue.write_texture(
                input.as_image_copy(),
                &data,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(iw * 4),
                    rows_per_image: None,
                },
                input.size(),
            );
            let mut encoder = device.create_command_encoder(&Default::default());
            direct.render(&device, &queue, &mut encoder, &input, &actual);
            let view =
                Viewport::new_render_target_sized_origin(WgpuOutputView::from(&expected), None)
                    .unwrap();
            reference
                .frame(&input, &view, &mut encoder, 0, None)
                .unwrap();
            let pitch = (ow * 4).div_ceil(256) * 256;
            let buffers: Vec<_> = [&actual, &expected]
                .iter()
                .map(|tex| {
                    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                        label: None,
                        size: (pitch * oh) as u64,
                        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    });
                    encoder.copy_texture_to_buffer(
                        tex.as_image_copy(),
                        wgpu::TexelCopyBufferInfo {
                            buffer: &buffer,
                            layout: wgpu::TexelCopyBufferLayout {
                                offset: 0,
                                bytes_per_row: Some(pitch),
                                rows_per_image: None,
                            },
                        },
                        tex.size(),
                    );
                    buffer
                })
                .collect();
            queue.submit([encoder.finish()]);
            for b in &buffers {
                b.slice(..).map_async(wgpu::MapMode::Read, |r| r.unwrap());
            }
            device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
            let a = buffers[0].slice(..).get_mapped_range().unwrap();
            let b = buffers[1].slice(..).get_mapped_range().unwrap();
            let mut total = 0u64;
            let mut max = 0u8;
            let mut changed = 0;
            for y in 0..oh as usize {
                for x in 0..(ow * 4) as usize {
                    let i = y * pitch as usize + x;
                    let d = a[i].abs_diff(b[i]);
                    total += d as u64;
                    max = max.max(d);
                    changed += usize::from(d != 0);
                }
            }
            let mean = total as f64 / (ow * oh * 4) as f64;
            println!(
                "{iw}x{ih} -> {ow}x{oh}: max channel difference {max}, mean {mean:.6}, changed {changed}"
            );
            // Different triangle interpolation can round adjacent UNORM levels.
            assert!(mean < 0.001 && max <= 1, "shader differs from upstream");
        }
    }
}
