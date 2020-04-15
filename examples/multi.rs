use minifb::{Key, Scale, Window, WindowOptions};

fn main() {
    let width = 640;
    let height = 320;
    let mut buffer = vec![0u32; width * height];
    let mut double = Window::new(
        "Larger",
        width,
        height,
        WindowOptions {
            scale: Scale::X2,
            ..WindowOptions::default()
        },
    )
    .unwrap();

    let mut orig = Window::new(
        "Smaller",
        width,
        height,
        WindowOptions {
            ..WindowOptions::default()
        },
    )
    .unwrap();

    let mut pos = 13;

    while orig.is_open()
        && double.is_open()
        && !orig.is_key_down(Key::Escape)
        && !double.is_key_down(Key::Escape)
    {
        orig.update_with_buffer(&buffer, width, height).unwrap();
        double.update_with_buffer(&buffer, width, height).unwrap();
        pos += 7;
        pos *= 13;
        pos %= buffer.len();
        buffer[pos] = 0xff_ff_ff;
    }
}
