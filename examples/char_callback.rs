use minifb::{InputCallback, Key, Window, WindowOptions};
use std::{cell::RefCell, rc::Rc};

const WIDTH: usize = 1280;
const HEIGHT: usize = 720;

type KeyVec = Rc<RefCell<Vec<u32>>>;

struct Input {
    keys: KeyVec,
}

impl InputCallback for Input {
    /// Will be called every time a character key is pressed
    fn add_char(&mut self, uni_char: u32) {
        self.keys.borrow_mut().push(uni_char);
    }
}

fn main() {
    let mut buffer = vec![0u32; WIDTH * HEIGHT];

    let mut window = Window::new(
        "char_callback example - press ESC to exit",
        WIDTH,
        HEIGHT,
        WindowOptions::default(),
    )
    .expect("Unable to create the window");

    window.set_target_fps(60);

    let keys = KeyVec::new(RefCell::new(Vec::new()));
    window.set_input_callback(Box::new(Input { keys: keys.clone() }));

    // produces a wave pattern where the screen goes from red to blue, and vice-versa
    let mut wave: u8 = 0;
    let mut wave_direction: i8 = 1;
    while window.is_open() && !window.is_key_down(Key::Escape) {
        let red: u32 = (wave as u32) << 16;
        let green: u32 = 64 << 8;
        let blue: u32 = (255 - wave) as u32;
        let bg_color = red | green | blue;

        for pixel in buffer.iter_mut() {
            *pixel = bg_color;
        }

        // We unwrap here as we want this code to exit if it fails. Real applications may want to handle this in a different way
        window.update_with_buffer(&buffer, WIDTH, HEIGHT).unwrap();

        let mut keys = keys.borrow_mut();

        for t in keys.iter() {
            println!("Code point: {}, Character: {:?}", *t, char::from_u32(*t));
        }

        keys.clear();

        // switch color wave directions
        match wave.checked_add_signed(wave_direction) {
            Some(new_wave) => wave = new_wave,
            None => {
                wave_direction = -wave_direction;
                if wave_direction > 0 {
                    wave += 1;
                } else {
                    wave -= 1;
                }
            }
        }
    }
}
