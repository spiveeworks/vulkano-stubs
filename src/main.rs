#[macro_use]
extern crate vulkano;
extern crate vulkano_shaders;
extern crate winit;
extern crate vulkano_win;

use vulkano::buffer::CpuBufferPool;
use vulkano::command_buffer::{AutoCommandBufferBuilder, DynamicState};
use vulkano::device::{Device, DeviceExtensions};
use vulkano::framebuffer::{Framebuffer, FramebufferAbstract, Subpass, RenderPassAbstract};
use vulkano::image::SwapchainImage;
use vulkano::instance::{Instance, PhysicalDevice};
use vulkano::pipeline::GraphicsPipeline;
use vulkano::pipeline::viewport::Viewport;
use vulkano::swapchain::{AcquireError, PresentMode, SurfaceTransform, Swapchain, SwapchainCreationError};
use vulkano::swapchain;
use vulkano::sync::{GpuFuture, FlushError};
use vulkano::sync;

use vulkano_win::VkSurfaceBuild;

use winit::{EventsLoop, Window, WindowBuilder};

use std::sync::Arc;

mod dir;
use dir::Dir1;
use dir::Dir2;

fn main() {
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


    #[derive(Debug, Clone, Copy)]
    struct Vertex {
        position: [f32; 3],
        color: [f32; 3],
    }
    impl_vertex!(Vertex, position, color);

    let vertex_buffer_pool = CpuBufferPool::vertex_buffer(device.clone());

    fn cube_vertices<F: FnMut(Vertex)>(x: f32, y: f32, z: f32, mut f: F) {
        f(Vertex { position: [x-0.5, y-0.5, z+0.0], color: [0.6, 0.6, 0.6] });
        f(Vertex { position: [x-0.5, y+0.5, z+0.0], color: [0.6, 0.6, 0.6] });
        f(Vertex { position: [x-0.5, y+0.5, z+1.0], color: [0.6, 0.6, 0.6] });
        f(Vertex { position: [x-0.5, y+0.5, z+1.0], color: [0.6, 0.6, 0.6] });
        f(Vertex { position: [x-0.5, y-0.5, z+1.0], color: [0.6, 0.6, 0.6] });
        f(Vertex { position: [x-0.5, y-0.5, z+0.0], color: [0.6, 0.6, 0.6] });

        f(Vertex { position: [x-0.5, y-0.5, z+0.0], color: [0.3, 0.3, 0.3] });
        f(Vertex { position: [x+0.5, y-0.5, z+0.0], color: [0.3, 0.3, 0.3] });
        f(Vertex { position: [x+0.5, y-0.5, z+1.0], color: [0.3, 0.3, 0.3] });
        f(Vertex { position: [x+0.5, y-0.5, z+1.0], color: [0.3, 0.3, 0.3] });
        f(Vertex { position: [x-0.5, y-0.5, z+1.0], color: [0.3, 0.3, 0.3] });
        f(Vertex { position: [x-0.5, y-0.5, z+0.0], color: [0.3, 0.3, 0.3] });

        f(Vertex { position: [x-0.5, y-0.5, z+1.0], color: [0.9, 0.9, 0.9] });
        f(Vertex { position: [x+0.5, y-0.5, z+1.0], color: [0.9, 0.9, 0.9] });
        f(Vertex { position: [x+0.5, y+0.5, z+1.0], color: [0.9, 0.9, 0.9] });
        f(Vertex { position: [x+0.5, y+0.5, z+1.0], color: [0.9, 0.9, 0.9] });
        f(Vertex { position: [x-0.5, y+0.5, z+1.0], color: [0.9, 0.9, 0.9] });
        f(Vertex { position: [x-0.5, y-0.5, z+1.0], color: [0.9, 0.9, 0.9] });
    }

    let mut cube_pos = [
        [0.0, 0.0, 0.0],
        [-1.0, 0.0, 0.0],
        [-2.0, 0.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, -2.0, 0.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 2.0],
    ];
    // this will sort primarily by z
    cube_pos.sort_by(|[_x1, _y1, _z1], [_x2, _y2, _z2]|
        PartialOrd::partial_cmp(_x2, _x1).unwrap());
    cube_pos.sort_by(|[_x1, _y1, _z1], [_x2, _y2, _z2]|
        PartialOrd::partial_cmp(_y2, _y1).unwrap());
    cube_pos.sort_by(|[_x1, _y1, _z1], [_x2, _y2, _z2]|
        PartialOrd::partial_cmp(_z1, _z2).unwrap());

    mod vs {
        vulkano_shaders::shader!{
            ty: "vertex",
            src: "
#version 450

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 color;

layout(location = 0) out vec3 v_color;

void main() {
    v_color = color;

    gl_Position = vec4(
        0.05 * (position.x - position.y),
        0.035 * (- position.x - position.y - 2.0 * position.z),
        0.0,
        1.0
    );
}"
        }
    }

    mod fs {
        vulkano_shaders::shader!{
            ty: "fragment",
            src: "
#version 450

layout(location = 0) in vec3 v_color;
layout(location = 0) out vec4 f_color;

void main() {
    f_color = vec4(v_color, 1.0);
}
"
        }
    }

    let vs = vs::Shader::load(device.clone()).unwrap();
    let fs = fs::Shader::load(device.clone()).unwrap();

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
        // We have to indicate which subpass of which render pass this pipeline
        // is going to be used in. The pipeline will only be usable from this
        // particular subpass.
        .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
        // Now that our builder is filled, we call `build()` to obtain an
        // actual pipeline.
        .build(device.clone())
        .unwrap());

    // Dynamic viewports allow us to recreate just the viewport when the window
    // is resized Otherwise we would have to recreate the whole pipeline.
    let mut dynamic_state = DynamicState { line_width: None, viewports: None, scissors: None };

    let mut framebuffers = window_size_dependent_setup(&images, render_pass.clone(), &mut dynamic_state);

    let mut recreate_swapchain = false;

    // hold it so that we don't block until we want to draw the next frame
    let mut previous_frame_end =
        Box::new(sync::now(device.clone())) as Box<GpuFuture>;

    let mut char_pos = [-1.5, 1.0, 0.0];

    let movement_bindings = Dir2 {
        x: Dir1 {
            pos: winit::VirtualKeyCode::D,
            neg: winit::VirtualKeyCode::A,
        },
        y: Dir1 {
            pos: winit::VirtualKeyCode::W,
            neg: winit::VirtualKeyCode::S,
        },
    };
    let mut movement = Dir2 {
        x: Dir1 {
            pos: false,
            neg: false,
        },
        y: Dir1 {
            pos: false,
            neg: false,
        },
    };

    loop {
        // cleanup unused gpu resources
        previous_frame_end.cleanup_finished();

        // resize
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
        let (image_num, acquire_future) = match swapchain::acquire_next_image(swapchain.clone(), None) {
            Ok(r) => r,
            Err(AcquireError::OutOfDate) => {
                recreate_swapchain = true;
                continue;
            },
            Err(err) => panic!("{:?}", err)
        };

        // color to clear screen with
        let clear_values = vec!([0.0, 0.0, 1.0, 1.0].into());
        let vertex_buffer = {
            // @Performance ideally we would reuse between frames
            let mut cube_vs = Vec::with_capacity(18 * cube_pos.len());
            let mut char_drawn = false;
            let [char_x, char_y, char_z] = char_pos;
            for &[x, y, z] in &cube_pos {
                // draw the character as soon as boxes start being "completely" behind it
                if !char_drawn && z >= char_z && y <= char_y && x <= char_x {
                    cube_vertices(char_x, char_y, char_z, |v| cube_vs.push(v));
                    char_drawn = true;
                }
                cube_vertices(x, y, z, |v| cube_vs.push(v));
            }
            if !char_drawn {
                cube_vertices(char_x, char_y, char_z, |v| cube_vs.push(v));
            }

            vertex_buffer_pool
                .chunk(cube_vs.into_iter())
                .unwrap()
        };

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

        match future {
            Ok(future) => {
                previous_frame_end = Box::new(future) as Box<_>;
            }
            Err(FlushError::OutOfDate) => {
                recreate_swapchain = true;
                previous_frame_end = Box::new(sync::now(device.clone())) as Box<_>;
            }
            Err(e) => {
                println!("{:?}", e);
                previous_frame_end = Box::new(sync::now(device.clone())) as Box<_>;
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
        let mut done = false;
        events_loop.poll_events(|ev| {
            use winit::{Event, WindowEvent, DeviceEvent, KeyboardInput, VirtualKeyCode, ElementState};
            match ev {
                Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => done = true,
                Event::WindowEvent { event: WindowEvent::Resized(_), .. } => recreate_swapchain = true,
                Event::DeviceEvent { event: DeviceEvent::Key (KeyboardInput { virtual_keycode: Some(key), state, .. }), .. } => {
                    if let VirtualKeyCode::Escape = key {
                        done = true;
                    }
                    let val = state == ElementState::Pressed;
                    movement.write_if_eq(&movement_bindings, &key, &val);
                }
                _ => ()
            }
        });
        if done { return; }

        let mvec = movement.dir_vec();
        let mut speed = 0.07;
        if mvec[0] != 0.0 && mvec[1] != 0.0 {
            speed *= 0.7;
        }
        char_pos[0] += speed * (mvec[0] + mvec[1]);
        char_pos[1] += speed * (-mvec[0] + mvec[1]);
    }
}

/// This method is called once during initialization, then again whenever the window is resized
fn window_size_dependent_setup(
    images: &[Arc<SwapchainImage<Window>>],
    render_pass: Arc<RenderPassAbstract + Send + Sync>,
    dynamic_state: &mut DynamicState
) -> Vec<Arc<FramebufferAbstract + Send + Sync>> {
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
