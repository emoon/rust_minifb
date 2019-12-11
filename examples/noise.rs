extern crate minifb;

use minifb::{Key, Scale, Window, WindowOptions, ScaleMode};

const WIDTH: usize = 640 / 2;
const HEIGHT: usize = 360 / 2;

fn main() {
    let mut noise;
    let mut carry;
    let mut seed = 0xbeefu32;

    let mut window = match Window::new(
        "Noise Test - Press ESC to exit",
        WIDTH,
        HEIGHT,
        WindowOptions {
            resize: true,
            //scale: Scale::X4,
            scale_mode: ScaleMode::UpperLeft,
            ..WindowOptions::default()
        },
    ) {
        Ok(win) => win,
        Err(err) => {
            println!("Unable to create window {}", err);
            return;
        }
    };

    let mut buffer: Vec<u32> = Vec::with_capacity(WIDTH * HEIGHT);

    let mut size = (0, 0);

    //buffer.resize(WIDTH * HEIGHT, 0);

    while window.is_open() && !window.is_key_down(Key::Escape) {
        // div by two as 2x scale
        let new_size = (window.get_size().0, window.get_size().1);
        if new_size != size {
            size = new_size;
            buffer.resize(size.0 * size.1, 0);
            println!("resize");
        }

        for i in buffer.iter_mut() {
            noise = seed;
            noise >>= 3;
            noise ^= seed;
            carry = noise & 1;
            noise >>= 1;
            seed >>= 1;
            seed |= carry << 30;
            noise &= 0xFF;
            *i = (noise << 16) | (noise << 8) | noise;
        }

        window.get_keys().map(|keys| {
            for t in keys {
                match t {
                    Key::W => println!("holding w!"),
                    Key::T => println!("holding t!"),
                    _ => (),
                }
            }
        });

        //println!("new size {:?}", new_size);

        // We unwrap here as we want this code to exit if it fails
        window.update_with_buffer(&buffer, new_size.0, new_size.1).unwrap();
        //window.update_with_buffer(&buffer, WIDTH, HEIGHT).unwrap();
    }
}
