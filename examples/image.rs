use minifb::{Key, Window, WindowOptions};
use png::{Decoder, Transformations};
use std::fs::File;

fn main() {
    let mut decoder = Decoder::new(File::open("examples/resources/planet.png").unwrap());

    // Reading the image in RGBA format.
    decoder.set_transformations(Transformations::ALPHA);
    let mut reader = decoder.read_info().unwrap();

    let mut buffer = vec![0u32; reader.output_buffer_size()];

    // View of pixels as individual subpixels (avoids allocating a second pixel buffer).
    let mut u8_buffer = unsafe {
        std::slice::from_raw_parts_mut(
            buffer.as_mut_ptr() as *mut u8,
            buffer.len() * std::mem::size_of::<u32>(),
        )
    };

    // Read the next frame. Currently this function should only be called once.
    reader.next_frame(&mut u8_buffer).unwrap();

    // convert RGBA buffer read by the reader to an ARGB buffer as expected by minifb.
    for (rgba, argb) in u8_buffer.chunks_mut(4).zip(buffer.iter_mut()) {
        // extracting the subpixels
        let r = rgba[0] as u32;
        let g = rgba[1] as u32;
        let b = rgba[2] as u32;
        let a = rgba[3] as u32;

        // merging the subpixels in ARGB format.
        *argb = a << 24 | r << 16 | g << 8 | b;
    }

    let width = reader.info().width as usize;
    let height = reader.info().height as usize;

    let mut window = Window::new(
        "Image background example - Press ESC to exit",
        width,
        height,
        WindowOptions::default(),
    )
    .expect("Unable to create the window");

    window.set_target_fps(60);

    while window.is_open() && !window.is_key_down(Key::Escape) {
        window.update_with_buffer(&buffer, width, height).unwrap();
    }
}
