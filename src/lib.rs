/*!
A cross-platform window crate.  Alternative to winit.

The main goal of this project is to provide a cross-platform API to bring up a window (or appropriate
platform surface) for rendering application or game content.  (The content is out of scope for
this crate, but the idea is to use wgpu or a toolkit like GTK for content.)  "Appropriate" means we
use the same APIs that native developers expect, and provide the same appearance and behaviors
that native users expect, where these expectations differ among platforms.

Some other goals of the project are:
* Use modern backends, like Wayland, ignoring legacy backends like X11
* Design thoughtful APIs that work well everywhere, especially on "odd" platforms like wasm and macOS


*/

pub mod window;
pub mod application;
mod sys;
pub mod coordinates;
mod surface;
pub mod executor;
#[cfg(feature = "some_executor")]
pub mod some_executor;