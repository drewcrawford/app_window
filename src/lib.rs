/*!
A cross-platform window crate.  Alternative to winit.

The main goal of this project is to provide a cross-platform API to bring up a window (or appropriate
platform surface) for rendering application or game content.  (The content is out of scope for
this crate, but the idea is to use wgpu or a toolkit like GTK for content.)

"Appropriate" means we use the same APIs that native developers expect, and provide the same appearance and behaviors
that native users expect, where these expectations differ among platforms.

Some other goals of the project are:
* Write code against this crate that runs anywhere.  In particular, support platforms with *very*
  different threading requirements without requiring the application developer to do anything special.
* Use modern backends, like Wayland, ignoring legacy backends like X11
* Provide APIs to spawn code onto the main thread, and even a built-in executor to spawn futures there.
* Optional support for wgpu.

# Cargo features
* `some_executor` - Provides interop with the `some-executor` crate.
* `wgpu` - Helper functions for creating a wgpu surface.
* `app_input` - Created windows are configured to receive input via [`app_input`](https://sealedabstract.com/code/app_input) crate.
*/

pub mod window;
pub mod application;
mod sys;
pub mod coordinates;
mod surface;
pub mod executor;
#[cfg(feature = "some_executor")]
pub mod some_executor;

#[cfg(feature = "wgpu")]
pub mod wgpu;