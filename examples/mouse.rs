use minifb::{Key, MouseButton, MouseMode, Scale, Window, WindowOptions};

const WIDTH: usize = 640;
const HEIGHT: usize = 360;

fn main() {
    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];

    let mut window = match Window::new(
        "Mouse Draw - Press ESC to exit",
        WIDTH,
        HEIGHT,
        WindowOptions {
            scale: Scale::X2,
            ..WindowOptions::default()
        },
    ) {
        Ok(win) => win,
        Err(err) => {
            println!("Unable to create window {}", err);
            return;
        }
    };

    let (mut width, mut height) = (WIDTH, HEIGHT);

    while window.is_open() && !window.is_key_down(Key::Escape) {
        {
            let (new_width, new_height) = window.get_size();
            if new_width != width || new_height != height {
                // Div by / 2 here as we use 2x scaling for the buffer
                // copy valid bits of old buffer to new buffer
                let mut new_buffer = vec![0; (new_width / 2) * (new_height / 2)];
                for y in 0..(height / 2).min(new_height / 2) {
                    for x in 0..(width / 2).min(new_width / 2) {
                        new_buffer[y * (new_width / 2) + x] = buffer[y * (width / 2) + x];
                    }
                }
                buffer = new_buffer;
                width = new_width;
                height = new_height;
            }
        }

        window.get_mouse_pos(MouseMode::Discard).map(|(x, y)| {
            let screen_pos = ((y as usize) * (width / 2)) + x as usize;

            if window.get_mouse_down(MouseButton::Left) {
                buffer[screen_pos] = 0x00ffffff;
            }

            if window.get_mouse_down(MouseButton::Right) {
                buffer[screen_pos] = 0;
            }
        });

        window.get_scroll_wheel().map(|scroll| {
            println!("Scrolling {} - {}", scroll.0, scroll.1);
        });

        // We unwrap here as we want this code to exit if it fails
        window
            .update_with_buffer(&buffer, width / 2, height / 2)
            .unwrap();
    }
}
