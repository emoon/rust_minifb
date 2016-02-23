extern crate minifb;

use minifb::{Window, Key, Scale, WindowOptions, Menu};

const WIDTH: usize = 640;
const HEIGHT: usize = 360;

fn main() {
    let mut noise;
    let mut carry;
    let mut seed = 0xbeefu32;

    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];

    let mut window = Window::new("Noise Test - Press ESC to exit",
                                 WIDTH,
                                 HEIGHT,
                                 WindowOptions {
                                     resize: true,
                                     scale: Scale::X2,
                                     ..WindowOptions::default()
                                 })
                         .expect("Unable to Open Window");

    let menu = vec![Menu {
                        name: "Some Menu",
                        key: Key::R,
                        id: 2,
                        ..Menu::default()
                    },
                    Menu::separotor(),
                    Menu {
                        name: "Some other menu!",
                        key: Key::V,
                        id: 1,
                        ..Menu::default()
                    }];

    window.add_menu("test", &menu);

    while window.is_open() && !window.is_key_down(Key::Escape) {
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

        window.update_with_buffer(&buffer);
    }
}
