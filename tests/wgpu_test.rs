// SPDX-License-Identifier: MPL-2.0

//! Browser smoke test for app_window's wgpu surface integration.
//!
//! wgpu's web backend is unmodified wasm-bindgen code. `scripts/wasm32/tests`
//! substitutes wasm_lite's compatibility shim, launches GPU-enabled Chrome,
//! and runs this through the wasm_lite harness.

#[cfg(not(target_arch = "wasm32"))]
fn main() {}

#[cfg(target_arch = "wasm32")]
wasm_lite::test_main!();

#[cfg(target_arch = "wasm32")]
#[wasm_lite::wasm_lite_test]
fn wgpu_surface_renders_a_frame() {
    wasm_lite_std::async_doctest!(async {
        use futures::FutureExt;
        use some_executor::task::{Configuration, Task};
        use std::panic::AssertUnwindSafe;

        assert!(app_window::application::is_main_thread());
        let (sender, receiver) = r#continue::continuation();

        app_window::application::main(move || {
            app_window::application::submit_to_main_thread("wgpu_test".to_owned(), move || {
                Task::without_notifications(
                    "wgpu_test".to_owned(),
                    Configuration::default(),
                    async move {
                        let outcome = AssertUnwindSafe(render_one_frame()).catch_unwind().await;
                        sender.send(outcome);
                    },
                )
                .spawn_static_current();
            });
        });

        if let Err(panic) = receiver.await {
            std::panic::resume_unwind(panic);
        }
    });
}

#[cfg(target_arch = "wasm32")]
async fn render_one_frame() {
    use app_window::window::Window;
    use wgpu::SurfaceTargetUnsafe;

    assert!(app_window::application::is_main_thread());

    let mut window = Window::default().await;
    let app_surface = window.surface().await;
    let (size, _) = app_surface.size_scale().await;

    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::from_env_or_default());
    let surface = unsafe {
        instance
            .create_surface_unsafe(SurfaceTargetUnsafe::RawHandle {
                raw_display_handle: app_surface.raw_display_handle(),
                raw_window_handle: app_surface.raw_window_handle(),
            })
            .expect("wgpu should create a surface from app_window's raw handles")
    };

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..Default::default()
        })
        .await
        .expect("GPU-enabled Chrome should provide a WebGPU adapter");

    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                .using_resolution(adapter.limits()),
            ..Default::default()
        })
        .await
        .expect("the WebGPU adapter should create a device");

    let config = surface
        .get_default_config(&adapter, size.width() as u32, size.height() as u32)
        .expect("the adapter should support the app_window surface");
    surface.configure(&device, &config);

    let frame = surface
        .get_current_texture()
        .expect("the configured surface should provide a frame");
    let view = frame
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("app_window wgpu smoke test"),
    });
    {
        let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("app_window wgpu smoke test"),
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
            multiview_mask: None,
        });
    }
    queue.submit([encoder.finish()]);
    frame.present();
}
