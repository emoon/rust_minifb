use minifb::{Key, ScaleMode, Window, WindowOptions};

const WIDTH: usize = 1280;
const HEIGHT: usize = 720;

fn main() {
    let buffer = vec![0x00_FF_FF_00u32; WIDTH * HEIGHT];

    let mut window = Window::new(
        "Topmost example - press ESC to exit",
        WIDTH,
        HEIGHT,
        WindowOptions {
            resize: true,
            scale_mode: ScaleMode::Center,
            topmost: true,
            ..WindowOptions::default()
        },
    )
    .expect("Unable to open the window");

    window.set_target_fps(60);

    while window.is_open() && !window.is_key_down(Key::Escape) {
        window.update_with_buffer(&buffer, WIDTH, HEIGHT).unwrap();
    }
}
