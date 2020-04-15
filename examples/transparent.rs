use minifb::{Key, ScaleMode, Window, WindowOptions};

fn main() {
    // Allocate the output buffer.
    let buf = vec![0x00AAFF33; 320 * 480];

    let mut window = Window::new(
        "Press ESC to exit",
        320,
        480,
        WindowOptions {
            resize: true,
            scale_mode: ScaleMode::Center,
            borderless: true,
            transparency: true,
            ..WindowOptions::default()
        },
    )
    .expect("Unable to open Window");

    while window.is_open() && !window.is_key_down(Key::Escape) {
        window.update_with_buffer(&buf, 320, 480).unwrap();
    }
}
