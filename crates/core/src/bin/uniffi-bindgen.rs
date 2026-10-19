//! Generates the foreign bindings (Kotlin for the Android app) from the built library:
//! `cargo run -p rc2s2-core --bin uniffi-bindgen -- generate --library <lib> --language kotlin --out-dir <dir>`.
fn main() {
    uniffi::uniffi_bindgen_main()
}
