use conrod_core::render::Primitives;
use std::sync::Arc;
use vulkano::{
    command_buffer::AutoCommandBufferBuilder,
    format::D16Unorm,
    framebuffer::{Framebuffer, FramebufferAbstract, RenderPassAbstract},
    image::AttachmentImage,
    swapchain,
    swapchain::AcquireError,
    sync::FenceSignalFuture,
};

use crate::app::INITIAL_HEIGHT;
use crate::window::Window;
use conrod_core::image::Map;
use conrod_vulkano::Image;
use vulkano::command_buffer::{AutoCommandBuffer, CommandBufferExecFuture};
use vulkano::format::Format;
use vulkano::swapchain::{PresentFuture, SwapchainAcquireFuture};
use vulkano::sync::GpuFuture;

type DepthFormat = D16Unorm;

const DEPTH_FORMAT_TY: DepthFormat = D16Unorm;
const DEPTH_FORMAT: Format = Format::D16Unorm;
const CLEAR_COLOR: [f32; 4] = [0.2, 0.2, 0.2, 1.0];

pub struct Renderer {
    vulkano_render: conrod_vulkano::Renderer,
    pub window: Window,
    image_map: Map<Image>,
    render_target: RenderTarget,
    previous_frame_end: Option<
        FenceSignalFuture<
            PresentFuture<
                CommandBufferExecFuture<SwapchainAcquireFuture<winit::Window>, AutoCommandBuffer>,
                winit::Window,
            >,
        >,
    >,
}

impl Renderer {
    pub fn new(events_loop: &winit::EventsLoop, title: &str, width: u32) -> Self {
        let window = Window::new(width, title, events_loop);

        let render_target = RenderTarget::new(&window);

        let subpass = vulkano::framebuffer::Subpass::from(render_target.render_pass.clone(), 0)
            .expect("Couldn't create subpass for gui!");
        let queue = window.queue.clone();
        let vulkano_render = conrod_vulkano::Renderer::new(
            window.device.clone(),
            subpass,
            queue.family(),
            [width, INITIAL_HEIGHT],
            window.surface.window().get_hidpi_factor() as f64,
        )
        .unwrap();

        let image_map = conrod_core::image::Map::new();

        // Keep track of the previous frame so we can wait for it to complete before presenting a new
        // one. This should make sure the CPU never gets ahead of the presentation of frames, which can
        // cause high user-input latency and synchronisation strange bugs.
        let previous_frame_end: Option<FenceSignalFuture<_>> = None;

        Renderer {
            vulkano_render,
            window,
            image_map,
            render_target,
            previous_frame_end,
        }
    }

    pub fn render(&mut self, primitives: Primitives) {
        let (win_w, win_h) = match self.window.get_dimensions() {
            Some(s) => s,
            None => return,
        };

        let (image_num, acquire_future) =
            match swapchain::acquire_next_image(self.window.swapchain.clone(), None) {
                Ok(r) => r,
                Err(AcquireError::OutOfDate) => {
                    self.window.handle_resize();
                    self.render_target.handle_resize(&mut self.window);
                    return;
                }
                Err(err) => panic!("{:?}", err),
            };

        let mut command_buffer_builder = AutoCommandBufferBuilder::primary_one_time_submit(
            self.window.device.clone(),
            self.window.queue.family(),
        )
        .expect("Failed to create AutoCommandBufferBuilder");

        let viewport = [0.0, 0.0, win_w as f32, win_h as f32];
        let dpi_factor = self.window.surface.window().get_hidpi_factor() as f64;
        if let Some(cmd) = self
            .vulkano_render
            .fill(&self.image_map, viewport, dpi_factor, primitives)
            .unwrap()
        {
            let buffer = cmd
                .glyph_cpu_buffer_pool
                .chunk(cmd.glyph_cache_pixel_buffer.iter().cloned())
                .unwrap();
            command_buffer_builder = command_buffer_builder
                .copy_buffer_to_image(buffer, cmd.glyph_cache_texture)
                .expect("failed to submit command for caching glyph");
        }

        let mut command_buffer_builder = command_buffer_builder
            .begin_render_pass(
                self.render_target.framebuffers[image_num].clone(),
                false,
                vec![CLEAR_COLOR.into(), 1f32.into()],
            ) // Info: We need to clear background AND depth buffer here!
            .expect("Failed to begin render pass!");

        let draw_cmds = self
            .vulkano_render
            .draw(
                self.window.queue.clone(),
                &self.image_map,
                [0.0, 0.0, win_w as f32, win_h as f32],
            )
            .unwrap();
        for cmd in draw_cmds {
            let conrod_vulkano::DrawCommand {
                graphics_pipeline,
                dynamic_state,
                vertex_buffer,
                descriptor_set,
            } = cmd;
            command_buffer_builder = command_buffer_builder
                .draw(
                    graphics_pipeline,
                    &dynamic_state,
                    vec![vertex_buffer],
                    descriptor_set,
                    (),
                )
                .expect("failed to submit draw command");
        }

        let command_buffer = command_buffer_builder
            .end_render_pass()
            .unwrap()
            .build()
            .unwrap();

        // Wait for the previous frame to finish presentation.
        if let Some(prev_frame) = self.previous_frame_end.take() {
            prev_frame
                .wait(None)
                .expect("failed to wait for presentation of previous frame");
        }

        let future_result = acquire_future
            .then_execute(self.window.queue.clone(), command_buffer)
            .expect("failed to join previous frame with new one")
            .then_swapchain_present(
                self.window.queue.clone(),
                self.window.swapchain.clone(),
                image_num,
            )
            .then_signal_fence_and_flush();

        // Hold onto the future representing the presentation of this frame.
        // We'll wait for it before we present the next one.
        if let Ok(future) = future_result {
            self.previous_frame_end = Some(future);
        }
    }
}

pub struct RenderTarget {
    depth_buffer: Arc<AttachmentImage<D16Unorm>>,
    render_pass: Arc<dyn RenderPassAbstract + Send + Sync>,
    framebuffers: Vec<Arc<dyn FramebufferAbstract + Send + Sync>>,
}

impl RenderTarget {
    pub fn new(window: &Window) -> Self {
        let (win_w, win_h) = window
            .get_dimensions()
            .expect("couldn't get window dimensions");
        let win_dims = [win_w, win_h];
        let device = window.device.clone();
        let depth_buffer = AttachmentImage::transient(device, win_dims, DEPTH_FORMAT_TY).unwrap();

        let render_pass = Arc::new(
            vulkano::single_pass_renderpass!(window.device.clone(),
                attachments: {
                    color: {
                        load: Clear,
                        store: Store,
                        format: window.swapchain.format(),
                        samples: 1,
                    },
                    depth: {
                        load: Clear,
                        store: DontCare,
                        format: DEPTH_FORMAT,
                        samples: 1,
                    }
                },
                pass: {
                    color: [color],
                    depth_stencil: {depth}
                }
            )
            .unwrap(),
        );

        let framebuffers = create_framebuffers(window, render_pass.clone(), depth_buffer.clone());

        RenderTarget {
            depth_buffer,
            framebuffers,
            render_pass,
        }
    }

    pub fn handle_resize(&mut self, window: &Window) {
        let [fb_w, fb_h, _] = self.framebuffers[0].dimensions();
        let (win_w, win_h) = window
            .get_dimensions()
            .expect("couldn't get window dimensions");
        let win_dims = [win_w, win_h];
        let device = window.device.clone();
        if fb_w != win_w || fb_h != win_h {
            self.depth_buffer =
                AttachmentImage::transient(device, win_dims, DEPTH_FORMAT_TY).unwrap();
            self.framebuffers =
                create_framebuffers(window, self.render_pass.clone(), self.depth_buffer.clone());
        }
    }
}

fn create_framebuffers(
    window: &Window,
    render_pass: Arc<dyn RenderPassAbstract + Send + Sync>,
    depth_buffer: Arc<AttachmentImage<D16Unorm>>,
) -> Vec<Arc<dyn FramebufferAbstract + Send + Sync>> {
    window
        .images
        .iter()
        .map(|image| {
            Arc::new(
                Framebuffer::start(render_pass.clone())
                    .add(image.clone())
                    .unwrap()
                    .add(depth_buffer.clone())
                    .unwrap()
                    .build()
                    .unwrap(),
            ) as Arc<_>
        })
        .collect()
}
