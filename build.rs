use std::env;
extern crate cc;

fn main() {
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
	}
	else if !env.contains("windows") {
		// build scalar on non-windows and non-mac
		cc::Build::new()
			.file("src/native/unix/scalar.cpp")
			.opt_level(3) // always build with opts for scaler so it's fast in debug also
			.compile("libscalar.a")
	}
}
