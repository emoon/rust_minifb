use std::env;
extern crate cc;

fn main() {
    let env = env::var("TARGET").unwrap();
    if env.contains("darwin") {
        cc::Build::new()
            .file("src/native/macosx/MacMiniFB.m")
            .file("src/native/macosx/OSXWindow.m")
            .file("src/native/macosx/OSXWindowFrameView.m")
            .compile("libminifb_native.a");
    } else if env.contains("linux") {
        cc::Build::new()
            .file("src/native/x11/X11MiniFB.c")
            .compile("libminifb_native.a");
    }
}
