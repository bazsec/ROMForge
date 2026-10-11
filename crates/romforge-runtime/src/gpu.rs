//! GPU presentation and the unmodified upstream Libretro CRT-Lottes preset.
use crate::lottes::Lottes;
use crate::{menu::Menu, player::destination};
use anyhow::{Context, Result, bail};
use sdl3::{
    pixels::{Color, PixelFormat},
    render::Canvas,
    surface::Surface,
    video::Window,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Shader {
    #[default]
    Off,
    #[serde(alias = "Guest", alias = "Royale")]
    Lottes,
}
impl Shader {
    pub const ALL: [Self; 2] = [Self::Off, Self::Lottes];
    pub fn name(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Lottes => "CRT-Lottes",
        }
    }
}
struct Image {
    texture: wgpu::Texture,
    binding: wgpu::BindGroup,
}
pub struct Renderer {
    // Surface must drop before the SDL window whose native handles it borrows.
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    blit: wgpu::RenderPipeline,
    overlay: wgpu::RenderPipeline,
    input: Option<Image>,
    filtered: Option<Image>,
    menu_image: Option<Image>,
    menu_key: String,
    lottes: Lottes,
    pub active: Shader,
    pub notice: String,
    shader_dirty: bool,
    window: Window,
}
impl Renderer {
    pub fn new(window: Window) -> Result<Self> {
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        // SAFETY: Renderer owns the window, stays on the SDL main thread, and drops
        // its surface first. Native handles remain valid for the surface's lifetime.
        let surface = unsafe {
            instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::from_display_and_window(
                &window, &window,
            )?)
        }?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        }))?;
        println!(
            "GPU: {} ({:?})",
            adapter.get_info().name,
            adapter.get_info().backend
        );
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("ROMForge GPU"),
                required_limits: wgpu::Limits::default(),
                required_features: wgpu::Features::empty(),
                ..Default::default()
            }))?;
        let caps = surface.get_capabilities(&adapter);
        // CRT presets perform their own gamma encoding. Avoid a second sRGB conversion.
        let format = *caps
            .formats
            .iter()
            .find(|f| !f.is_srgb())
            .context("GPU lacks an SDR output format")?;
        let (w, h) = window.size_in_pixels();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: w.max(1),
            height: h.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
            color_space: wgpu::SurfaceColorSpace::Auto,
        };
        surface.configure(&device, &config);
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("image layout"),
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
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("pixel sampler"),
            ..Default::default()
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("pixel blit"),
            source: wgpu::ShaderSource::Wgsl(include_str!("present.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let make_pipeline = |blend| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("present"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("vertex"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some("fragment"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                multiview_mask: None,
                cache: None,
            })
        };
        let blit = make_pipeline(None);
        let overlay = make_pipeline(Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING));
        let lottes = Lottes::new(&device);
        Ok(Self {
            surface,
            device,
            queue,
            config,
            layout,
            sampler,
            blit,
            overlay,
            input: None,
            filtered: None,
            menu_image: None,
            menu_key: String::new(),
            lottes,
            active: Shader::Off,
            notice: String::new(),
            shader_dirty: true,
            window,
        })
    }
    pub fn reset_history(&mut self) {
        self.shader_dirty = true;
    }
    pub fn window_mut(&mut self) -> &mut Window {
        &mut self.window
    }
    pub fn select(&mut self, shader: Shader) {
        self.active = shader;
        self.shader_dirty = true;
        self.notice = format!("{} ready", shader.name());
    }
    fn image(&self, w: u32, h: u32) -> Image {
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("game image"),
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
        });
        let binding = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        &texture.create_view(&Default::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        Image { texture, binding }
    }
    fn upload(&self, image: &Image, bytes: &[u8], pitch: u32) {
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &image.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(pitch),
                rows_per_image: None,
            },
            image.texture.size(),
        );
    }
    pub fn render(
        &mut self,
        rgba: &[u8],
        width: u32,
        height: u32,
        aspect: f32,
        advanced: bool,
        menu: Option<(&Menu, &[String], &str, &str)>,
    ) -> Result<bool> {
        let (w, h) = self.window.size_in_pixels();
        if w == 0 || h == 0 || width == 0 || height == 0 {
            return Ok(false);
        }
        let resized = (self.config.width, self.config.height) != (w, h);
        if resized {
            self.config.width = w;
            self.config.height = h;
            self.surface.configure(&self.device, &self.config);
            self.menu_key.clear();
        }
        if self
            .input
            .as_ref()
            .is_none_or(|i| (i.texture.width(), i.texture.height()) != (width, height))
        {
            self.input = Some(self.image(width, height));
            self.shader_dirty = true;
        }
        let input = self.input.as_ref().unwrap();
        self.upload(input, rgba, width * 4);
        let dest = destination(w, h, aspect);
        if self.filtered.as_ref().is_none_or(|i| {
            (i.texture.width(), i.texture.height()) != (dest.width(), dest.height())
        }) {
            self.filtered = Some(self.image(dest.width(), dest.height()));
            self.shader_dirty = true;
        }
        if let Some((menu, rows, footer, status)) = menu {
            let key = format!(
                "{:?}:{:?}:{}:{}:{footer}:{status}",
                menu.view, rows, menu.selected, menu.message
            );
            if key != self.menu_key {
                let scale = ((w / 480).min(h / 340)).clamp(1, 4);
                let (mw, mh) = (w / scale, h / scale);
                let mut canvas = Canvas::from_surface(Surface::new(mw, mh, PixelFormat::RGBA32)?)?;
                canvas.set_draw_color(Color::RGBA(0, 0, 0, 0));
                canvas.clear();
                menu.draw(&mut canvas, rows, footer, status)?;
                canvas.present();
                let image = self.image(mw, mh);
                canvas
                    .surface()
                    .with_lock(|bytes| self.upload(&image, bytes, canvas.surface().pitch()));
                self.menu_image = Some(image);
                self.menu_key = key;
            }
        } else {
            self.menu_key.clear();
        }
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(false);
            }
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.config);
                return Ok(false);
            }
            error => bail!("GPU presentation failed: {error:?}"),
        };
        let mut encoder = self.device.create_command_encoder(&Default::default());
        if self.active == Shader::Lottes && (advanced || self.shader_dirty) {
            self.lottes.render(
                &self.device,
                &self.queue,
                &mut encoder,
                &self.input.as_ref().unwrap().texture,
                &self.filtered.as_ref().unwrap().texture,
            );
            self.shader_dirty = false;
        }
        let output = frame.texture.create_view(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("game and menu"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &output,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(&self.blit);
            pass.set_viewport(
                dest.x() as f32,
                dest.y() as f32,
                dest.width() as f32,
                dest.height() as f32,
                0.0,
                1.0,
            );
            let image = if self.active == Shader::Lottes {
                self.filtered.as_ref().unwrap()
            } else {
                self.input.as_ref().unwrap()
            };
            pass.set_bind_group(0, &image.binding, &[]);
            pass.draw(0..3, 0..1);
            if menu.is_some() {
                pass.set_pipeline(&self.overlay);
                pass.set_viewport(0.0, 0.0, w as f32, h as f32, 0.0, 1.0);
                pass.set_bind_group(0, &self.menu_image.as_ref().unwrap().binding, &[]);
                pass.draw(0..3, 0..1);
            }
        }
        self.queue.submit([encoder.finish()]);
        self.queue.present(frame);
        Ok(true)
    }
}
