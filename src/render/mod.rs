use crate::ball::Ball;
use legion::{system, world::SubWorld};
use std::{any::Any, sync::Arc};
use vulkano::buffer::BufferUsage;
use vulkano::{
    buffer::CpuAccessibleBuffer,
    command_buffer::{AutoCommandBufferBuilder, DynamicState, SubpassContents},
    descriptor::PipelineLayoutAbstract,
    format::Format,
    framebuffer::{RenderPass, RenderPassAbstract, Subpass},
    image::{view::ImageView, ImageUsage},
    instance::InstanceExtensions,
    pipeline::{vertex::SingleBufferDefinition, viewport::Viewport, GraphicsPipelineAbstract},
    swapchain::{
        self, AcquireError, ColorSpace, FullscreenExclusive, PresentMode, SurfaceTransform,
    },
    sync::{self, FlushError, GpuFuture, NowFuture},
};
use vulkano::{device::DeviceExtensions, framebuffer::Framebuffer};
use vulkano::{device::Features, pipeline::GraphicsPipeline};
use vulkano::{
    device::{Device, Queue},
    swapchain::Swapchain,
};
use vulkano::{framebuffer::FramebufferAbstract, instance::PhysicalDevice};
use vulkano::{image::SwapchainImage, instance::Instance};
use vulkano_win::VkSurfaceBuild;
use winit::{
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

pub struct DisplayConfig {
    pub width: u32,
    pub height: u32,
}

#[derive(Default, Copy, Clone)]
pub struct Vertex {
    position: [f32; 2],
}
vulkano::impl_vertex!(Vertex, position);

pub struct Graphics {
    config: DisplayConfig,
    instance: Arc<Instance>,
    device: Arc<Device>,
    queue: Arc<Queue>,
    swapchain: Arc<Swapchain<Window>>,
    dynamic_state: DynamicState,
    framebuffers: Vec<Arc<dyn FramebufferAbstract + Send + Sync>>,
    pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
    previous_frame_end: Option<Box<dyn GpuFuture>>,
    vertex_buffer: Arc<CpuAccessibleBuffer<[Vertex]>>,
}

fn window_size_dependent_setup(
    images: &[Arc<SwapchainImage<Window>>],
    render_pass: Arc<dyn RenderPassAbstract + Send + Sync>,
    dynamic_state: &mut DynamicState,
) -> Vec<Arc<dyn FramebufferAbstract + Send + Sync>> {
    let dimensions = images[0].dimensions();

    let viewport = Viewport {
        origin: [0.0, 0.0],
        dimensions: [dimensions[0] as f32, dimensions[1] as f32],
        depth_range: 0.0..1.0,
    };
    dynamic_state.viewports = Some(vec![viewport]);

    images
        .iter()
        .map(|image| {
            let view = ImageView::new(image.clone()).unwrap();
            Arc::new(
                Framebuffer::start(render_pass.clone())
                    .add(view)
                    .unwrap()
                    .build()
                    .unwrap(),
            ) as Arc<dyn FramebufferAbstract + Send + Sync>
        })
        .collect::<Vec<_>>()
}

pub fn init_graphics(display_config: DisplayConfig) -> (Graphics, EventLoop<()>) {
    let instance = {
        let extensions = vulkano_win::required_extensions();
        Instance::new(None, &extensions, None).expect("failed to create Vulkan instance")
    };
    let event_loop = EventLoop::new();
    let surface = WindowBuilder::new()
        .build_vk_surface(&event_loop, instance.clone())
        .unwrap();

    let physical = PhysicalDevice::enumerate(&instance)
        .next()
        .expect("no device available");
    let queue_family = physical
        .queue_families()
        .find(|&q| q.supports_graphics() && surface.is_supported(q).unwrap_or(false))
        .expect("couldn't find a graphical queue family");

    let device_ext = DeviceExtensions {
        khr_swapchain: true,
        ..DeviceExtensions::none()
    };
    let (device, mut queues) = {
        Device::new(
            physical,
            physical.supported_features(),
            &device_ext,
            [(queue_family, 0.5)].iter().cloned(),
        )
        .expect("failed to create device")
    };
    let queue = queues.next().unwrap();

    // Swapchain.
    let caps = surface.capabilities(physical).unwrap();
    let dimensions = caps
        .current_extent
        .unwrap_or([display_config.width, display_config.height]);
    let alpha = caps.supported_composite_alpha.iter().next().unwrap();
    let format = caps.supported_formats[0].0;
    let (swapchain, images) = Swapchain::new(
        device.clone(),
        surface.clone(),
        caps.min_image_count,
        format,
        dimensions,
        1,
        ImageUsage::color_attachment(),
        &queue,
        SurfaceTransform::Identity,
        alpha,
        PresentMode::Fifo,
        FullscreenExclusive::Default,
        true,
        ColorSpace::SrgbNonLinear,
    )
    .expect("failed to create swapchain");
    let mut dynamic_state = DynamicState {
        line_width: None,
        viewports: None,
        scissors: None,
        compare_mask: None,
        write_mask: None,
        reference: None,
    };

    let render_pass = Arc::new(
        vulkano::single_pass_renderpass!(device.clone(),
            attachments: {
                color: {
                    load: Clear,
                    store: Store,
                    format: Format::B8G8R8A8Unorm,
                    samples: 1,
                }
            },
            pass: {
                color: [color],
                depth_stencil: {}
            }
        )
        .unwrap(),
    );

    let (vs, fs) = create_shaders(&device);
    let pipeline = Arc::new(
        GraphicsPipeline::start()
            .vertex_input_single_buffer::<Vertex>()
            .vertex_shader(vs.main_entry_point(), ())
            .triangle_list()
            .viewports_dynamic_scissors_irrelevant(1)
            .fragment_shader(fs.main_entry_point(), ())
            .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
            .build(device.clone())
            .unwrap(),
    );

    let mut framebuffers =
        window_size_dependent_setup(&images, render_pass.clone(), &mut dynamic_state);

    let mut previous_frame_end = Some(sync::now(device.clone()).boxed());

    // We now create a buffer that will store the shape of our triangle.
    let vertex_buffer = {
        CpuAccessibleBuffer::from_iter(
            device.clone(),
            BufferUsage::all(),
            false,
            [
                Vertex {
                    position: [-0.5, -0.25],
                },
                Vertex {
                    position: [0.0, 0.5],
                },
                Vertex {
                    position: [0.25, -0.1],
                },
            ]
            .iter()
            .cloned(),
        )
        .unwrap()
    };

    (
        Graphics {
            config: display_config,
            instance: instance,
            device: device,
            queue: queue,
            swapchain: swapchain,
            dynamic_state: dynamic_state,
            framebuffers: framebuffers,
            pipeline: pipeline,
            previous_frame_end: previous_frame_end,
            vertex_buffer: vertex_buffer,
        },
        event_loop,
    )
}

mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        src: "
            #version 450
            layout(location = 0) in vec2 position;
            void main() {
                gl_Position = vec4(position, 0.0, 1.0);
            }
        "
    }
}

mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: "
            #version 450
            layout(location = 0) out vec4 f_color;
            void main() {
                f_color = vec4(1.0, 0.0, 0.0, 1.0);
            }
        "
    }
}
fn create_shaders(device: &Arc<Device>) -> (vs::Shader, fs::Shader) {
    let vs = vs::Shader::load(device.clone()).unwrap();
    let fs = fs::Shader::load(device.clone()).unwrap();
    (vs, fs)
}

#[system]
#[read_component(Ball)]
pub fn render_balls(world: &mut SubWorld, #[resource] graphics: &mut Graphics) {
    let (image_num, suboptimal, acquire_future) =
        match swapchain::acquire_next_image(graphics.swapchain.clone(), None) {
            Ok(r) => r,
            Err(AcquireError::OutOfDate) => {
                // recreate_swapchain = true;
                return;
            }
            Err(e) => panic!("Failed to acquire next image: {:?}", e),
        };
    let clear_values = vec![[0.0, 0.0, 1.0, 1.0].into()];
    let mut builder = AutoCommandBufferBuilder::primary_one_time_submit(
        graphics.device.clone(),
        graphics.queue.family(),
    )
    .unwrap();

    builder
        .begin_render_pass(
            graphics.framebuffers[image_num].clone(),
            SubpassContents::Inline,
            clear_values,
        )
        .unwrap()
        .draw(
            graphics.pipeline.clone(),
            &graphics.dynamic_state,
            vec![graphics.vertex_buffer.clone()],
            (),
            (),
            vec![],
        )
        .unwrap()
        .end_render_pass()
        .unwrap();

    // Finish building the command buffer by calling `build`.
    let command_buffer = builder.build().unwrap();

    let future = graphics
        .previous_frame_end
        .take()
        .unwrap()
        .join(acquire_future)
        .then_execute(graphics.queue.clone(), command_buffer)
        .unwrap()
        // The color output is now expected to contain our triangle. But in order to show it on
        // the screen, we have to *present* the image by calling `present`.
        //
        // This function does not actually present the image immediately. Instead it submits a
        // present command at the end of the queue. This means that it will only be presented once
        // the GPU has finished executing the command buffer that draws the triangle.
        .then_swapchain_present(
            graphics.queue.clone(),
            graphics.swapchain.clone(),
            image_num,
        )
        .then_signal_fence_and_flush();

    match future {
        Ok(future) => {
            graphics.previous_frame_end = Some(future.boxed());
        }
        Err(FlushError::OutOfDate) => {
            // recreate_swapchain = true;
            graphics.previous_frame_end = Some(sync::now(graphics.device.clone()).boxed());
        }
        Err(e) => {
            println!("Failed to flush future: {:?}", e);
            graphics.previous_frame_end = Some(sync::now(graphics.device.clone()).boxed());
        }
    }
}
