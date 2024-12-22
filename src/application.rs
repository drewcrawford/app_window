/**
Performs the runloop or event loop.

Call this function exactly once, from the first thread in your program.

On most platforms, this function parks the thread, possibly in a platform-specific way to receive UI events.

On platforms like wasm, this function may return immediately.

# Discussion

On many platforms, UI needs some kind of application-wide runloop or event loop.  Calling this function
turns the current thread into that runloop (on platforms where this is necessary).

Many platforms, such as macOS, require that the first thread created by the application perform the runloop
(you can't do it on an arbitrary thread).  Accordingly on all platforms, require this function to be called
from the first thread.


*/
pub fn main() {
    todo!()
}

