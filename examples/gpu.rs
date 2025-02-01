/*! An example for wgpu.
*/
#[cfg(feature = "wgpu")]
mod gpu {
    use std::borrow::Cow;
    use std::sync::{Arc, Mutex};
    use some_executor::hint::Hint;
    use some_executor::observer::Observer;
    use some_executor::{Priority, SomeExecutor};
    use some_executor::task::Configuration;
    use wgpu::{Device, Queue, SurfaceTargetUnsafe};
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

    #[cfg(feature = "wgpu")]
    async fn wgpu_run(mut window: Window) {
        logwise::warn_sync!("main_run");
        let mut app_surface = window.surface().await;
        let ( sender,mut receiver) = ampsc::channel();
        let size = app_surface.size().await;
        let latest_size = Arc::new(Mutex::new(size));
        let move_latest_size = latest_size.clone();
        app_surface.size_update(move |size| {
            let mut update_sender = sender.clone();
            let mut some_executor = some_executor::current_executor::current_executor();
            //it's nice to do this inline so that if we get many size updates back-to-back the last one wins
            *move_latest_size.lock().unwrap() = size;
            println!("got size update {:?}", size);
            let task = some_executor::task::Task::new_objsafe("resize".into(), Box::new(async move {
                update_sender.send(Message::SizeChanged).await.unwrap();
                update_sender.async_drop().await;
                Box::new(()) as Box<dyn std::any::Any + Send>
            }),Configuration::new(Hint::CPU, Priority::UserInteractive, some_executor::Instant::now()),None);
            some_executor.spawn_objsafe(task).detach();
        });
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::from_env_or_default());

        let surface = unsafe{instance.create_surface_unsafe(SurfaceTargetUnsafe::RawHandle {
            raw_display_handle: app_surface.raw_display_handle(),
            raw_window_handle: app_surface.raw_window_handle(),
        })}.expect("Can't create surface");
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                // Request an adapter which can render to our surface
                compatible_surface: Some(&surface),
            }).await.expect("Can't create adapter");
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
            .expect("Can't configure");
        surface.configure(&device, &config);


        let state = State {
            surface,
            device,
            queue,
            render_pipeline,
        };
        render(&state);



        enum Message {
            SizeChanged,
        }

        loop {
            let msg = receiver.receive().await;
            match msg {
                Ok(Message::SizeChanged) => {
                    let new_size = *latest_size.lock().unwrap();
                    config.width = new_size.width() as u32;
                    config.height = new_size.height() as u32;
                    state.surface.configure(&state.device, &config);
                    render(&state);
                }
                Err(e) => {
                    panic!("Error receiving message: {:?}", e);
                },
            }
        }

    }
    pub fn main() {
        app_window::application::main(|| {
            let task = some_executor::task::Task::without_notifications("run".into(), async {
                let window = Window::default().await;
                #[cfg(feature = "wgpu")] {
                    app_window::application::on_main_thread(|| {
                        app_window::wgpu::wgpu_spawn(async move {
                            wgpu_run(window).await;
                        })
                    }).await;

                }

                #[cfg(not(feature = "wgpu"))]
                panic!("wgpu feature not enabled");
            }, Configuration::new(Hint::Unknown, Priority::UserInteractive, some_executor::Instant::now()));
            some_executor::current_executor::current_executor().spawn_objsafe(task.into_objsafe()).detach();

        });
    }
}




pub fn main() {
    #[cfg(target_arch = "wasm32")]
    {
        console_error_panic_hook::set_once();
    }

    #[cfg(feature = "wgpu")]
    gpu::main();
    #[cfg(not(feature = "wgpu"))]
    panic!("wgpu feature not enabled");






}