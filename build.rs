//! This build script copies the `memory.x` linker script for the selected
//! board (`rp2350/memory.x`, or `qemu/memory.x` with the qemu feature) into a
//! directory where the linker can always find it at build time. They can't
//! live in the crate root, as the linker always searches the project root
//! directory -- wherever `Cargo.toml` is -- first, so a `memory.x` there would
//! always be used. Additionally, by requesting that Cargo re-run the build
//! script whenever `memory.x` is changed, updating `memory.x` ensures a
//! rebuild of the application with the new memory settings.

use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

fn main() {
    // Put the linker script somewhere the linker can find it
    let out = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    println!("cargo:rustc-link-search={}", out.display());

    // The file `memory.x` is loaded by cortex-m-rt's `link.x` script, which
    // is what we specify in `.cargo/config.toml` for Arm builds
    if std::env::var_os("CARGO_FEATURE_QEMU").is_some() {
        // QEMU's mps2-an521 machine has a different memory map, and there's no
        // PAC to provide `device.x` (the interrupt handler names)
        let memory_x = include_bytes!("qemu/memory.x");
        let mut f = File::create(out.join("memory.x")).unwrap();
        f.write_all(memory_x).unwrap();
        let device_x = include_bytes!("qemu/device.x");
        let mut f = File::create(out.join("device.x")).unwrap();
        f.write_all(device_x).unwrap();
    } else {
        let memory_x = include_bytes!("rp2350/memory.x");
        let mut f = File::create(out.join("memory.x")).unwrap();
        f.write_all(memory_x).unwrap();
    }
    println!("cargo:rerun-if-changed=rp2350/memory.x");
    println!("cargo:rerun-if-changed=qemu/memory.x");
    println!("cargo:rerun-if-changed=qemu/device.x");

    println!("cargo:rerun-if-changed=build.rs");
}
