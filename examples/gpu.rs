// SPDX-License-Identifier: MPL-2.0
/*! An example for wgpu.
*/
logwise::declare_logging_domain!();

mod gpu {
    use app_window::window::Window;
    use some_executor::hint::Hint;
    use some_executor::observer::Observer;
    use some_executor::task::Configuration;
    use some_executor::{Priority, SomeExecutor};
    use std::borrow::Cow;
    use std::sync::{Arc, Mutex};

    use app_window::{WGPU_SURFACE_STRATEGY, WGPUStrategy};
    use wgpu::{Device, Queue, SurfaceTargetUnsafe};

    #[cfg(not(target_arch = "wasm32"))]
    async fn use_strategy<C, R>(strategy: WGPUStrategy, for_closure: C) -> R
    where
        C: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        match strategy {
            WGPUStrategy::Relaxed => for_closure(),
            WGPUStrategy::MainThread => {
                let f = app_window::application::on_main_thread(
                    "use_strategy".to_string(),
                    move || for_closure(),
                )
                .await;
                f
            }
            WGPUStrategy::NotMainThread => {
                if app_window::application::is_main_thread() {
                    todo!()
                } else {
                    //effectively relaxed
                    for_closure()
                }
            }
            _ => todo!("Unsupported WGPU strategy: {:?}", strategy),
        }
    }

    #[cfg(target_arch = "wasm32")]
    async fn use_strategy<C, R>(strategy: WGPUStrategy, for_closure: C) -> R
    where
        C: FnOnce() -> R,
    {
        match strategy {
            WGPUStrategy::Relaxed => for_closure(),
            WGPUStrategy::MainThread => {
                assert!(app_window::application::is_main_thread());
                for_closure()
            }
            WGPUStrategy::NotMainThread => {
                if app_window::application::is_main_thread() {
                    todo!()
                } else {
                    //effectively relaxed
                    for_closure()
                }
            }
            _ => todo!("Unsupported WGPU strategy: {:?}", strategy),
        }
    }

    enum Message {
        SizeChanged,
    }

    struct State<'window> {
        surface: wgpu::Surface<'window>,
        device: Device,
        queue: Queue,
        render_pipeline: wgpu::RenderPipeline,
    }

    fn render(state: &State) {
        //render a frame
        let frame = state
            .surface
            .get_current_texture()
            .expect("Failed to acquire next swap chain texture");
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = state
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
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

    async fn wgpu_run(mut window: Window) {
        logwise::warn_sync!("main_run");
        let mut app_surface = window.surface().await;
        let (sender, mut receiver) = ampsc::channel();
        let (size, _scale) = app_surface.size_scale().await;
        let latest_size = Arc::new(Mutex::new(size));
        let move_latest_size = latest_size.clone();
        app_surface.size_update(move |size| {
            let mut update_sender = sender.clone();
            let mut some_executor = some_executor::current_executor::current_executor();
            //it's nice to do this inline so that if we get many size updates back-to-back the last one wins
            *move_latest_size.lock().unwrap() = size;
            println!("got size update {:?}", size);
            let task = some_executor::task::Task::new_objsafe(
                "resize".into(),
                Box::new(async move {
                    update_sender.send(Message::SizeChanged).await.unwrap();
                    update_sender.async_drop().await;
                    Box::new(()) as Box<dyn std::any::Any + Send>
                }),
                Configuration::new(
                    Hint::CPU,
                    Priority::UserInteractive,
                    some_executor::Instant::now(),
                ),
                None,
            );
            some_executor.spawn_objsafe(task).detach();
        });

        let app_surface = Arc::new(app_surface);
        let move_surface = app_surface.clone();
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::from_env_or_default());

        let move_instance = instance.clone();
        let surface = use_strategy(WGPU_SURFACE_STRATEGY, move || unsafe {
            move_instance
                .create_surface_unsafe(SurfaceTargetUnsafe::RawHandle {
                    raw_display_handle: move_surface.raw_display_handle(),
                    raw_window_handle: move_surface.raw_window_handle(),
                })
                .expect("Can't create surface")
        })
        .await;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                // Request an adapter which can render to our surface
                compatible_surface: Some(&surface),
            })
            .await
            .expect("Can't create adapter");
        // Create the logical device and command queue
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                // Make sure we use the texture resolution limits from the adapter, so we can support images the size of the swapchain.
                required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                    .using_resolution(adapter.limits()),
                memory_hints: wgpu::MemoryHints::MemoryUsage,
                trace: wgpu::Trace::default(),
                experimental_features: wgpu::ExperimentalFeatures::default(),
            })
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
                }
            }
        }
    }
    pub fn main() {
        //set up main thread
        app_window::application::main(|| match app_window::WGPU_STRATEGY {
            WGPUStrategy::MainThread => {
                app_window::application::submit_to_main_thread("gpu".to_owned(), || {
                    some_executor::task::Task::without_notifications(
                        "gpu_main".to_string(),
                        Configuration::default(),
                        async {
                            wgpu_run(Window::default().await).await;
                        },
                    )
                    .spawn_static_current()
                });
            }
            WGPUStrategy::NotMainThread => {
                std::thread::Builder::new()
                    .name("gpu_main".to_string())
                    .spawn(|| {
                        some_executor::task::Task::without_notifications(
                            "gpu_main".to_string(),
                            Configuration::default(),
                            async {
                                wgpu_run(Window::default().await).await;
                            },
                        )
                        .spawn_static_current()
                    })
                    .unwrap();
            }
            WGPUStrategy::Relaxed => some_executor::task::Task::without_notifications(
                "gpu_main".to_string(),
                Configuration::default(),
                async {
                    wgpu_run(Window::default().await).await;
                },
            )
            .spawn_static_current(),
            _ => {
                panic!("Unsupported WGPU strategy: {:?}", app_window::WGPU_STRATEGY);
            }
        });
    }
}

pub fn main() {
    #[cfg(target_arch = "wasm32")]
    {
        console_error_panic_hook::set_once();
    }

    gpu::main();
}
