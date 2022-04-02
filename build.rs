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
    let target = env::var("TARGET").unwrap_or("".to_string());
    if target != "wasm32-unknown-unknown"
        && cfg!(not(any(
            target_os = "macos",
            target_os = "windows",
            target_os = "redox",
            target_arch = "wasm32", // this is ignored. Why?
        )))
        && cfg!(not(any(feature = "wayland", feature = "x11")))
    {
        panic!("At least one of the x11 or wayland features must be enabled");
    }

    let env = env::var("TARGET").unwrap();
    if env.contains("darwin") {
        cc::Build::new()
            .flag("-mmacosx-version-min=10.10")
            .file("src/native/macosx/MacMiniFB.m")
            .file("src/native/macosx/OSXWindow.m")
            .file("src/native/macosx/OSXWindowFrameView.m")
            .compile("libminifb_native.a");
        println!("cargo:rustc-link-lib=framework=Metal");
        println!("cargo:rustc-link-lib=framework=MetalKit");
    } else if !env.contains("windows") && !env.contains("wasm32") {
        // build scalar on non-windows and non-mac
        cc::Build::new()
            .file("src/native/posix/scalar.cpp")
            .opt_level(3) // always build with opts for scaler so it's fast in debug also
            .compile("libscalar.a")
    }
}
