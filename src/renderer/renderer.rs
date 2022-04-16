use anyhow::Result;
use bytemuck;
use winit;
use wgpu;
use wgpu::util::DeviceExt;
use std::time::Duration;
use super::Shader;
use super::Camera;
use super::Vertex;
use super::Complex;

// A rect that covers the entire screen space (-1,-1 to 1,1)

pub struct Renderer {
    instance: wgpu::Instance,
    surface: wgpu::Surface,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,

    pipeline: wgpu::RenderPipeline,

    shader: Shader,
    camera: Camera,
    vertex_buffer: wgpu::Buffer,
}

impl Renderer {
    pub const DEFAULT_CAMERA_SCALE: f32 = 3.0;
    pub const DEFAULT_CAMERA_ORIGIN: Complex = Complex { re: -0.5, im: 0.0 };
    pub const DEFAULT_SHADER: &'static str = "./shaders/mandelbrot.wgsl";
    const VERTICES: [Vertex; 6] = [
        Vertex { position: [-1.0, -1.0, 0.0] },
        Vertex { position: [ 1.0, -1.0, 0.0] },
        Vertex { position: [-1.0,  1.0, 0.0] },
    
        Vertex { position: [-1.0,  1.0, 0.0] },
        Vertex { position: [ 1.0, -1.0, 0.0] },
        Vertex { position: [ 1.0,  1.0, 0.0] },
    ];

    pub async fn new(window: &winit::window::Window, scale: Option<f32>, origin: Option<Complex>) -> Result<Self> {
        let scale = scale.unwrap_or(Self::DEFAULT_CAMERA_SCALE);
        let origin = origin.unwrap_or(Self::DEFAULT_CAMERA_ORIGIN);
        let shader_path = Self::DEFAULT_SHADER;
        let size = window.inner_size();

        let (
            instance,
            surface,
            adapter,
            device,
            queue,
            config,
        ) = Self::init_device(&window).await?;

        let shader = Shader::new(&device, shader_path)?;
        let camera = Camera::new(&device, size.width as f32, size.height as f32, scale, origin)?;
        let vertex_buffer = Self::init_vertex_buffer(&device)?;
        let render_pipeline = Self::init_pipeline(&device, &config, &shader, &camera)?;

        Ok(Self {
            instance,
            surface,
            adapter,
            device,
            queue,
            config,

            pipeline: render_pipeline,

            shader,
            camera,
            vertex_buffer,
        })
    }

    pub fn render(&mut self) -> std::result::Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 1.0,
                            g: 1.0,
                            b: 1.0,
                            a: 1.0,
                        }),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, self.camera.get_bind_group(), &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.draw(0..Self::VERTICES.len() as u32, 0..1);
        }
    
        // submit will accept anything that implements IntoIter
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
    
    pub fn update(&mut self, dt: &Duration) -> Result<()> {
        self.camera.update(dt, &self.queue);

        Ok(())
    }

    pub fn input(&mut self, window: &winit::window::Window, event: &winit::event::WindowEvent) -> bool {
        let mut done: bool;

        done = self.camera.input(window, event);

        done
    }

    pub fn resize(&mut self, mut size: winit::dpi::PhysicalSize<u32>) {
        if size.width <= 0 {
            size.width = 1;
        }
        if size.height <= 0 {
            size.height = 1;
        }

        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);
        self.camera.resize(size.width, size.height);
    }

    async fn init_device(window: &winit::window::Window) -> Result<(wgpu::Instance, wgpu::Surface, wgpu::Adapter, wgpu::Device, wgpu::Queue, wgpu::SurfaceConfiguration)> {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::Backends::all());
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            },
        ).await.unwrap();

        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::empty(),
                // WebGL doesn't support all of wgpu's features, so if
                // we're building for the web we'll have to disable some.
                limits: if cfg!(target_arch = "wasm32") {
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    wgpu::Limits::default()
                },
                label: None,
            },
            None, // Trace path
        ).await.unwrap();

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_preferred_format(&adapter).unwrap(),
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };
        surface.configure(&device, &config);

        Ok((instance, surface, adapter, device, queue, config))
    }

    fn init_vertex_buffer(device: &wgpu::Device) -> Result<wgpu::Buffer> {
        let buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("vertex_buffer"),
                contents: bytemuck::cast_slice(&Self::VERTICES),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );

        Ok(buffer)
    }

    fn init_pipeline(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration, shader: &Shader, camera: &Camera) -> Result<wgpu::RenderPipeline> {
        let layout = device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: Some("render_pipline_layout"),
                bind_group_layouts: &[
                    camera.get_layout()
                ],
                push_constant_ranges: &[],
            }
        );
        let pipeline = device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                label: Some("render_pipeline"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: shader.get_module(),
                    entry_point: "vs_main",
                    buffers: &[
                        Vertex::desc(),
                    ],
                },
                fragment: Some(wgpu::FragmentState {
                    module: shader.get_module(),
                    entry_point: "fs_main",
                    targets: &[wgpu::ColorTargetState {
                        format: config.format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    }],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
            }
        );

        Ok(pipeline)
    }


}
