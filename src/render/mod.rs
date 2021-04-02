use crate::{ball::Ball, ball::Trail, ball::Trails, simulation::SimulationData};
use legion::IntoQuery;
use legion::{system, world::SubWorld};
use nalgebra::Vector2;
use std::{any::Any, sync::Arc};
use vulkano::{
    buffer::BufferUsage,
    pipeline::blend::{AttachmentBlend, BlendFactor, BlendOp},
};
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
    dpi::LogicalSize,
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};
pub struct DisplayConfig {
    pub width: u32,
    pub height: u32,
    pub max_vertices: i32,
    pub blur: bool,
}

#[derive(Default, Copy, Clone)]
pub struct Vertex {
    position: [f32; 2],
    coords: [f32; 2],
    color: [f32; 3],
    trail_length: f32,
    total_portion: f32,
}
vulkano::impl_vertex!(Vertex, position, coords, color, trail_length, total_portion);

pub struct Graphics {
    pub config: DisplayConfig,
    instance: Arc<Instance>,
    device: Arc<Device>,
    queue: Arc<Queue>,
    swapchain: Arc<Swapchain<Window>>,
    dynamic_state: DynamicState,
    framebuffers: Vec<Arc<dyn FramebufferAbstract + Send + Sync>>,
    pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
    previous_frame_ends: Vec<Option<Box<dyn GpuFuture>>>,
    vertex_buffers: Vec<Arc<CpuAccessibleBuffer<[Vertex]>>>,
    index_buffers: Vec<Arc<CpuAccessibleBuffer<[u16]>>>,
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
        .with_inner_size(LogicalSize::new(
            display_config.width,
            display_config.height,
        ))
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
            .blend_collective(AttachmentBlend {
                enabled: true,
                color_op: BlendOp::Add,
                color_source: BlendFactor::SrcAlpha,
                color_destination: BlendFactor::One,
                alpha_op: BlendOp::Add,
                alpha_source: BlendFactor::One,
                alpha_destination: BlendFactor::Zero,
                mask_red: true,
                mask_green: true,
                mask_blue: true,
                mask_alpha: true,
            })
            .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
            .build(device.clone())
            .unwrap(),
    );

    let mut framebuffers =
        window_size_dependent_setup(&images, render_pass.clone(), &mut dynamic_state);

    let mut previous_frame_ends = images.iter().map(|image| None).collect::<Vec<_>>();

    // We now create a buffer that will store the shape of our triangle.
    let vertex_buffers = images
        .iter()
        .map(|image| {
            CpuAccessibleBuffer::from_iter(
                device.clone(),
                BufferUsage::all(),
                false,
                (0..display_config.max_vertices).map(|i| Vertex::default()),
            )
            .expect("failed to create buffer")
        })
        .collect::<Vec<_>>();

    let index_buffers = images
        .iter()
        .map(|image| {
            CpuAccessibleBuffer::from_iter(
                device.clone(),
                BufferUsage::all(),
                false,
                (0..display_config.max_vertices).map(|i| 0u16),
            )
            .expect("failed to create buffer")
        })
        .collect::<Vec<_>>();

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
            previous_frame_ends: previous_frame_ends,
            vertex_buffers: vertex_buffers,
            index_buffers: index_buffers,
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
            layout(location = 1) in vec2 coords;
            layout(location = 2) in vec3 color;
            layout(location = 3) in float trail_length;
            layout(location = 4) in float total_portion;
            layout(location = 0) out vec2 outCoords;
            layout(location = 1) out vec3 outColor;
            layout(location = 2) out float out_trail_length;
            layout(location = 3) out float out_total_portion;
            void main() {
                gl_Position = vec4(position, 0.0, 1.0);
                outCoords = coords;
                outColor = color;
                out_trail_length = trail_length;
                out_total_portion = total_portion;
            }
        "
    }
}

mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: "
            #version 450
            const float EPSILON = 0.0001;
            const float aa_pixels = 2.;
            layout(location = 0) in vec2 coords;
            layout(location = 1) in vec3 color;
            layout(location = 2) in float trail_length;
            layout(location = 3) in float total_portion;
            layout(location = 0) out vec4 f_color;

            float correct_value(float val, float d){
                if (val - d < 0){
                    return (val+d)/2;
                }
                return val;
            }

            void main() {
                // The goal of anti aliasing is estimating the lit area, and sampling the color at the middle of
                // that area (i.e. average color).
                // For the second bit, we make sure to correct the values sampled.
                // For the first, we compute the area with the 'factor' logic.
                
                // In our case, the color is seg.
                // At the top/bottom, 
                float d2 = 1-coords.y*coords.y;
                float dwidth = length(vec2(dFdx(d2), dFdy(d2)));
                d2 = correct_value(d2, dwidth*0.5*aa_pixels);

                float d = sqrt(max(0,d2));
                float t0 = max(0, coords.x-d);
                float t1 = min(trail_length, coords.x+d);
                // Note that seg reaches negative value at the sides.
                float seg = t1 - t0;
                float xwidth = length(vec2(dFdx(coords.x), dFdy(coords.x)));
                seg = min(correct_value(seg, xwidth*0.5*aa_pixels), trail_length);
                float normalized_length = (seg+EPSILON)/(trail_length+EPSILON)*total_portion;
                float alpha = clamp(normalized_length, 0, 1);
                // alpha=seg;

                float ex = coords.x-clamp(coords.x, 0, trail_length);
                float dist = sqrt(ex*ex + coords.y*coords.y);
                float pwidth = length(vec2(dFdx(dist), dFdy(dist)));
                float factor = smoothstep(-0.5*aa_pixels, 0.5*aa_pixels, (1-dist)/pwidth);
                // alpha = factor;
                alpha *= factor;
                f_color = vec4(color, alpha);
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
#[read_component(Trails)]
pub fn render_balls(
    world: &mut SubWorld,
    #[resource] graphics: &mut Graphics,
    #[resource] simulation_data: &mut SimulationData,
) {
    let (image_num, suboptimal, acquire_future) =
        match swapchain::acquire_next_image(graphics.swapchain.clone(), None) {
            Ok(r) => r,
            Err(AcquireError::OutOfDate) => {
                // recreate_swapchain = true;
                return;
            }
            Err(e) => panic!("Failed to acquire next image: {:?}", e),
        };
    let clear_values = vec![[0.0, 0.0, 0.0, 1.0].into()];
    let mut builder = AutoCommandBufferBuilder::primary_one_time_submit(
        graphics.device.clone(),
        graphics.queue.family(),
    )
    .unwrap();

    // Wait for last render of that image to end.
    graphics.previous_frame_ends[image_num].take().map(|res| {
        res.then_signal_fence().wait(None).unwrap();
    });
    let vertex_buffer = &mut graphics.vertex_buffers[image_num];
    let index_buffer = &mut graphics.index_buffers[image_num];

    // Fill buffers.
    {
        let mut vertex_buffer_data = vertex_buffer.write().unwrap();
        let mut index_buffer_data = index_buffer.write().unwrap();
        let mut vertex_index = 0;
        let mut index_index = 0;
        for (ball, trails) in <(&Ball, &Trails)>::query().iter(world) {
            let mut local_trails: Vec<Trail>;
            let all_trails = if !graphics.config.blur {
                local_trails = vec![Trail {
                    position0: ball.position,
                    position1: ball.position,
                    initial_time: simulation_data.time,
                    final_time: simulation_data.next_time,
                }];
                &local_trails
            } else {
                &trails.trails
            };
            for trail in all_trails {
                let mut u_vec = trail.position1 - trail.position0;
                let trail_length = u_vec.norm() / ball.radius;
                if u_vec.norm() < 0.001 {
                    u_vec = Vector2::new(1.0, 0.0);
                } else {
                    u_vec /= u_vec.norm();
                }
                let v_vec = Vector2::new(-u_vec[1], u_vec[0]);

                index_buffer_data[index_index + 0] = (vertex_index) as u16;
                index_buffer_data[index_index + 1] = (vertex_index + 1) as u16;
                index_buffer_data[index_index + 2] = (vertex_index + 2) as u16;
                index_buffer_data[index_index + 3] = (vertex_index + 2) as u16;
                index_buffer_data[index_index + 4] = (vertex_index + 1) as u16;
                index_buffer_data[index_index + 5] = (vertex_index + 3) as u16;
                index_index += 6;

                for vo in [-1.1f64, 1.1].iter() {
                    for ho in [-1.1f64, trail_length + 1.1].iter() {
                        let position = trail.position0 + (*vo * v_vec + *ho * u_vec) * ball.radius;
                        vertex_buffer_data[vertex_index] = Vertex {
                            position: [
                                -1.0 + 2.0 * position[0] as f32 / graphics.config.width as f32,
                                -1.0 + 2.0 * position[1] as f32 / graphics.config.height as f32,
                            ],
                            coords: [*ho as f32, *vo as f32],
                            color: [ball.color[0], ball.color[1], ball.color[2]],
                            trail_length: trail_length as f32,
                            total_portion: ((trail.final_time - trail.initial_time)
                                / (simulation_data.next_time - simulation_data.time))
                                as f32,
                        };
                        vertex_index += 1;
                    }
                }
            }
        }

        // Clear the rest of the index buffer;
        while index_index < index_buffer_data.len() {
            index_buffer_data[index_index] = 0;
            index_index += 1;
        }
    }

    // Start rendering.
    builder
        .begin_render_pass(
            graphics.framebuffers[image_num].clone(),
            SubpassContents::Inline,
            clear_values,
        )
        .unwrap()
        .draw_indexed(
            graphics.pipeline.clone(),
            &graphics.dynamic_state,
            vec![vertex_buffer.clone()],
            index_buffer.clone(),
            (),
            (),
            vec![],
        )
        .unwrap()
        .end_render_pass()
        .unwrap();

    // Finish building the command buffer by calling `build`.
    let command_buffer = builder.build().unwrap();

    let future = sync::now(graphics.device.clone())
        .join(acquire_future)
        .then_execute(graphics.queue.clone(), command_buffer)
        .unwrap()
        .then_swapchain_present(
            graphics.queue.clone(),
            graphics.swapchain.clone(),
            image_num,
        )
        .then_signal_fence_and_flush();

    match future {
        Ok(future) => {
            graphics.previous_frame_ends[image_num] = Some(future.boxed());
        }
        Err(FlushError::OutOfDate) => {
            // recreate_swapchain = true;
            graphics.previous_frame_ends[image_num] = None;
        }
        Err(e) => {
            println!("Failed to flush future: {:?}", e);
            graphics.previous_frame_ends[image_num] = None;
        }
    }
}
