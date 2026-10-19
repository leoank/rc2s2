// Thin hand-written "sys" build script: link a prebuilt libtailscale.a.
// The Go archive is built by the Nix derivation; this never runs `go`.
fn main() {
    let dir = std::env::var("LIBTAILSCALE_LIB_DIR")
        .expect("LIBTAILSCALE_LIB_DIR must point at the directory holding libtailscale.a");
    println!("cargo:rerun-if-env-changed=LIBTAILSCALE_LIB_DIR");
    println!("cargo:rustc-link-search=native={dir}");
    println!("cargo:rustc-link-lib=static=tailscale");

    let target = std::env::var("TARGET").unwrap();
    if target.contains("apple") {
        // Go's runtime and net package on darwin need these.
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
        println!("cargo:rustc-link-lib=framework=Security");
    }
}
