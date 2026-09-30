// Build script: records a few facts about how this binary was built so that
// `img2ico --version` can print them. They are what a bug report needs to
// reproduce a problem (which platform, which compiler), and only the build
// itself can tell - at run time nothing knows which compiler produced the
// binary.
//
// Cargo passes each `cargo:rustc-env=NAME=value` line below to the compiler,
// where `env!("NAME")` turns it into a compile-time string constant (see
// LONG_VERSION in src/cli.rs).

use std::process::Command;

fn main() {
    // The platform the binary is built FOR, e.g. "x86_64-pc-windows-msvc".
    let target = std::env::var("TARGET").unwrap_or_else(|_| "unknown".to_string());

    // The compiler's own version line, e.g. "rustc 1.97.1 (8bab26f4f 2026-07-14)".
    // Cargo tells us which compiler it is using through the RUSTC variable.
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
    let compiler = Command::new(rustc)
        .arg("--version")
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=IMG2ICO_TARGET={target}");
    println!("cargo:rustc-env=IMG2ICO_COMPILER={compiler}");

    // Nothing else can change these values, so there is no need to re-run
    // this script on every build.
    println!("cargo:rerun-if-changed=build.rs");
}
