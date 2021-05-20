use imgui::im_str;
use std::{
    sync::{mpsc::Receiver, Arc, Mutex},
    time::Instant,
};
use winit::dpi::PhysicalSize;

mod gui;

use crate::program::{Constants, Program, Variable};

// Use a linear (not sRGB) format for the swapchain images  to allow shaders
// to work in linear color space without having to perform gamma correction
// before presenting.
//
// TODO: This might not be guaranteed to be supported. Add an explicit gamma
// correction pass and use Bgra8UnormSrgb instead?
pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8Unorm;

// Container for wgpu and imgui state and rendering logic.
pub struct Renderer {
    window: winit::window::Window,

    _instance: wgpu::Instance,
    _adapter: wgpu::Adapter,
    pub device: Arc<wgpu::Device>,
    queue: wgpu::Queue,

    size: PhysicalSize<u32>,
    surface: wgpu::Surface,
    swap_chain: wgpu::SwapChain,

    start: Instant,
    mouse_click: [f32; 2],

    program: Arc<Mutex<Option<Program>>>,

    imgui: imgui::Context,
    imgui_plaf: imgui_winit_support::WinitPlatform,
    imgui_renderer: imgui_wgpu::Renderer,
}

impl Renderer {
    pub fn new<T: 'static>(
        event_loop: &winit::event_loop::EventLoopWindowTarget<T>,
        rx: Receiver<Program>,
    ) -> Self {
        // Create the wgpu instance and request an adapter and device
        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: None,
        }))
        .expect("Failed to create graphics adapter!");
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("device"),
                features: wgpu::Features::PUSH_CONSTANTS,
                limits: wgpu::Limits {
                    max_push_constant_size: Constants::SIZE,
                    ..std::default::Default::default()
                },
            },
            None,
        ))
        .expect("Failed to create graphics device!");
        let device = Arc::new(device);

        // Set up the winit window and swapchain
        let window = winit::window::Window::new(event_loop).unwrap();
        let size = window.inner_size();
        let surface = unsafe { instance.create_surface(&window) };
        let swap_chain = Self::create_swap_chain(&device, &surface, size);

        // Spawn a thread to listen for and replace the shader program with newly compiled ones
        let program: Arc<Mutex<Option<Program>>> = Arc::new(Mutex::new(None));
        let self_program = Arc::clone(&program);
        std::thread::spawn(move || loop {
            let mut new = rx.recv().unwrap();
            let mut program = program.lock().unwrap();

            // Copy variables from the old if existing
            if let Some(old) = program.take() {
                new.initialize(&old);
            }

            // Swap in the new
            *program = Some(new);

            log::info!("Shader reloaded.");
        });

        // Set up imgui facilities.
        let mut imgui = imgui::Context::create();
        let mut imgui_plaf = imgui_winit_support::WinitPlatform::init(&mut imgui);
        imgui_plaf.attach_window(
            imgui.io_mut(),
            &window,
            imgui_winit_support::HiDpiMode::Default,
        );
        // Disable the default saving of config to an ini file.
        imgui.set_ini_filename(None);
        imgui
            .fonts()
            .add_font(&[imgui::FontSource::DefaultFontData {
                config: Some(imgui::FontConfig {
                    oversample_h: 1,
                    pixel_snap_h: true,
                    size_pixels: 12.0,
                    ..Default::default()
                }),
            }]);
        let imgui_renderer = imgui_wgpu::Renderer::new(
            &mut imgui,
            &*device,
            &queue,
            imgui_wgpu::RendererConfig {
                texture_format: FORMAT,
                ..Default::default()
            },
        );

        Self {
            window,

            _instance: instance,
            _adapter: adapter,
            device,
            queue,

            size,
            surface,
            swap_chain,

            start: Instant::now(),
            mouse_click: [size.width as f32 / 2.0, size.height as f32 / 2.0],

            program: self_program,

            imgui,
            imgui_plaf,
            imgui_renderer,
        }
    }

    pub fn update(&mut self) {
        if self.imgui.io().mouse_down[0] {
            self.mouse_click = self.imgui.io().mouse_pos;
        }

        self.window.request_redraw();
    }

    pub fn event<T>(&mut self, event: &winit::event::Event<T>) {
        self.imgui_plaf
            .handle_event(self.imgui.io_mut(), &self.window, event);
    }

    pub fn render(&mut self) {
        let frame = self
            .swap_chain
            .get_current_frame()
            .expect("Failed to acquire next swap chain image!")
            .output;

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("main"),
            });

        self.render_program(&self.queue, &frame.view, &mut encoder);
        self.render_gui(&frame.view, &mut encoder);

        self.queue.submit(Some(encoder.finish()));
    }

    fn render_program(
        &self,
        queue: &wgpu::Queue,
        target: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        if let Some(program) = self.program.lock().unwrap().as_mut() {
            // Update the program constants
            let (w, h) = (self.size.width as f32, self.size.height as f32);
            program.consts.t = self.start.elapsed().as_secs_f32();
            program.consts.resolution = [w, h];
            program.consts.aspect = w / h;
            program.consts.mpos = Self::transform(self.size, self.imgui.io().mouse_pos);
            program.consts.mclick = Self::transform(self.size, self.mouse_click);

            // Update the program uniforms
            // TODO: Don't need to do this every frame.
            for group in program.uniform_groups.values() {
                for uniform in group.uniforms.values() {
                    uniform.write(queue);
                }
            }

            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });
            pass.set_pipeline(&program.pipeline);

            // Bind each uniform group
            for (i, group) in &program.uniform_groups {
                pass.set_bind_group(*i, &group.bind_group, &[]);
            }

            // Copy the push constants
            pass.set_push_constants(
                wgpu::ShaderStage::all(),
                0,
                &[
                    // Time
                    program.consts.t.to_le_bytes(),
                    // Resolution, padded to 8 byte alignment
                    [0, 0, 0, 0],
                    program.consts.resolution[0].to_le_bytes(),
                    program.consts.resolution[1].to_le_bytes(),
                    // Aspect Ratio
                    program.consts.aspect.to_le_bytes(),
                    // Mouse Position, padded to 8 byte alignment
                    [0, 0, 0, 0],
                    program.consts.mpos[0].to_le_bytes(),
                    program.consts.mpos[1].to_le_bytes(),
                    // Click Position
                    program.consts.mclick[0].to_le_bytes(),
                    program.consts.mclick[1].to_le_bytes(),
                ]
                .concat(),
            );
            pass.draw(0..3, 0..1);
        }
    }

    fn render_gui(&mut self, target: &wgpu::TextureView, encoder: &mut wgpu::CommandEncoder) {
        self.imgui_plaf
            .prepare_frame(self.imgui.io_mut(), &self.window)
            .unwrap();
        let ui = self.imgui.frame();

        {
            // Display program constants.
            // Make copies since we can't borrow &self in the closure since the 
            // function already mutably borrows &mut self.
            let secs = self.start.elapsed().as_secs_f32();
            let size = self.size;
            let mpos = Self::transform(self.size, ui.io().mouse_pos);
            let mclick = Self::transform(self.size, self.mouse_click);
            imgui::Window::new(im_str!("Stats"))
                .position([50.0, 50.0], imgui::Condition::FirstUseEver)
                .size([200.0, 0.0], imgui::Condition::Always)
                .build(&ui, || {
                    ui.text(im_str!("Time: {:.2}s", secs));
                    ui.text(im_str!("FPS: {:.2}", ui.io().framerate));
                    ui.text(im_str!("Resolution: {}x{}", size.width, size.height));
                    ui.text(im_str!(
                        "Aspect Ratio: {:.2}",
                        size.width as f32 / size.height as f32
                    ));
                    ui.text(im_str!("Mouse Position: ({:+.03},{:+.03})", mpos[0], mpos[1]));
                    ui.text(im_str!(
                        "Click Position: ({:+.03},{:+.03})",
                        mclick[0],
                        mclick[1]
                    ));
                });

            // Display program uniforms if one is loaded
            if let Some(program) = self.program.lock().unwrap().as_mut() {
                let uniforms = program
                    .uniform_groups
                    .values_mut()
                    .flat_map(|g| g.uniforms.iter_mut());
                for (i, (_, uniform)) in uniforms.enumerate() {
                    imgui::Window::new(&im_str!("Uniform: {}", &uniform.name))
                        .position(
                            [50.0 + ((i + 1) as f32 * 225.0), 50.0],
                            imgui::Condition::FirstUseEver,
                        )
                        .size([200.0, 0.0], imgui::Condition::Always)
                        .build(&ui, || {
                            let n = uniform.vars.len();
                            for (j, (name, var)) in uniform.vars.iter_mut().enumerate() {
                                match var {
                                    Variable::Int(i) => gui::input_int(&ui, name, i),
                                    Variable::Float(f) => gui::input_float(&ui, name, f),
                                    Variable::Vec2(v) => gui::input_vec2(&ui, name, v),
                                    Variable::Vec3(v) => gui::input_vec3(&ui, name, v),
                                    Variable::Vec4(v) => gui::input_vec4(&ui, name, v),
                                }

                                if j != n - 1 {
                                    ui.separator();
                                }
                            }
                        });
                }
            }
        }

        // Encode the GUI draw calls
        self.imgui_plaf.prepare_render(&ui, &self.window);
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("gui"),
            color_attachments: &[wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
        });
        self.imgui_renderer
            .render(ui.render(), &self.queue, &*self.device, &mut pass)
            .unwrap();
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        self.size = size;
        self.swap_chain = Self::create_swap_chain(&*self.device, &self.surface, size);
    }

    fn create_swap_chain(
        device: &wgpu::Device,
        surface: &wgpu::Surface,
        size: PhysicalSize<u32>,
    ) -> wgpu::SwapChain {
        device.create_swap_chain(
            surface,
            &wgpu::SwapChainDescriptor {
                usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
                format: FORMAT,
                width: size.width,
                height: size.height,
                present_mode: wgpu::PresentMode::Mailbox,
            },
        )
    }

    // Transform a screen coodinate to a shader UV coordinate
    fn transform(size: PhysicalSize<u32>, pos: [f32; 2]) -> [f32; 2] {
        let (w, h) = (size.width as f32, size.height as f32);

        // Center UVs in [-0.5, 0.5]
        let mut x = (pos[0] / w) - 0.5;
        let mut y = (1.0 - (pos[1] / h)) - 0.5;

        // Extend UVs past 0.5 in the larger dimension
        let aspect = w / h;
        if aspect > 1.0 {
            x *= aspect;
        } else {
            y /= aspect;
        }

        [x, y]
    }
}
