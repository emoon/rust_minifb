use std::str::FromStr;

use minifb::{Icon, Key, ScaleMode, Window, WindowOptions};

fn main() {
    use std::fs::File;
    // The decoder is a build for reader and can be used to set various decoding options
    // via `Transformations`. The default output transformation is `Transformations::EXPAND
    // | Transformations::STRIP_ALPHA`.
    let decoder = png::Decoder::new(File::open("resources/uv.png").unwrap());
    let mut reader = decoder.read_info().unwrap();
    // Allocate the output buffer.
    let mut buf = vec![0; reader.output_buffer_size()];
    // Read the next frame. Currently this function should only called once.
    // The default options
    reader.next_frame(&mut buf).unwrap();
    // convert buffer to u32

    let u32_buffer: Vec<u32> = buf
        .chunks(3)
        .map(|v| ((v[0] as u32) << 16) | ((v[1] as u32) << 8) | v[2] as u32)
        .collect();

    let mut window = Window::new(
        "Noise Test - Press ESC to exit",
        reader.info().width as usize,
        reader.info().height as usize,
        WindowOptions {
            resize: true,
            scale_mode: ScaleMode::Center,
            ..WindowOptions::default()
        },
    )
    .expect("Unable to open Window");

    window.set_icon(Icon::from_str("after256.ico").unwrap());

    while window.is_open() && !window.is_key_down(Key::Escape) {
        window
            .update_with_buffer(
                &u32_buffer,
                reader.info().width as usize,
                reader.info().height as usize,
            )
            .unwrap();
    }
}
