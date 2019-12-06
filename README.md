[![Build Status](https://github.com/emoon/rust_minifb/workflows/CI/badge.svg)](https://github.com/emoon/rust_minifb/actions?workflow=CI)
[![Crates.io](https://img.shields.io/crates/v/minifb.svg)](https://crates.io/crates/minifb)
[![Documentation](https://docs.rs/minifb/badge.svg)](https://docs.rs/minifb)

minifb is a cross platform library written in [Rust](https://www.rust-lang.org) and that makes it easy to setup a window and to (optional) display a 32-bit pixel buffer.  It also makes it easy to get input from keyboard and mouse.
An example is the best way to show how it works:

[Changelog](https://github.com/emoon/rust_minifb/blob/master/CHANGELOG.md)

Usage
-----

```toml
# Cargo.toml
[dependencies]
minifb = "0.14"
```

Example
-------

```rust
extern crate minifb;

use minifb::{Key, Window, WindowOptions};

const WIDTH: usize = 640;
const HEIGHT: usize = 360;

fn main() {
    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];

    let mut window = Window::new(
        "Test - ESC to exit",
        WIDTH,
        HEIGHT,
        WindowOptions::default(),
    )
    .unwrap_or_else(|e| {
        panic!("{}", e);
    });

    while window.is_open() && !window.is_key_down(Key::Escape) {
        for i in buffer.iter_mut() {
            *i = 0; // write something more funny here!
        }

        // We unwrap here as we want this code to exit if it fails. Real applications may want to handle this in a different way
        window
            .update_with_buffer_size(&buffer, WIDTH, HEIGHT)
            .unwrap();
    }
}
```

Status
------
Currently macOS, Redox, Linux and Windows (64-bit and 32-bit) are the current supported platforms. X11 (Linux/FreeBSD/etc) support has been tested on Ubuntu (x64). Bug report(s) for other OSes/CPUs are welcome!


Build instructions
------------------

```
cargo build
cargo run --example noise
```

This will run the [noise example](https://github.com/emoon/rust_minifb/blob/master/examples/noise.rs)

## License

Licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
