rust_minifb
======

rust_minifb (Mini FrameBuffer) is a small cross platform library written in [Rust](https://www.rust-lang.org) and that makes it easy to render (32-bit) pixels in a window. An example is the best way to show how it works:


```rust
extern crate minifb;

use minifb::*;

const WIDTH: usize = 640;
const HEIGHT: usize = 360;

fn main() {
    let mut buffer: [u32; WIDTH * HEIGHT] = [0; WIDTH * HEIGHT];

    let mut window = Window::new("Noise Test - Press ESC to exit",
                                 WIDTH,
                                 HEIGHT,
                                 Scale::X1,
                                 Vsync::No)
                         .unwrap();

    while window.is_open() && !window.is_key_down(Key::Escape) {
        for i in buffer.iter_mut() {
            *i = 0; // write something more funny here!
        }

        window.update(&buffer);
    }
}
```

Status
------
Currently Windows and Mac are the current supported platforms. X11 (Linux/FreeBSD/etc) support is coming soon.


Build instruction
------------------

```
cargo build
cargo run --example noise 
```

This will run the [noise example](https://github.com/emoon/rust_minifb/blob/windows-rs/examples/noise.rs) which should look something like this (Mac screenshot)

![mac_screenshot](https://dl.dropboxusercontent.com/u/5205843/rust_minifb/noise_screen.png)
