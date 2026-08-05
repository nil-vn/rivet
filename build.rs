//! Build script for generated artifacts introduced by later work packages.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
}
