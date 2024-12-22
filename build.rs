fn main() {
    #[cfg(target_os="macos")] {
        let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
        if target_os == "macos" {
            use swift_rs::SwiftLinker;
            println!("cargo:rustc-env=MACOSX_DEPLOYMENT_TARGET=12");
            SwiftLinker::new("12")
                .with_package("SwiftAppWindow", "SwiftAppWindow")
                .link();
        }
    }
}