// Ядро рендера на wgpu: спрайт-батчер (текстурированные квады + цвет), offscreen
// render target'и, чтение пикселей RT. Замена macroquad draw_*/Texture2D/RenderTarget.
//
// Архитектура: кадр собирается как список пассов на CPU (вершины/индексы/скиссоры),
// весь GPU-ворк записывается разом в end_frame. RenderPass никогда не хранится в
// структуре — никаких 'static трюков.
//
// Порт семантики macroquad:
//  - альфа-блендинг SrcAlpha/OneMinusSrcAlpha, не-premultiplied (как в miniquad);
//  - формат Bgra8Unorm (без sRGB-конверсии) — смешивание в том же пространстве,
//    что и в miniquad, цвета выглядят одинаково;
//  - фильтрация: Linear для текстур, загруженных на старте (макроквадовский
//    дефолт), Nearest для создаваемых в рантайме (set_default_filter_mode(Nearest)
//    в игровом цикле). НАПЛЫВЫ тайлов специально Linear — плавное угасание по
//    альфе, ради чего миграция и затевалась.
use crate::camera::Camera;
use bytemuck::{Pod, Zeroable};
use std::num::NonZeroU64;
use wgpu::util::DeviceExt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TexId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Screen,
    Rt(usize),
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Vertex {
    pub pos: [f32; 2],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    Linear,
    Nearest,
}

pub struct Tex {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub bind: wgpu::BindGroup,
    pub size: (f32, f32),
}

/// Offscreen render target (замена macroquad render_target()).
pub struct Rt {
    pub index: usize,
    pub size: (u32, u32),
}

struct RtSlot {
    tex: wgpu::Texture,
    view: wgpu::TextureView,
}

struct Run {
    tex: TexId,
    first: u32,
    count: u32,
    scissor: Option<[u32; 4]>,
}

struct Pass {
    target: Target,
    clear: Option<[f32; 4]>,
    camera: [f32; 4],
    cam_buf: wgpu::Buffer,
    cam_bind: wgpu::BindGroup,
    verts: Vec<Vertex>,
    idx: Vec<u32>,
    runs: Vec<Run>,
    scissor: Option<[u32; 4]>,
    draw_index_base: u32,
}

pub struct Gfx {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface<'static>,
    pub config: wgpu::SurfaceConfiguration,
    pub viewport: (f32, f32),
    textures: Vec<Tex>,
    rts: Vec<RtSlot>,
    tex_layout: wgpu::BindGroupLayout,
    cam_layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
    linear: wgpu::Sampler,
    nearest: wgpu::Sampler,
    passes: Vec<Pass>,
    current: Pass,
}

pub const WHITE: [f32; 4] = [1., 1., 1., 1.];
pub const BLACK: [f32; 4] = [0., 0., 0., 1.];

impl Gfx {

    pub fn textures_ref(&self, id: TexId) -> &Tex {
        &self.textures[id.0 as usize]
    }

    pub fn textures_count(&self) -> u32 {
        self.textures.len() as u32
    }
    pub fn new(window: &std::sync::Arc<winit::window::Window>) -> Gfx {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        // Arc<Window> -> Surface<'static>: владение окном уходит в surface.
        let surface = instance
            .create_surface(window.clone())
            .expect("surface creation failed");
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
            apply_limit_buckets: false,
        }))
        .expect("no suitable GPU adapter");
        let (device, queue) = pollster::block_on(
            adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("wgpu_ui device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::Off,
            }),
        )
        .expect("device creation failed");

        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| *f == wgpu::TextureFormat::Bgra8Unorm)
            .unwrap_or(caps.formats[0]);
        let size = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            color_space: wgpu::SurfaceColorSpace::Auto,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let tex_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("tex"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });
        let cam_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("cam"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: NonZeroU64::new(16),
                },
                count: None,
            }],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sprite"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/sprite.wgsl").into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sprite"),
            bind_group_layouts: &[Some(&cam_layout), Some(&tex_layout)],
            immediate_size: 0,
        });
        let vertex_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 0,
                },
                wgpu::VertexAttribute {
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 8,
                },
                wgpu::VertexAttribute {
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 16,
                },
            ],
        };
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sprite"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[Some(vertex_layout)],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let linear = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let nearest = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let initial = Self::new_pass(&device, &cam_layout, Target::Screen, Some(BLACK), [0.; 4]);
        let mut gfx = Gfx {
            device,
            queue,
            surface,
            config,
            viewport: (0., 0.),
            textures: Vec::new(),
            rts: Vec::new(),
            tex_layout,
            cam_layout,
            pipeline,
            linear,
            nearest,
            passes: Vec::new(),
            current: initial,
        };
        // Белая 1x1 — для draw_rect.
        gfx.push_texture_rgba(&[255, 255, 255, 255], 1, 1, Filter::Nearest);
        gfx
    }

    pub fn white(&self) -> TexId {
        TexId(0)
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.config.width = width.max(1);
        self.config.height = height.max(1);
        self.viewport = (self.config.width as f32, self.config.height as f32);
        self.surface.configure(&self.device, &self.config);
    }

    /// RGBA8 -> текстура.
    pub fn push_texture_rgba(&mut self, rgba: &[u8], w: u32, h: u32, filter: Filter) -> TexId {
        assert_eq!(rgba.len(), (w * h * 4) as usize, "texture size mismatch");
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d { width: w.max(1), height: h.max(1), depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(w.max(1) * 4),
                rows_per_image: Some(h.max(1)),
            },
            wgpu::Extent3d { width: w.max(1), height: h.max(1), depth_or_array_layers: 1 },
        );
        let id = self.make_tex(texture, filter);
        TexId(id)
    }

    fn make_tex(&mut self, texture: wgpu::Texture, filter: Filter) -> u32 {
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = match filter {
            Filter::Linear => &self.linear,
            Filter::Nearest => &self.nearest,
        };
        let bind = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.tex_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::Sampler(sampler) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&view) },
            ],
        });
        let id = self.textures.len() as u32;
        let size = (texture.width() as f32, texture.height() as f32);
        self.textures.push(Tex {
            texture,
            view,
            bind,
            size,
        });
        id
    }

    /// Кольцевое свечение (радиальный ореол вокруг карточки): прозрачный центр,
    /// гауссов пик на r=0.78, спад к краю. Юнит в центре остаётся видимым.
    pub fn push_glow_ring(&mut self, size: u32) -> TexId {
        let mut rgba = vec![0u8; (size * size * 4) as usize];
        let c = (size as f32 - 1.) / 2.;
        let radius = c.max(1.);
        for y in 0..size {
            for x in 0..size {
                let dx = x as f32 - c;
                let dy = y as f32 - c;
                let r = ((dx * dx + dy * dy).sqrt() / radius).clamp(0., 1.);
                let t = (r - 0.78) / 0.16;
                let a = (-t * t).exp();
                let i = ((y * size + x) * 4) as usize;
                rgba[i] = 255;
                rgba[i + 1] = 255;
                rgba[i + 2] = 255;
                rgba[i + 3] = (a * 255.) as u8;
            }
        }
        self.push_texture_rgba(&rgba, size, size, Filter::Linear)
    }

    /// Радиальный градиент (белый центр -> прозрачный край).
    pub fn push_radial_gradient(&mut self, size: u32, inner: [f32; 4], outer: [f32; 4]) -> TexId {
        let mut rgba = vec![0u8; (size * size * 4) as usize];
        let center = (size as f32 - 1.) / 2.;
        let radius = center.max(1.);
        for y in 0..size {
            for x in 0..size {
                let dx = x as f32 - center;
                let dy = y as f32 - center;
                let t = ((dx * dx + dy * dy).sqrt() / radius).clamp(0., 1.);
                let i = ((y * size + x) * 4) as usize;
                for c in 0..3 {
                    let v = (inner[c] + (outer[c] - inner[c]) * t).clamp(0., 1.);
                    rgba[i + c] = (v * 255.) as u8;
                }
                let a = (inner[3] + (outer[3] - inner[3]) * t).clamp(0., 1.);
                rgba[i + 3] = (a * 255.) as u8;
            }
        }
        self.push_texture_rgba(&rgba, size, size, Filter::Linear)
    }

    /// Декодирование png/jpeg через image crate (замена macroquad load_texture).
    pub fn push_texture_bytes(&mut self, bytes: &[u8], filter: Filter) -> TexId {
        let img = image::load_from_memory(bytes).expect("image decode failed");
        let img = img.to_rgba8();
        let (w, h) = img.dimensions();
        self.push_texture_rgba(img.as_raw(), w, h, filter)
    }

    pub fn tex_size(&self, id: TexId) -> (f32, f32) {
        self.textures[id.0 as usize].size
    }

    /// Offscreen RT; пиксельный формат как у экрана — один пайплайн на всё.
    pub fn create_rt(&mut self, w: u32, h: u32) -> Rt {
        let tex = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("rt"),
            size: wgpu::Extent3d { width: w.max(1), height: h.max(1), depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
        let index = self.rts.len();
        self.rts.push(RtSlot { tex, view });
        Rt { index, size: (w.max(1), h.max(1)) }
    }

    /// RT как рисуемая текстура (порт `target[0].texture.clone()`).
    pub fn rt_as_texture(&mut self, rt: &Rt, filter: Filter) -> TexId {
        let slot = &self.rts[rt.index];
        let sampler = match filter {
            Filter::Linear => &self.linear,
            Filter::Nearest => &self.nearest,
        };
        let bind = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.tex_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::Sampler(sampler) },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&slot.view),
                },
            ],
        });
        let id = self.textures.len() as u32;
        self.textures.push(Tex {
            texture: slot.tex.clone(),
            view: slot.view.clone(),
            bind,
            size: (rt.size.0 as f32, rt.size.1 as f32),
        });
        TexId(id)
    }

    pub fn begin_frame(&mut self) {
        self.passes.clear();
        self.current.verts.clear();
        self.current.idx.clear();
        self.current.runs.clear();
        self.current.draw_index_base = 0;
        self.current.scissor = None;
    }

    fn new_pass(
        device: &wgpu::Device,
        cam_layout: &wgpu::BindGroupLayout,
        target: Target,
        clear: Option<[f32; 4]>,
        camera: [f32; 4],
    ) -> Pass {
        let cam_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cam"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let cam_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("cam"),
            layout: cam_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: cam_buf.as_entire_binding(),
            }],
        });
        Pass {
            target,
            clear,
            camera,
            cam_buf,
            cam_bind,
            verts: Vec::new(),
            idx: Vec::new(),
            runs: Vec::new(),
            scissor: None,
            draw_index_base: 0,
        }
    }

    /// Начало пасса: target=Target::Screen — экран. Предыдущий аккумулированный
    /// пасс закрывается; весь GPU-ворк — в end_frame. clear=None — Load (дослоёная
    /// отрисовка поверх предыдущего пасса той же цели — смена камеры внутри кадра).
    pub fn begin_pass(&mut self, target: Target, clear: Option<[f32; 4]>, camera: &Camera) {
        let pass = Self::new_pass(&self.device, &self.cam_layout, target, clear, camera.gpu_uniform());
        let finished = std::mem::replace(&mut self.current, pass);
        self.passes.push(finished);
    }

    pub fn set_scissor(&mut self, x: i32, y: i32, w: u32, h: u32) {
        let cw = match self.current.target {
            Target::Screen => self.config.width,
            Target::Rt(i) => self.rts[i].tex.width(),
        };
        let ch = match self.current.target {
            Target::Screen => self.config.height,
            Target::Rt(i) => self.rts[i].tex.height(),
        };
        if w == 0 || h == 0 {
            self.current.scissor = Some([0, 0, 0, 0]);
            return;
        }
        let x0 = x.clamp(0, cw as i32) as u32;
        let y0 = y.clamp(0, ch as i32) as u32;
        self.current.scissor = Some([x0, y0, w.min(cw - x0), h.min(ch - y0)]);
    }
    fn push_quad(&mut self, tex: TexId, p: [[f32; 2]; 4], uv: [[f32; 2]; 4], color: [f32; 4]) {
        if self
            .current
            .runs
            .last()
            .map(|r| r.tex != tex || r.scissor != self.current.scissor)
            .unwrap_or(true)
        {
            self.current.runs.push(Run {
                tex,
                first: self.current.draw_index_base,
                count: 0,
                scissor: self.current.scissor,
            });
        }
        let base = self.current.verts.len() as u32;
        for (pos, uv) in p.into_iter().zip(uv.into_iter()) {
            self.current.verts.push(Vertex { pos, uv, color });
        }
        for i in [0u32, 1, 2, 0, 2, 3] {
            self.current.idx.push(base + i);
        }
        let last = self.current.runs.last_mut().unwrap();
        last.count += 6;
        self.current.draw_index_base += 6;
    }
    /// Порт draw_texture_ex: x, y — верх-лево, w/h — размер назначения.
    pub fn draw_texture(&mut self, tex: TexId, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) {
        let p = [[x, y], [x + w, y], [x + w, y + h], [x, y + h]];
        let uv = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
        self.push_quad(tex, p, uv, color);
    }

    /// Порт draw_rectangle: сплошной цветной прямоугольник.
    pub fn draw_rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) {
        self.draw_texture(self.white(), x, y, w, h, color);
    }

    /// Порт draw_rectangle_lines.
    pub fn draw_rect_lines(&mut self, x: f32, y: f32, w: f32, h: f32, t: f32, color: [f32; 4]) {
        self.draw_rect(x, y, w, t, color);
        self.draw_rect(x, y + h - t, w, t, color);
        self.draw_rect(x, y + t, t, h - t * 2., color);
        self.draw_rect(x + w - t, y + t, t, h - t * 2., color);
    }

    /// Квад с произвольным UV (глифы текста). p — 4 угла, uv — те же 4 угла.
    pub fn draw_quad(&mut self, tex: TexId, p: [[f32; 2]; 4], uv: [[f32; 2]; 4], color: [f32; 4]) {
        self.push_quad(tex, p, uv, color);
    }

    pub fn end_pass(&mut self) {
        // Пасс просто закрывается; следующий begin_pass откроет новый.
    }

    pub fn end_frame(&mut self) {
        let finished = std::mem::replace(
            &mut self.current,
            Self::new_pass(&self.device, &self.cam_layout, Target::Screen, Some(BLACK), [0.; 4]),
        );
        self.passes.push(finished);

        let mut frame: Option<wgpu::SurfaceTexture> = None;
        let mut encoder = self.device.create_command_encoder(&Default::default());
        for pass in self.passes.drain(..) {
            if pass.verts.is_empty() {
                continue;
            }
            let (view, is_screen) = match pass.target {
                Target::Rt(i) => (self.rts[i].view.clone(), false),
                Target::Screen => {
                    if frame.is_none() {
                        frame = Some(match self.surface.get_current_texture() {
                            wgpu::CurrentSurfaceTexture::Success(t) => t,
                            wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
                            wgpu::CurrentSurfaceTexture::Outdated => {
                                self.surface.configure(&self.device, &self.config);
                                return;
                            }
                            // Timeout/Occluded/Lost/Validation: кадр пропускается.
                            _ => return,
                        });
                    }
                    (frame.as_ref().unwrap().texture.create_view(&Default::default()), true)
                }
            };
            let vbuf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&pass.verts),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let ibuf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&pass.idx),
                usage: wgpu::BufferUsages::INDEX,
            });
            let cam_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("cam"),
                size: 16,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.queue.write_buffer(&cam_buf, 0, bytemuck::bytes_of(&pass.camera));
            let cam_bind = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("cam"),
                layout: &self.cam_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: cam_buf.as_entire_binding(),
                }],
            });
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: match pass.clear {
                            Some(c) => wgpu::LoadOp::Clear(wgpu::Color {
                                r: c[0] as f64,
                                g: c[1] as f64,
                                b: c[2] as f64,
                                a: c[3] as f64,
                            }),
                            None => wgpu::LoadOp::Load,
                        },
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            rpass.set_pipeline(&self.pipeline);
            rpass.set_bind_group(0, &cam_bind, &[]);
            rpass.set_vertex_buffer(0, vbuf.slice(..));
            rpass.set_index_buffer(ibuf.slice(..), wgpu::IndexFormat::Uint32);
            for run in &pass.runs {
                if run.count == 0 {
                    continue;
                }
                if let Some([x, y, w, h]) = run.scissor {
                    rpass.set_scissor_rect(x, y, w, h);
                }
                rpass.set_bind_group(1, &self.textures[run.tex.0 as usize].bind, &[]);
                rpass.draw_indexed(run.first..run.first + run.count, 0, 0..1);
            }
            drop(rpass);
            let _ = is_screen;
        }
        self.queue.submit([encoder.finish()]);
        if let Some(frame) = frame {
            self.queue.present(frame);
        }
    }

    /// Чтение пикселей RT — порт Texture2D::get_texture_data (RGBA8).
    pub fn read_rt(&self, rt: &Rt) -> Vec<u8> {
        let slot = &self.rts[rt.index];
        let (w, h) = (slot.tex.width(), slot.tex.height());
        let bytes_per_row = (w * 4).div_ceil(256) * 256;
        let buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: (bytes_per_row * h) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            slot.tex.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buf,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(h),
                },
            },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        );
        self.queue.submit([encoder.finish()]);
        let slice = buf.slice(..);
        let (tx, rx) = std::sync::mpsc::channel::<bool>();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            tx.send(r.is_ok()).ok();
        });
        loop {
            let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
            if rx.try_recv().is_ok() {
                break;
            }
        }
        let data = slice.get_mapped_range().expect("buffer map");
        let mut out = Vec::with_capacity((w * h * 4) as usize);
        for row in 0..h {
            let start = (row * bytes_per_row) as usize;
            out.extend_from_slice(&data[start..start + (w * 4) as usize]);
        }
        drop(data);
        buf.unmap();
        // Bgra8 -> Rgba8
        for px in out.chunks_exact_mut(4) {
            px.swap(0, 2);
        }
        out
    }
}

/// Порт macroquad Color::from_rgba.
pub fn rgba(r: u8, g: u8, b: u8, a: u8) -> [f32; 4] {
    [r as f32 / 255., g as f32 / 255., b as f32 / 255., a as f32 / 255.]
}

pub mod colors {
    pub const WHITE: [f32; 4] = [1., 1., 1., 1.];
    pub const BLACK: [f32; 4] = [0., 0., 0., 1.];
    pub const BLUE: [f32; 4] = [0., 0., 1., 1.];
    pub const GREEN: [f32; 4] = [0., 1., 0., 1.];
    pub const RED: [f32; 4] = [1., 0., 0., 1.];
    pub const VIOLET: [f32; 4] = [0.5, 0., 0.5, 1.];
    pub const SKYBLUE: [f32; 4] = [0.392, 0.584, 0.929, 1.];
    pub const DARKGRAY: [f32; 4] = [0.25, 0.25, 0.25, 1.];
    pub const DARKBLUE: [f32; 4] = [0., 0., 0.545, 1.];
    pub const DARKGREEN: [f32; 4] = [0., 0.392, 0., 1.];
    pub const ORANGE: [f32; 4] = [1., 0.625, 0., 1.];
}
