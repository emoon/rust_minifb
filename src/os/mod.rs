#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "netbsd",
    target_os = "openbsd"
))]
pub mod posix;
#[cfg(target_os = "redox")]
pub mod redox;
#[cfg(target_os = "windows")]
pub mod windows;
#[cfg(target_arch = "wasm32")]
pub mod wasm;