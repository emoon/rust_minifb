rust_minifb
======

rust_mini (Mini FrameBuffer) is a small cross platform library written in Rust and that makes it easy to render (32-bit) pixels in a window. An example is the best way to show how it works:

```rust
extern crate minifb;

const WIDTH: usize = 1280;
const HEIGHT: usize = 720;

fn main() {
    let mut buffer: [u32; WIDTH * HEIGHT] = [0; WIDTH * HEIGHT];

    if !(minifb::open("TestWindow", WIDTH, HEIGHT)) {
        return;
    }

    while minifb::update(&buffer) {
        for i in buffer.iter_mut() {
            *i = ... // write something here 
        }
    }

    minifb::close();
}
```

Build instructions
------------------

```
cargo build

cargo run --example noise to test the noise example which should look something like this (Mac screenshot)
```

![mac_screenshot](https://dl.dropboxusercontent.com/u/5205843/rust_minifb/noise_screen.png)
