fn main() {
    println!("cargo:rerun-if-changed=memory.x");
    println!("cargo:rustc-link-arg=-Tmemory.x");

    // Embed firmware version at build time
    println!(
        "cargo:rustc-env=FIRMWARE_VERSION={}",
        env!("CARGO_PKG_VERSION")
    );
}
