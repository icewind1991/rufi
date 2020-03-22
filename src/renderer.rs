use conrod_core::image::Map;
use conrod_core::render::Primitives;
use conrod_wgpu::Image;
use wgpu::{Device, Queue, Surface, SwapChain, SwapChainDescriptor};
use winit::dpi::PhysicalSize;
use winit::window::Window;

const MSAA_SAMPLES: u32 = 1;

pub struct Renderer {
    pub wgpu_renderer: conrod_wgpu::Renderer,
    pub swap_chain: SwapChain,
    pub size: PhysicalSize<u32>,
    pub device: Device,
    pub queue: Queue,
    pub surface: Surface,
    pub swap_chain_desc: SwapChainDescriptor,
    pub multisampled_framebuffer: wgpu::TextureView,
}

impl Renderer {
    pub fn new(window: &Window) -> Self {
        let size = window.inner_size();
        let surface = wgpu::Surface::create(window);

        // Select an adapter and gpu device.
        let adapter_opts = wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            backends: wgpu::BackendBit::PRIMARY,
        };
        let adapter = wgpu::Adapter::request(&adapter_opts).unwrap();
        let extensions = wgpu::Extensions {
            anisotropic_filtering: false,
        };
        let limits = wgpu::Limits::default();
        let device_desc = wgpu::DeviceDescriptor { extensions, limits };
        let (device, queue) = adapter.request_device(&device_desc);

        // Create the swapchain.
        let format = wgpu::TextureFormat::Bgra8UnormSrgb;
        let swap_chain_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Vsync,
        };
        let swap_chain = device.create_swap_chain(&surface, &swap_chain_desc);

        // Create the renderer for rendering conrod primitives.
        let wgpu_renderer = conrod_wgpu::Renderer::new(&device, MSAA_SAMPLES, format);

        // The intermediary multisampled texture that will be resolved (MSAA).
        let multisampled_framebuffer =
            create_multisampled_framebuffer(&device, &swap_chain_desc, MSAA_SAMPLES);

        Renderer {
            wgpu_renderer,
            multisampled_framebuffer,
            swap_chain,
            size,
            device,
            surface,
            swap_chain_desc,
            queue,
        }
    }

    pub fn render(&mut self, primitives: Primitives, window: &Window, image_map: &Map<Image>) {
        // The window frame that we will draw to.
        let frame = self.swap_chain.get_next_texture();

        // Begin encoding commands.
        let cmd_encoder_desc = wgpu::CommandEncoderDescriptor { todo: 0 };
        let mut encoder = self.device.create_command_encoder(&cmd_encoder_desc);

        // Feed the renderer primitives and update glyph cache texture if necessary.
        let scale_factor = window.scale_factor();
        let [win_w, win_h]: [f32; 2] = [self.size.width as f32, self.size.height as f32];
        let viewport = [0.0, 0.0, win_w, win_h];
        if let Some(cmd) = self
            .wgpu_renderer
            .fill(image_map, viewport, scale_factor, primitives)
            .unwrap()
        {
            cmd.load_buffer_and_encode(&self.device, &mut encoder);
        }

        // Begin the render pass and add the draw commands.
        {
            // This condition allows to more easily tweak the MSAA_SAMPLES constant.
            let (attachment, resolve_target) = match MSAA_SAMPLES {
                1 => (&frame.view, None),
                _ => (&self.multisampled_framebuffer, Some(&frame.view)),
            };
            let color_attachment_desc = wgpu::RenderPassColorAttachmentDescriptor {
                attachment,
                resolve_target,
                load_op: wgpu::LoadOp::Clear,
                store_op: wgpu::StoreOp::Store,
                clear_color: wgpu::Color::BLACK,
            };

            let render_pass_desc = wgpu::RenderPassDescriptor {
                color_attachments: &[color_attachment_desc],
                depth_stencil_attachment: None,
            };
            let mut render_pass = encoder.begin_render_pass(&render_pass_desc);

            let render = self.wgpu_renderer.render(&self.device, image_map);
            render_pass.set_pipeline(render.pipeline);
            render_pass.set_vertex_buffers(0, &[(&render.vertex_buffer, 0)]);
            let instance_range = 0..1;
            for cmd in render.commands {
                match cmd {
                    conrod_wgpu::RenderPassCommand::SetBindGroup { bind_group } => {
                        render_pass.set_bind_group(0, bind_group, &[]);
                    }
                    conrod_wgpu::RenderPassCommand::SetScissor {
                        top_left,
                        dimensions,
                    } => {
                        let [x, y] = top_left;
                        let [w, h] = dimensions;
                        render_pass.set_scissor_rect(x, y, w, h);
                    }
                    conrod_wgpu::RenderPassCommand::Draw { vertex_range } => {
                        render_pass.draw(vertex_range, instance_range.clone());
                    }
                }
            }
        }

        self.queue.submit(&[encoder.finish()]);
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.size = new_size;
        self.swap_chain_desc.width = new_size.width;
        self.swap_chain_desc.height = new_size.height;
        self.swap_chain = self
            .device
            .create_swap_chain(&self.surface, &self.swap_chain_desc);
        self.multisampled_framebuffer =
            create_multisampled_framebuffer(&self.device, &self.swap_chain_desc, MSAA_SAMPLES);
    }
}

fn create_multisampled_framebuffer(
    device: &wgpu::Device,
    sc_desc: &wgpu::SwapChainDescriptor,
    sample_count: u32,
) -> wgpu::TextureView {
    let multisampled_texture_extent = wgpu::Extent3d {
        width: sc_desc.width,
        height: sc_desc.height,
        depth: 1,
    };
    let multisampled_frame_descriptor = &wgpu::TextureDescriptor {
        size: multisampled_texture_extent,
        array_layer_count: 1,
        mip_level_count: 1,
        sample_count: sample_count,
        dimension: wgpu::TextureDimension::D2,
        format: sc_desc.format,
        usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
    };
    device
        .create_texture(multisampled_frame_descriptor)
        .create_default_view()
}
