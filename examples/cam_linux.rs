use minifb::{Key, Window, WindowOptions};

const WIDTH: usize = 640;
const HEIGHT: usize = 360;

fn main() {
    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];

    let mut window = Window::new(
        "Test - ESC to exit",
        WIDTH,
        HEIGHT,
        WindowOptions::default(),
    )
    .unwrap_or_else(|e| {
        panic!("{}", e);
    });

    // Limit to max ~30 fps update rate
    window.limit_update_rate(Some(std::time::Duration::from_micros(8300)));

    let rx = v4l();
    while window.is_open() && !window.is_key_down(Key::Escape) {
        let frame = rx.recv().unwrap();

        let frame: Vec<u32> = frame
            .chunks(4)
            .map(|v| {
                // convert form YUYV to RGB
                #[allow(non_snake_case)]
                let v = if let [Y, U, _, V] = v {
                    let Y = *Y as f32;
                    let U = *U as f32;
                    let V = *V as f32;

                    let B = 1.164 * (Y - 16.) + 2.018 * (U - 128.);

                    let G = 1.164 * (Y - 16.) - 0.813 * (V - 128.) - 0.391 * (U - 128.);

                    let R = 1.164 * (Y - 16.) + 1.596 * (V - 128.);
                    [0, R as u8, G as u8, B as u8]
                } else {
                    unreachable!()
                };
                use byteorder::{BigEndian, ByteOrder};

                BigEndian::read_u32(&v)
            })
            .collect();

        // write our frame to the screen buffer
        for (idx, i) in buffer.iter_mut().enumerate() {
            if idx == frame.len() {
                break;
            }
            *i = frame[idx]; // write something more funny here!
        }

        // We unwrap here as we want this code to exit if it fails. Real applications may want to handle this in a different way
        window.update_with_buffer(&buffer, WIDTH, HEIGHT).unwrap();
    }
}

use std::sync::mpsc::*;
fn v4l() -> Receiver<Vec<u8>> {
    let (tx, rx) = channel();

    std::thread::spawn(move || {
        use v4l::prelude::*;
        use v4l::FourCC;

        let mut dev = CaptureDevice::new(0).expect("Failed to open device");
        let mut fmt = dev.format().expect("Failed to read format");
        fmt.fourcc = FourCC::new(b"YUYV");
        dev.set_format(&fmt).expect("Failed to write format");

        let mut stream =
            MmapStream::with_buffers(&mut dev, 4).expect("Failed to create buffer stream");

        loop {
            let frame = stream.next().unwrap().data().to_owned();
            tx.send(frame).unwrap();
        }
    });
    rx
}
