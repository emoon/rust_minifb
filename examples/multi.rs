use minifb::{Key, Scale, Window, WindowOptions};

// Size of the main window
const WIDTH: usize = 1280 / 2;
const HEIGHT: usize = 720 / 2;

fn main() {
    let mut buffer = vec![0u32; WIDTH * HEIGHT];

    let mut larger_window = Window::new(
        "Larger - press ESC to exit",
        WIDTH,
        HEIGHT,
        WindowOptions {
            scale: Scale::X2,
            ..WindowOptions::default()
        },
    )
    .expect("Unable to create the larger window");

    larger_window.set_target_fps(60);

    // Creating the smaller window after the larger one to make it appear on top
    let mut smaller_window = Window::new(
        "Smaller - press ESC to exit",
        WIDTH,
        HEIGHT,
        WindowOptions {
            scale: Scale::X1, // Which is also the default
            ..WindowOptions::default()
        },
    )
    .expect("Unable to create the smaller window");

    smaller_window.set_target_fps(60);

    // Randomly drawing dots
    let mut dot_position = 13;

    while smaller_window.is_open()
        && larger_window.is_open()
        && !smaller_window.is_key_down(Key::Escape)
        && !larger_window.is_key_down(Key::Escape)
    {
        smaller_window
            .update_with_buffer(&buffer, WIDTH, HEIGHT)
            .unwrap();
        larger_window
            .update_with_buffer(&buffer, WIDTH, HEIGHT)
            .unwrap();

        dot_position += 7;
        dot_position *= 13;
        dot_position %= buffer.len();

        buffer[dot_position] = 0x00_ff_ff_ff;
    }
}
