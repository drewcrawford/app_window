CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER="wasm-server-runner" \
RUSTFLAGS='-C target-feature=+atomics,+bulk-memory,+mutable-globals' \
cargo +nightly run --example gpu --target wasm32-unknown-unknown \
--features wgpu \
-Z build-std=std,panic_abort