//! Embed the Windows application manifest, and do nothing else ever.
//!
//! This is the crate's only build script and `DESIGN.md` §2 is why it is worth
//! reading before adding a second thing to it. That section's rule is that
//! nothing compiles C and that a build needs a Rust toolchain and nothing else.
//! This holds to both: it prints two linker arguments and the linker that was
//! already linking the binary embeds
//! `packaging/windows/slipcase-desktop.manifest`. No resource compiler, no
//! object file, nothing compiled that was not compiled before.
//!
//! The distinction matters because the obvious way to do this is `rc.exe` or
//! `windres`, and `packaging/windows/README.md` rejected exactly that when the
//! window icon wanted a resource — which is why the icon travels through
//! `include_bytes!`. That rejection was of the resource compiler and not of the
//! outcome, and the linker route needs no compiler.
//!
//! **A second use for this file is a decision, not a precedent.** The one
//! opened here is narrow on purpose.
//!
//! Author: David M. Anderson
//! Built with AI assistance (Claude, Anthropic)

#![forbid(unsafe_code)]

use std::path::Path;

fn main() {
    // The manifest lives with the rest of this platform's files rather than at
    // the crate root, which is the same rule as staying inside your own
    // directory under `packaging/`.
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("packaging")
        .join("windows")
        .join("slipcase-desktop.manifest");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", manifest.display());

    // Read from the environment rather than from `cfg!`, because a build script
    // is compiled for the host and `cfg!(windows)` in here answers about the
    // machine doing the building. Cross-checking from Linux with
    // `--target x86_64-pc-windows-msvc` is a thing this repository does, and it
    // would take the wrong branch.
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if os != "windows" || env != "msvc" {
        return;
    }

    // `/MANIFEST:EMBED` is MSVC's, which is why the guard above tests the
    // environment and not just the operating system: a `windows-gnu` target
    // links with something that would not understand it.
    //
    // Named rather than `-bins`, because the package builds two binaries and
    // `corpus` is a console runner that has no window to be aware about.
    for arg in ["/MANIFEST:EMBED", &format!("/MANIFESTINPUT:{}", manifest.display())] {
        println!("cargo:rustc-link-arg-bin=slipcase-desktop={arg}");
    }
}
