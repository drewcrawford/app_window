use std::borrow::Cow;
use std::sync::Arc;
use wgpu::{Device, Queue, SurfaceTargetUnsafe};
use app_window::application::on_main_thread;
use app_window::window::Window;

struct State<'window> {
    surface: wgpu::Surface<'window>,
    device: Device,
    queue: Queue,
    render_pipeline: wgpu::RenderPipeline,

}

fn render(state: &State) {
    //render a frame
    let frame = state.surface
        .get_current_texture()
        .expect("Failed to acquire next swap chain texture");
    let view = frame
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder =
        state.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: None,
        });
    {
        let mut rpass =
            encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::GREEN),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        rpass.set_pipeline(&state.render_pipeline);
        rpass.draw(0..3, 0..1);
    }

    state.queue.submit(Some(encoder.finish()));
    frame.present();
}

async fn run(window: Window) {
    logwise::warn_sync!("gpu::run");
    let mut app_surface = window.surface().await;
    logwise::warn_sync!("SURFACE CREATED; will return");
    return;

    let size = app_surface.size().await;
    let instance = Arc::new(wgpu::Instance::default());


    let app_surface_arc = Arc::new(app_surface);
    let app_surface_extra = app_surface_arc.clone();
    let surface = app_surface_extra.create_wgpu_surface(&instance).await;
    let mut app_surface = Arc::into_inner(app_surface_arc).expect("Can't get app surface");
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            force_fallback_adapter: false,
            // Request an adapter which can render to our surface
            compatible_surface: Some(&surface),
        })
        .await
        .expect("Failed to find an appropriate adapter");

    // Create the logical device and command queue
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                // Make sure we use the texture resolution limits from the adapter, so we can support images the size of the swapchain.
                required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                    .using_resolution(adapter.limits()),
                memory_hints: wgpu::MemoryHints::MemoryUsage,
            },
            None,
        )
        .await
        .expect("Failed to create device");


    // Load the shaders from disk
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[],
        push_constant_ranges: &[],
    });

    let swapchain_capabilities = surface.get_capabilities(&adapter);
    let swapchain_format = swapchain_capabilities.formats[0];

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: Default::default(),
            targets: &[Some(swapchain_format.into())],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    });

    let mut config = surface
        .get_default_config(&adapter, size.width() as u32, size.height() as u32)
        .unwrap();
    surface.configure(&device, &config);

    let state = State {
        surface,
        device,
        queue,
        render_pipeline,
    };

    render(&state);

    let (sender,receiver) = std::sync::mpsc::channel();

    enum Message {
        SizeChanged(app_window::coordinates::Size),
    }
    app_surface.size_update(move |size| {
        sender.send(Message::SizeChanged(size)).unwrap();
    });

    while let Ok(msg) = receiver.recv() {
        match msg {
            Message::SizeChanged(new_size) => {
                config.width = new_size.width() as u32;
                config.height = new_size.height() as u32;
                state.surface.configure(&state.device, &config);
                render(&state);
            }
        }
    }





    std::mem::forget(window);

    // let window = &window;

    // event_loop
    //     .run(move |event, target| {
    //         // Have the closure take ownership of the resources.
    //         // `event_loop.run` never returns, therefore we must do this to ensure
    //         // the resources are properly cleaned up.
    //         let _ = (&instance, &adapter, &shader, &pipeline_layout);
    //
    //         if let Event::WindowEvent {
    //             window_id: _,
    //             event,
    //         } = event
    //         {
    //             match event {
    //                 WindowEvent::Resized(new_size) => {
    //                     // Reconfigure the surface with the new size
    //                     config.width = new_size.width.max(1);
    //                     config.height = new_size.height.max(1);
    //                     surface.configure(&device, &config);
    //                     // On macos the window needs to be redrawn manually after resizing
    //                     window.request_redraw();
    //                 }
    //                 WindowEvent::RedrawRequested => {
    //                     let frame = surface
    //                         .get_current_texture()
    //                         .expect("Failed to acquire next swap chain texture");
    //                     let view = frame
    //                         .texture
    //                         .create_view(&wgpu::TextureViewDescriptor::default());
    //                     let mut encoder =
    //                         device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
    //                             label: None,
    //                         });
    //                     {
    //                         let mut rpass =
    //                             encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
    //                                 label: None,
    //                                 color_attachments: &[Some(wgpu::RenderPassColorAttachment {
    //                                     view: &view,
    //                                     resolve_target: None,
    //                                     ops: wgpu::Operations {
    //                                         load: wgpu::LoadOp::Clear(wgpu::Color::GREEN),
    //                                         store: wgpu::StoreOp::Store,
    //                                     },
    //                                 })],
    //                                 depth_stencil_attachment: None,
    //                                 timestamp_writes: None,
    //                                 occlusion_query_set: None,
    //                             });
    //                         rpass.set_pipeline(&render_pipeline);
    //                         rpass.draw(0..3, 0..1);
    //                     }
    //
    //                     queue.submit(Some(encoder.finish()));
    //                     frame.present();
    //                 }
    //                 WindowEvent::CloseRequested => target.exit(),
    //                 _ => {}
    //             };
    //         }
    //     })
    //     .unwrap();
}

pub fn main() {
    #[cfg(target_arch = "wasm32")]
    {
        console_error_panic_hook::set_once();
    }
    logwise::warn_sync!("gpu::main");

    app_window::application::main(|| {
        logwise::warn_sync!("gpu::after_main");
        let w = Window::default();
        logwise::warn_sync!("gpu::spawn_local");

        test_executors::spawn_local(run(w), "gpu_main");
    });




}