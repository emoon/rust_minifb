use minifb::{Key, MouseButton, MouseMode, Scale, Window, WindowOptions};

// Divided by 2 to account for the scale of the window
const WIDTH: usize = 1280 / 2;
const HEIGHT: usize = 720 / 2;

fn main() {
    let mut buffer = vec![0u32; WIDTH * HEIGHT];

    let mut window = Window::new(
        "Mouse drawing example - press ESC to exit",
        WIDTH,
        HEIGHT,
        WindowOptions {
            scale: Scale::X2,
            ..WindowOptions::default()
        },
    )
    .expect("Unable to create the window");

    window.set_target_fps(60);

    let (mut width, mut height) = (WIDTH, HEIGHT);

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let (new_width, new_height) = window.get_size();
        if new_width != width || new_height != height {
            // Divide by / 2 here as we use 2x scaling for the buffer
            let mut new_buffer = vec![0; (new_width / 2) * (new_height / 2)];

            // copy valid bits of old buffer to new buffer
            for y in 0..(height / 2).min(new_height / 2) {
                for x in 0..(width / 2).min(new_width / 2) {
                    new_buffer[y * (new_width / 2) + x] = buffer[y * (width / 2) + x];
                }
            }

            buffer = new_buffer;
            width = new_width;
            height = new_height;
        }

        if let Some((x, y)) = window.get_mouse_pos(MouseMode::Discard) {
            let screen_pos = ((y as usize) * (width / 2)) + x as usize;

            if window.get_mouse_down(MouseButton::Left) {
                buffer[screen_pos] = 0x00ffffff; // white
            }

            if window.get_mouse_down(MouseButton::Right) {
                buffer[screen_pos] = 0x00000000; // black
            }
        }

        if let Some((scroll_x, scroll_y)) = window.get_scroll_wheel() {
            println!("Scrolling {} - {}", scroll_x, scroll_y);
        }

        // We unwrap here as we want this code to exit if it fails
        window
            .update_with_buffer(&buffer, width / 2, height / 2)
            .unwrap();
    }
}
