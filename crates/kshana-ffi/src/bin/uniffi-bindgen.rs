//! `cargo run -p kshana-ffi --bin uniffi-bindgen -- generate --library <libkshana_ffi.so> --language kotlin --out-dir <dir>`
fn main() {
    uniffi::uniffi_bindgen_main()
}
