use anyhow::Result;
use bytemuck;
use wgpu;
use wgpu::util::DeviceExt;
use winit;
use winit::event;
use winit::event::WindowEvent;
use cgmath::Vector2;
use std::time::Duration;
use lerp::Lerp;
use super::complex::Complex;


#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct CameraState {
    /// Width of the viewport, in pixels
    width: f32,

    /// Height of the viewport, in pixels
    height: f32,

    /// The viewport center (x = w/2, y = h/2) corresponds to this
    /// origin point, on the complex plane
    origin: Complex,

    /// The initial scale; used to ensure the fractal fits completely
    /// in the default viewport after opening
    scale: f32,

    /// The zoom level;
    zoom: f32,

    /// The complex number corresponding to 0,0 in viewport coordinates
    /// These numbers are used to perform the "zooming"; the closer min
    /// and max are to 0, the higher the zoom level
    min: Complex,

    /// The complex number corresponding to w/h in viewport coordinates
    /// See [`Self::min`]
    max: Complex,

    /// Has the Camera changed at all, and requires a from-scratch redraw?
    needs_redraw: u32,
}

#[derive(Debug)]
pub struct Camera {
    /// Data associated with the camera
    state: CameraState,

    /// WGPU objects
    buffer: wgpu::Buffer,
    layout: wgpu::BindGroupLayout,
    group: wgpu::BindGroup,


    cursor_pos: Vector2<f64>,
    mouse_left_down: bool,
    grab_pos: Vector2<f64>,
    grab_point: Complex,
}

impl CameraState {
    pub fn new(width: f32, height: f32, scale: f32, origin: Complex) -> Self {
        let zoom = 1.0;
        let (min, max) = Self::calculate_limits(width, height, scale, &origin, zoom);

        Self {
            width,
            height,
            origin,
            scale,
            zoom,
            min,
            max,
            needs_redraw: 1,
        }
    }
    
    fn resize(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;
        self.update_limits();
        self.redraw();
    }

    fn set_origin(&mut self, origin: Complex){
        //! Manually set the origin position. This corresponds with the center
        //! of the viewport (screen)

        self.origin = origin;
        self.update_limits();
        self.redraw();
    }

    fn move_origin(&mut self, pixel_x: f32, pixel_y: f32) {
        let new_origin = Complex::new(
            self.origin.re + (self.min.re - self.max.re) / self.width * pixel_x,
            self.origin.im + (self.min.im - self.max.im) / self.height * pixel_y,
        );
        self.set_origin(new_origin);
    }

    fn set_zoom(&mut self, zoom: f32) {
        //! Manually set the zoom level. Note that this *only* overrides
        //! the zoom float, it does not perform any centering logic. So,
        //! this effectively zooms around the center of the screen
        //!
        //! See [Self::zoom_at] to zoom around a specific pixel

        self.zoom = zoom;
        self.update_limits();
        self.redraw();
    }

    fn zoom_at_point(&mut self, x: f32, y: f32, zoom_by: f32) {
        //! Zoom in by @zoom_by, around the pixel coordinates (@x, @y)
        //! 
        //! This differs from [Self::set_zoom] in that you can specify
        //! a zoom origin, and the function will attempt to keep that
        //! point on the screen stationary

        if zoom_by > 0.0 {
            self.zoom *= 2.0;
        }
        else if zoom_by < 0.0 {
            self.zoom /= 2.0;
        }
        self.update_limits();
        self.redraw();

        //TODO: zoom such that (x, y)'s associated complex nums do not change
        // for now, this is just zooming around `self.origin`
        //let point = self.pixel_to_point(x, y);
    }

    fn zoom_rect(&mut self, x: f32, y: f32, w: f32, h: f32) {

        self.redraw();
    }

    fn redraw(&mut self) {
        self.needs_redraw = 1;
    }

    fn update_limits(&mut self) {
        //! Internal function, update the `min`/`max` points based on viewport
        //! width/height, the origin, and the current zoom level
        //! 
        //! Note that internally, this function uses [Self::calculate_limits]

        (self.min, self.max) = Self::calculate_limits(self.width, self.height, self.scale, &self.origin, self.zoom);
    }

    fn calculate_limits(width: f32, height: f32, scale: f32, origin: &Complex, zoom: f32) -> (Complex, Complex) {
        //! Calculate new `min` and `max` points based on @width/@height,
        //! @scale and @zoom, and @origin.

        let ratio_x = if width > height { 1.0 } else { height / width };
        let ratio_y = if width > height { width / height } else { 1.0 };
        let min_x = -scale / 2.0 / ratio_x;
        let max_x =  scale / 2.0 / ratio_x;
        let min_y = -scale / 2.0 / ratio_y;
        let max_y =  scale / 2.0 / ratio_y;

        let min = Complex::new(
            (min_x / zoom) + origin.re,
            (min_y / zoom) + origin.im,
        );
        let max = Complex::new(
            (max_x / zoom) + origin.re,
            (max_y / zoom) + origin.im,
        );

        (min, max)
    }
}

impl Camera {
    pub fn new(device: &wgpu::Device, width: f32, height: f32, scale: f32, origin: Complex) -> Result<Self> {
        let state = CameraState::new(width, height, scale, origin);
        let buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("camera_buffer"),
                contents: bytemuck::cast_slice(&[state]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ],
            label: Some("camera_bind_group_layout"),
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }
            ],
            label: Some("camera_bind_group"),
        });

        let cursor_pos = Vector2::new(0.0, 0.0);
        let grab_pos = Vector2::new(0.0, 0.0);
        let grab_point = Complex::new(0.0, 0.0);

        Ok(Self {
            state,
            buffer,
            layout,
            group,
            cursor_pos,
            mouse_left_down: false,
            grab_pos,
            grab_point,
        })
    }

    pub fn input(&mut self, window: &winit::window::Window, event: &winit::event::WindowEvent) -> bool {
        match event {
            WindowEvent::MouseWheel { delta, .. } => {
                match delta {
                    event::MouseScrollDelta::LineDelta(_horizontal, vertical) => {
                        self.zoom_at_point(self.state.width / 2.0, self.state.height / 2.0, *vertical);
                        true
                    },
                    _ => false
                }
                
            },
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_pos.x = position.x;
                self.cursor_pos.y = position.y;
                true
            },
            WindowEvent::MouseInput { state, button, .. } => {
                match (button, state) {
                    (event::MouseButton::Left, event::ElementState::Released) => {
                        self.mouse_left_down = false;
                        // window.set_cursor_icon(winit::window::CursorIcon::Default)
                        true
                    },
                    (event::MouseButton::Left, event::ElementState::Pressed) => {
                        if !self.mouse_left_down {
                            self.mouse_left_down = true;
                            self.grab_pos = self.cursor_pos.clone();
                            self.grab_point = self.pixel_to_point(self.grab_pos.x as f32, self.grab_pos.y as f32);

                        }
                        // window.set_cursor_icon(winit::window::CursorIcon::Hand)
                        true
                    },
                    _ => false
                }
            }
            _ => false,
        }
    }

    pub fn update(&mut self, dt: &Duration, queue: &wgpu::Queue) {
        if self.mouse_left_down {
            let grab_complex = self.pixel_to_point(self.cursor_pos.x as f32, self.cursor_pos.y as f32);
            self.state.set_origin(self.grab_point + self.state.origin - grab_complex);
        }
        if self.state.needs_redraw != 0 {
            let a = self.pixel_to_point(100.0, 100.0);
            let b = self.pixel_to_point(101.0, 100.0);
            println!("{:?} x {:?}", a, b);
            queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.state]));
            self.state.needs_redraw = 0;
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.state.resize(width as f32, height as f32);
    }

    pub fn set_origin(&mut self, origin: Complex){
        //! Manually set the origin position. This corresponds with the center
        //! of the viewport (screen)

        self.state.set_origin(origin);
    }

    pub fn move_origin(&mut self, pixel_x: f32, pixel_y: f32) {
        
        self.state.move_origin(pixel_x, pixel_y);
    }

    pub fn set_zoom(&mut self, zoom: f32) {
        //! Manually set the zoom level. Note that this *only* overrides
        //! the zoom float, it does not perform any centering logic. So,
        //! this effectively zooms around the center of the screen
        //!
        //! See [Self::zoom_at] to zoom around a specific pixel

        self.state.set_zoom(zoom);
    }

    pub fn zoom_at_point(&mut self, x: f32, y: f32, zoom_by: f32) {
        //! Zoom in by @zoom_by, around the pixel coordinates (@x, @y)
        //! 
        //! This differs from [Self::set_zoom] in that you can specify
        //! a zoom origin, and the function will attempt to keep that
        //! point on the screen stationary

        self.state.zoom_at_point(x, y, zoom_by);
    }

    pub fn zoom_rect(&mut self, x: f32, y: f32, w: f32, h: f32) {

        self.state.zoom_rect(x, y, w, h);
    }

    pub fn get_layout(&self) -> &wgpu::BindGroupLayout {
        &self.layout
    }

    pub fn get_bind_group(&self) -> &wgpu::BindGroup {
        &self.group
    }

    fn pixel_to_point(&self, x: f32, y: f32) -> Complex {
        let w = self.state.max.re - self.state.min.re;
        let h = self.state.min.im - self.state.max.im;
        Complex {
            re: self.state.min.re + x * w / self.state.width,
            im: self.state.min.im - y * h / self.state.height,
        }
    }
}
