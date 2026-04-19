fn main() {
    // STM32F407VGT6: 1 MB Flash, 192 KB SRAM (128 KB + 64 KB CCM)
    println!("cargo:rerun-if-changed=memory.x");

    // Embed firmware version at build time
    println!(
        "cargo:rustc-env=FIRMWARE_VERSION={}",
        env!("CARGO_PKG_VERSION")
    );
}
