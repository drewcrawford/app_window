use swift_rs::swift;

#[allow(non_snake_case)]
swift!(fn SwiftAppWindowIsMainThread() -> bool);

#[allow(non_snake_case)]
swift!(fn SwiftAppWindowRunMainThread());

pub fn is_main_thread() -> bool {
    unsafe{SwiftAppWindowIsMainThread()}
}

pub fn run_main_thread() {
    unsafe { SwiftAppWindowRunMainThread() }
}