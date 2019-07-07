#[macro_use]
extern crate vulkano;
// note that vulkano_shaders and any other 2018 edition crates
// won't be listed explicitly
extern crate winit;
extern crate vulkano_win;

extern crate rand;

use std::sync::Arc;

mod game;

fn main() {

    //////////////////////////
    // window, device, etc.

    use vulkano::instance::{Instance, PhysicalDevice};
    use vulkano::device::{Device, DeviceExtensions};
    use vulkano_win::VkSurfaceBuild;
    use winit::{EventsLoop, WindowBuilder};

    let instance = {
        let extensions = vulkano_win::required_extensions();

        Instance::new(None, &extensions, None).unwrap()
    };

    let physical = PhysicalDevice::enumerate(&instance).next().unwrap();
    println!("Using device: {} (type: {:?})", physical.name(), physical.ty());


    let mut events_loop = EventsLoop::new();
    let surface = WindowBuilder::new()
        .build_vk_surface(&events_loop, instance.clone()).unwrap();
    let window = surface.window();

    let queue_family = physical.queue_families().find(|&q| {
        // We take the first queue that supports drawing to our window.
        q.supports_graphics() && surface.is_supported(q).unwrap_or(false)
    }).unwrap();

    let device_ext = DeviceExtensions {
        khr_swapchain: true,
        .. DeviceExtensions::none()
    };
    let (device, mut queues) = Device::new(
        physical,
        physical.supported_features(),
        &device_ext,
        [(queue_family, 0.5)].iter().cloned(),
    ).unwrap();

    let queue = queues.next().unwrap();

    ///////////////
    // swapchain

    use vulkano::swapchain::{PresentMode, SurfaceTransform, Swapchain};

    let (mut swapchain, images) = {
        let caps = surface.capabilities(physical).unwrap();
        let usage = caps.supported_usage_flags;
        let alpha = caps.supported_composite_alpha.iter().next().unwrap();
        let format = caps.supported_formats[0].0;
        let initial_dimensions = if let Some(dimensions) = window.get_inner_size() {
            // convert to physical pixels
            let dimensions: (u32, u32) = dimensions.to_physical(window.get_hidpi_factor()).into();
            [dimensions.0, dimensions.1]
        } else {
            // The window no longer exists so exit the application.
            return;
        };

        Swapchain::new(
            device.clone(),
            surface.clone(),
            caps.min_image_count,
            format,
            initial_dimensions,
            1,
            usage,
            &queue,
            SurfaceTransform::Identity,
            alpha,
            PresentMode::Fifo,
            true,
            None
        ).unwrap()
    };

    /////////////////////////
    // shaders, vertex gen

    use vulkano::buffer::CpuBufferPool;

    #[derive(Debug, Clone, Copy)]
    struct Vertex {
        position: [f32; 2],
        color: [f32; 3],
        circle: [f32; 2],
    }
    impl_vertex!(Vertex, position, color, circle);

    let vertex_buffer_pool = CpuBufferPool::vertex_buffer(device.clone());

    fn square_coords<F: FnMut(f32, f32)>(mut f: F) {
        f(-1.0, -1.0);
        f( 1.0, -1.0);
        f( 1.0,  1.0);
        f( 1.0,  1.0);
        f(-1.0,  1.0);
        f(-1.0, -1.0);
    }

    fn circle_vertices<F: FnMut(Vertex)>(
        x: f32, y: f32,
        r: f32,
        color: [f32; 3],
        mut f: F
    ) {
        square_coords(|dx, dy|
            f(Vertex {
                position: [x+r*dx, y+r*dy],
                color,
                circle: [dx, dy]
            })
        );
    }

    fn square_vertices<F: FnMut(Vertex)>(
        x: f32, y: f32,
        r: f32,
        color: [f32; 3],
        mut f: F
    ) {
        square_coords(|dx, dy|
            f(Vertex {
                position: [x+r*dx, y+r*dy],
                color,
                circle: [0.0, 0.0],  // all pixels will be transparent
            })
        );
    }

    fn triangle_vertices<F: FnMut(Vertex)>(
        coords: [[f32; 2]; 3],
        color: [f32; 3],
        mut f: F
    ) {
        for &position in &coords {
            f(Vertex { position, color, circle: [0.0, 0.0] });
        }
    }

    fn rectangle_vertices<F: FnMut(Vertex)>(
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        color: [f32; 3],
        mut f: F
    ) {
        // inconsistent winding order who cares
        triangle_vertices([[x1, y1], [x2, y1], [x1, y2]], color, |v| f(v));
        triangle_vertices([[x1, y2], [x2, y2], [x2, y1]], color, |v| f(v));
    }

    const SCREEN_TOP_EDGE: f32 = -10.0;
    const SCREEN_BOTTOM_EDGE: f32 = 10.0;
    const SCREEN_LEFT_EDGE: f32 = -8.0;
    const SCREEN_RIGHT_EDGE: f32 = 12.0;
    const SCREEN_HEIGHT: f32 = SCREEN_BOTTOM_EDGE - SCREEN_TOP_EDGE;

    mod vs {
        vulkano_shaders::shader!{
            ty: "vertex",
            src: "
#version 450

layout(location = 0) in vec2 position;
layout(location = 1) in vec3 color;
layout(location = 2) in vec2 circle;

layout(location = 0) out vec3 v_color;
layout(location = 1) out vec2 v_circle;

void main() {
    v_color = color;
    v_circle = circle;

    vec2 disp = position - vec2(2.0, 0.0);
    gl_Position = vec4(0.1 * disp, 0.0, 1.0);
}"
        }
    }

    mod fs {
        vulkano_shaders::shader!{
            ty: "fragment",
            src: "
#version 450

layout(location = 0) in vec3 v_color;
layout(location = 1) in vec2 v_circle;
layout(location = 0) out vec4 f_color;

void main() {
    float rr = dot(v_circle, v_circle);
    f_color = vec4(v_color, rr < 1.0 ? 1.0 : 0.0);
}
"
        }
    }

    let vs = vs::Shader::load(device.clone()).unwrap();
    let fs = fs::Shader::load(device.clone()).unwrap();

    ///////////////////////////
    // render pass, pipeline

    use vulkano::framebuffer::Subpass;
    use vulkano::pipeline::GraphicsPipeline;

    let render_pass = Arc::new(single_pass_renderpass!(
        device.clone(),
        attachments: {
            // `color` is a custom name we give to the first and only attachment.
            color: {
                // `load: Clear` means that we ask the GPU to clear the content of this
                // attachment at the start of the drawing.
                load: Clear,
                // `store: Store` means that we ask the GPU to store the output of the draw
                // in the actual image. We could also ask it to discard the result.
                store: Store,
                // `format: <ty>` indicates the type of the format of the image. This has to
                // be one of the types of the `vulkano::format` module (or alternatively one
                // of your structs that implements the `FormatDesc` trait). Here we use the
                // same format as the swapchain.
                format: swapchain.format(),
                samples: 1,
            }
        },
        pass: {
            // We use the attachment named `color` as the one and only color attachment.
            color: [color],
            // No depth-stencil attachment is indicated with empty brackets.
            depth_stencil: {}
        }
    ).unwrap());

    let pipeline = Arc::new(GraphicsPipeline::start()
        .vertex_input_single_buffer()
        .vertex_shader(vs.main_entry_point(), ())
        // The content of the vertex buffer describes a list of triangles.
        .triangle_list()
        // Use a resizable viewport set to draw over the entire window
        .viewports_dynamic_scissors_irrelevant(1)
        .fragment_shader(fs.main_entry_point(), ())
        .blend_alpha_blending()
        // We have to indicate which subpass of which render pass this pipeline
        // is going to be used in. The pipeline will only be usable from this
        // particular subpass.
        .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
        // Now that our builder is filled, we call `build()` to obtain an
        // actual pipeline.
        .build(device.clone())
        .unwrap());

    ////////////////////////////
    // loop (incl. variables)

    use vulkano::sync::GpuFuture;

    // Dynamic viewports allow us to recreate just the viewport when the window
    // is resized Otherwise we would have to recreate the whole pipeline.
    let mut dynamic_state = vulkano::command_buffer::DynamicState {
        line_width: None,
        viewports: None,
        scissors: None,
    };

    let mut framebuffers = window_size_dependent_setup(&images, render_pass.clone(), &mut dynamic_state);

    let mut recreate_swapchain = false;

    // hold it so that we don't block until we want to draw the next frame
    let mut previous_frame_end =
        Box::new(vulkano::sync::now(device.clone())) as Box<GpuFuture>;

    let mut game: game::Game = Default::default();

    loop {
        // cleanup unused gpu resources
        previous_frame_end.cleanup_finished();

        ////////////
        // resize

        use vulkano::swapchain::{AcquireError, SwapchainCreationError};

        if recreate_swapchain {
            // Get the new dimensions of the window.
            let dimensions = if let Some(dimensions) = window.get_inner_size() {
                let dimensions: (u32, u32) = dimensions.to_physical(window.get_hidpi_factor()).into();
                [dimensions.0, dimensions.1]
            } else {
                return;
            };

            let (new_swapchain, new_images) = match swapchain.recreate_with_dimension(dimensions) {
                Ok(r) => r,
                // This error tends to happen when the user is manually
                // resizing the window.
                // Simply restarting the loop is the easiest way to fix this
                // issue.
                Err(SwapchainCreationError::UnsupportedDimensions) => continue,
                Err(err) => panic!("{:?}", err)
            };

            swapchain = new_swapchain;
            // Because framebuffers contains an Arc on the old swapchain,
            // we need to recreate framebuffers as well.
            framebuffers = window_size_dependent_setup(&new_images, render_pass.clone(), &mut dynamic_state);

            recreate_swapchain = false;
        }

        // blocks if all images are being drawn to
        let next =
            vulkano::swapchain::acquire_next_image(swapchain.clone(), None);
        let (image_num, acquire_future) = match next {
            Ok(r) => r,
            Err(AcquireError::OutOfDate) => {
                recreate_swapchain = true;
                continue;
            },
            Err(err) => panic!("{:?}", err)
        };

        /////////////////
        // draw things

        let clear_values = vec!([0.7, 0.7, 0.7, 1.0].into());

        let vertex_buffer = {
            // colours
            const HUNGER_C: [f32; 3] = [0.5, 0.2, 0.0];
            const NOURISH_C: [f32; 3] = [0.0, 0.0, 0.4];
            const HEALTH_C: [f32; 3] = [1.0, 0.0, 0.0];
            const DAMAGE_C: [f32; 3] = [0.2, 0.0, 0.0];
            const PLAYER_C: [f32; 3] = HEALTH_C;

            // @Performance ideally we would reuse between frames
            let mut vs = Vec::with_capacity(6 * 400);

            // background
            let mut max = 10;
            for i in 0..3 {
                if max < game.counts[i] {
                    max = game.counts[i];
                }
            }
            let mut heights = [
                (0.0, HUNGER_C),
                (0.0, NOURISH_C),
                (0.0, HEALTH_C),
                (0.0, DAMAGE_C),
            ];
            for i in 0..3 {
                heights[i].0 = game.counts[i] as f32 / max as f32;
            }
            heights[3].0 = game.counts[3] as f32 / game::INV_CAP as f32;
            // descending order
            heights[0..3].sort_by(|&(l, _), &(r, _)|
                std::cmp::PartialOrd::partial_cmp(&r, &l).unwrap()
            );
            for &(height, color) in &heights {
                rectangle_vertices(
                    SCREEN_LEFT_EDGE,
                    SCREEN_BOTTOM_EDGE - height * SCREEN_HEIGHT,
                    SCREEN_RIGHT_EDGE,
                    SCREEN_BOTTOM_EDGE,
                    color,
                    |v| vs.push(v),
                );
            }


            // water lines

            // grid squares
            for i in -7..8 {
                for j in -7..8 {
                    square_vertices(
                        i as f32,
                        j as f32,
                        0.4,
                        [0.6, 0.6, 0.6],
                        |v| vs.push(v),
                    );
                }
            }

            // pickups
            for &([x, y], disp, flav) in &game.world {
                use game::PickupFlavor::*;
                let color = match flav {
                    Hunger => HUNGER_C,
                    Nourishment => NOURISH_C,
                };
                use game::Displacement;
                let [dx, dy] = match disp {
                    Displacement::TL => [-0.2, -0.2],
                    Displacement::TR => [ 0.2, -0.2],
                    Displacement::M  => [ 0.0,  0.0],
                    Displacement::BL => [-0.2,  0.2],
                    Displacement::BR => [ 0.2,  0.2],
                };
                circle_vertices(
                    x as f32 + dx,
                    y as f32 + dy,
                    0.1,
                    color,
                    |v| vs.push(v),
                );
            }

            // player
            circle_vertices(
                game.pos[0] as f32,
                game.pos[1] as f32,
                0.3,
                PLAYER_C,
                |v| vs.push(v),
            );

            // inv
            for (i, &flav) in game.items.iter().enumerate() {
                let x = i as i64 % 4 + 8;
                let y = i as i64 / 4 - 7;
                use game::Item::*;
                let (color, size) = match flav {
                    Hunger(i) => (
                        HUNGER_C,
                        i as f32 / game::HUNGER_TIMER as f32
                    ),
                    Nourishment(i) => (
                        NOURISH_C,
                        i as f32 / game::NOURISH_TIMER as f32
                    ),
                    Health(i) => (
                        HEALTH_C,
                        i as f32 / game::HEALTH_TIMER as f32
                    ),
                    Damage => (DAMAGE_C, 1.0),
                };
                circle_vertices(
                    x as f32,
                    y as f32,
                    0.3 * size,
                    color,
                    |v| vs.push(v),
                );
            }

            vertex_buffer_pool
                .chunk(vs.into_iter())
                .unwrap()
        };

        ///////////////////////
        // gpu do your thing

        use vulkano::command_buffer::AutoCommandBufferBuilder;
        let command_buffer = AutoCommandBufferBuilder::primary_one_time_submit(
            device.clone(),
            queue.family(),
        ).unwrap()
            // Before we can draw, we have to *enter a render pass*. There are two methods to do
            // this: `draw_inline` and `draw_secondary`. The latter is a bit more advanced and is
            // not covered here.
            //
            // The third parameter builds the list of values to clear the attachments with. The API
            // is similar to the list of attachments when building the framebuffers, except that
            // only the attachments that use `load: Clear` appear in the list.
            .begin_render_pass(framebuffers[image_num].clone(), false, clear_values)
            .unwrap()

            // We are now inside the first subpass of the render pass. We add a draw command.
            //
            // The last two parameters contain the list of resources to pass to the shaders.
            // Since we used an `EmptyPipeline` object, the objects have to be `()`.
            .draw(pipeline.clone(), &dynamic_state, vertex_buffer.clone(), (), ())
            .unwrap()

            // We leave the render pass by calling `draw_end`. Note that if we had multiple
            // subpasses we could have called `next_inline` (or `next_secondary`) to jump to the
            // next subpass.
            .end_render_pass()
            .unwrap()

            .build().unwrap();

        let future = previous_frame_end.join(acquire_future)
            .then_execute(queue.clone(), command_buffer).unwrap()

            // add a present command to tell the gpu to show the frame once done
            .then_swapchain_present(queue.clone(), swapchain.clone(), image_num)
            .then_signal_fence_and_flush();

        use vulkano::sync::FlushError;
        match future {
            Ok(future) => {
                previous_frame_end = Box::new(future) as Box<_>;
            }
            Err(FlushError::OutOfDate) => {
                recreate_swapchain = true;
                previous_frame_end =
                    Box::new(vulkano::sync::now(device.clone())) as Box<_>;
            }
            Err(e) => {
                println!("{:?}", e);
                previous_frame_end =
                    Box::new(vulkano::sync::now(device.clone())) as Box<_>;
            }
        }

        // Note that in more complex programs it is likely that one of `acquire_next_image`,
        // `command_buffer::submit`, or `present` will block for some time. This happens when the
        // GPU's queue is full and the driver has to wait until the GPU finished some work.
        //
        // Unfortunately the Vulkan API doesn't provide any way to not wait or to detect when a
        // wait would happen. Blocking may be the desired behavior, but if you don't want to
        // block you should spawn a separate thread dedicated to submissions.

        // Handling the window events in order to close the program when the user wants to close
        // it.

        //////////////////////////////
        // window events/user input

        let mut done = false;
        let mut inputs = Vec::new();
        events_loop.poll_events(|ev| {
            use winit::{Event, WindowEvent, DeviceEvent, KeyboardInput, ElementState};
            match ev {
                Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => done = true,
                Event::WindowEvent { event: WindowEvent::Resized(_), .. } => recreate_swapchain = true,
                Event::DeviceEvent { event: DeviceEvent::Key (KeyboardInput { virtual_keycode: Some(key), state: ElementState::Pressed, .. }), .. } => {
                    use winit::VirtualKeyCode::*;
                    match key {
                        Escape => done = true,
                        W | Up => inputs.push(game::Dir::Up),
                        S | Down => inputs.push(game::Dir::Down),
                        A | Left => inputs.push(game::Dir::Left),
                        D | Right => inputs.push(game::Dir::Right),
                        R => game.reset(),
                        _ => (),
                    }
                }
                _ => ()
            }
        });
        if done { return; }

        for input in inputs {
            game.update(input);
        }
    }
}

use vulkano::command_buffer::DynamicState;
use vulkano::framebuffer::{Framebuffer, FramebufferAbstract, RenderPassAbstract};
use vulkano::image::SwapchainImage;
use winit::Window;

/// This method is called once during initialization, then again whenever the window is resized
fn window_size_dependent_setup(
    images: &[Arc<SwapchainImage<Window>>],
    render_pass: Arc<RenderPassAbstract + Send + Sync>,
    dynamic_state: &mut DynamicState
) -> Vec<Arc<FramebufferAbstract + Send + Sync>> {

    use vulkano::pipeline::viewport::Viewport;

    let dimensions = images[0].dimensions();

    let viewport = Viewport {
        origin: [0.0, 0.0],
        dimensions: [dimensions[0] as f32, dimensions[1] as f32],
        depth_range: 0.0 .. 1.0,
    };
    dynamic_state.viewports = Some(vec!(viewport));

    images.iter().map(|image| {
        Arc::new(
            Framebuffer::start(render_pass.clone())
                .add(image.clone()).unwrap()
                .build().unwrap()
        ) as Arc<FramebufferAbstract + Send + Sync>
    }).collect::<Vec<_>>()
}
