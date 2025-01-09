use std::borrow::Cow;
use some_executor::hint::Hint;
use some_executor::observer::Observer;
use some_executor::{Priority, SomeExecutor};
use some_executor::task::Configuration;
use wgpu::{Device, Queue};
use app_window::application::{on_main_thread};
use app_window::executor::{already_on_main_thread_submit};
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
async fn main_run(mut window: Window) {
    logwise::warn_sync!("main_run");
    let mut app_surface = window.surface().await;
    let ( sender,mut receiver) = ampsc::channel();
    app_surface.size_update(move |size| {
        let mut update_sender = sender.clone();
        let mut some_executor = some_executor::current_executor::current_executor();
        let task = some_executor::task::Task::new_objsafe("resize".into(), Box::new(async move {
            update_sender.send(Message::SizeChanged(size)).await.unwrap();
            update_sender.async_drop().await;
            Box::new(()) as Box<dyn std::any::Any + Send>
        }),Configuration::new(Hint::CPU, Priority::UserInteractive, some_executor::Instant::now()),None);
        some_executor.spawn_objsafe(task).detach();
    });
    let instance = wgpu::Instance::new(&wgpu::util::instance_descriptor_from_env());

    let surface = app_surface.create_wgpu_surface(&instance).expect("Can't create surface");
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            force_fallback_adapter: false,
            // Request an adapter which can render to our surface
            compatible_surface: Some(&surface),
        })
        .await
        .expect("Failed to find an appropriate adapter");
    let size = app_surface.size().await;
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
        SizeChanged(app_window::coordinates::Size),
    }


    while let Ok(msg) = receiver.receive().await {
        match msg {
            Message::SizeChanged(new_size) => {
                config.width = new_size.width() as u32;
                config.height = new_size.height() as u32;
                state.surface.configure(&state.device, &config);
                render(&state);
            }
        }
    }
}

async fn run(window: Window) {

    //on wasm32, we primarily need to operate on the main thread
    on_main_thread(|| {
        #[cfg(feature = "wgpu")]
        already_on_main_thread_submit(main_run(window));
        #[cfg(not(feature = "wgpu"))]
        panic!("wgpu feature not enabled");
    }).await;
}

pub fn main() {
    #[cfg(target_arch = "wasm32")]
    {
        console_error_panic_hook::set_once();
    }

    app_window::application::main(|| {
        let task = some_executor::task::Task::without_notifications("run".into(), async {
            let window = Window::default().await;
            run(window).await;
        }, Configuration::new(Hint::Unknown, Priority::UserInteractive, some_executor::Instant::now()));
        some_executor::current_executor::current_executor().spawn_objsafe(task.into_objsafe()).detach();

    });




}