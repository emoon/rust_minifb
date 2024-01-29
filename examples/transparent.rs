use minifb::{Key, ScaleMode, Window, WindowOptions};

const WIDTH: usize = 1280;
const HEIGHT: usize = 720;

fn main() {
    let mut buffer = vec![0u32; WIDTH * HEIGHT];

    let mut window = Window::new(
        "Transparent window example - press ESC to exit",
        WIDTH,
        HEIGHT,
        WindowOptions {
            transparency: true,
            ..WindowOptions::default()
        },
    )
    .expect("Unable to create the window");

    window.set_target_fps(60);

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
