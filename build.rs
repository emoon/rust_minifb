use std::env;
extern crate cc;

//cargo build --target=wasm32-unknown-unknown --verbose --no-default-features --features web

fn main() {
    /*
    println!("Environment configuration:");
    for (key, value) in env::vars() {
        if key.starts_with("CARGO_CFG_") {
            println!("{}: {:?}", key, value);
        }
    }
    println!("OS: {:?}", env::var("OS").unwrap_or("".to_string()));
    println!("FAMILY: {:?}", env::var("FAMILY").unwrap_or("".to_string()));
    println!("ARCH: {:?}", env::var("ARCH").unwrap_or("".to_string()));
    println!("TARGET: {:?}", env::var("TARGET").unwrap_or("".to_string()));
    */
    // target_arch is not working? OS FAMILY and ARCH variables were empty too
    // I think the cross-compilation is broken. We could take these from the environment,
    // since the build script seems to have a different target_arch than the destination.
    let target = env::var("TARGET").expect("cargo should have set $TARGET");
    if target != "wasm32-unknown-unknown"
        && !target.contains("-macos")
		&& !target.contains("-windows")
		&& !target.contains("-redox")
		&& !target.starts_with("wasm32-")	// this is ignored. Why?
        && cfg!(not(any(feature = "wayland", feature = "x11")))
    {
        panic!("At least one of the x11 or wayland features must be enabled");
    }

    // `cc` does not emit these itself, so without them cargo keeps a stale
    // object file when only the native sources change. Emitting any of these
    // also switches cargo off watching the whole package, so the headers have
    // to be listed too.
    for source in [
        "src/native/macosx/MacMiniFB.m",
        "src/native/macosx/OSXWindow.h",
        "src/native/macosx/OSXWindow.m",
        "src/native/macosx/OSXWindowFrameView.h",
        "src/native/macosx/OSXWindowFrameView.m",
        "src/native/macosx/shared_data.h",
        "src/native/posix/scalar.c",
    ] {
        println!("cargo:rerun-if-changed={}", source);
    }

    if target.contains("darwin") {
        cc::Build::new()
            .flag("-mmacosx-version-min=10.11")
            .file("src/native/macosx/MacMiniFB.m")
            .file("src/native/macosx/OSXWindow.m")
            .file("src/native/macosx/OSXWindowFrameView.m")
            .compile("libminifb_native.a");
        println!("cargo:rustc-link-lib=framework=Metal");
        println!("cargo:rustc-link-lib=framework=MetalKit");
    } else if !target.contains("windows") && !target.contains("wasm32") {
        // build scalar on non-windows and non-mac
        cc::Build::new()
            .file("src/native/posix/scalar.c")
            .opt_level(3) // always build with opts for scaler so it's fast in debug also
            .compile("libscalar.a")
    }
}
