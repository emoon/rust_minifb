use std::env;
extern crate gcc;

fn main() {
    let env = env::var("TARGET").unwrap();
    if env.contains("darwin") {
        gcc::compile_library("libminifb_native.a",
                             &["src/native/macosx/MacMiniFB.m",
                               "src/native/macosx/OSXWindow.m",
                               "src/native/macosx/OSXWindowFrameView.m"]);   // MacOS
    // } else if env.contains("windows") {
    //    gcc::compile_library("libminifb_native.a", &["src/native/windows/WinMiniFB.c"]);   // Windows
    } else if env.contains("linux") {
        gcc::compile_library("libminifb_native.a", &["src/native/x11/X11MiniFB.c"]);   // Unix
    }
}
