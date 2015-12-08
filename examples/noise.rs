extern crate minifb;

use minifb::*;

const WIDTH: usize = 640;
const HEIGHT: usize = 360;

fn main() {
    let mut noise;
    let mut carry;
    let mut seed = 0xbeefu32;

    let mut buffer: [u32; WIDTH * HEIGHT] = [0; WIDTH * HEIGHT];

    let mut window = Window::new("Noise Test - Press ESC to exit",
                                 WIDTH,
                                 HEIGHT,
                                 Scale::X1,
                                 Vsync::No)
                         .unwrap();

    while window.update(&buffer) {
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

        for key in window.get_keys().iter() {
            match *key {
                Key::A => println!("Pressed A"),
                Key::B => println!("Pressed B"),
                _ => (),
            }
        }
    }
}
