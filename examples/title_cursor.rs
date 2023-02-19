use minifb::{CursorStyle, Key, MouseMode, Scale, Window, WindowOptions};

const WIDTH: usize = 640;
const HEIGHT: usize = 360;

struct Rect {
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    color: u32,
    cursor_style: CursorStyle,
}

impl Rect {
    pub fn is_inside(&self, xf: f32, yf: f32) -> bool {
        let x = xf as usize;
        let y = yf as usize;
        let xe = self.x + self.width;
        let ye = self.y + self.height;

        (y >= self.y) && (y <= ye) && (x >= self.x) && (x <= xe)
    }
}

fn fill_rect(dest: &mut [u32], rect: &Rect) {
    for y in 0..rect.height {
        for x in 0..rect.width {
            dest[((rect.y + y) * WIDTH) + rect.x + x] = rect.color;
        }
    }
}

fn main() {
    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];

    let mut window = Window::new(
        "I haz no title :(",
        WIDTH,
        HEIGHT,
        WindowOptions {
            scale: Scale::X2,
            ..WindowOptions::default()
        },
    )
    .expect("Unable to Open Window");
    let rects = [
        Rect {
            x: 0,
            y: 0,
            width: 160,
            height: 180,
            color: 0x00b27474,
            cursor_style: CursorStyle::Arrow,
        },
        Rect {
            x: 160,
            y: 0,
            width: 160,
            height: 180,
            color: 0x00b28050,
            cursor_style: CursorStyle::Ibeam,
        },
        Rect {
            x: 320,
            y: 0,
            width: 160,
            height: 180,
            color: 0x00a9b250,
            cursor_style: CursorStyle::Crosshair,
        },
        Rect {
            x: 480,
            y: 0,
            width: 160,
            height: 180,
            color: 0x0060b250,
            cursor_style: CursorStyle::ClosedHand,
        },
        Rect {
            x: 0,
            y: 180,
            width: 160,
            height: 180,
            color: 0x004fb292,
            cursor_style: CursorStyle::OpenHand,
        },
        Rect {
            x: 160,
            y: 180,
            width: 160,
            height: 180,
            color: 0x004f71b2,
            cursor_style: CursorStyle::ResizeLeftRight,
        },
        Rect {
            x: 320,
            y: 180,
            width: 160,
            height: 180,
            color: 0x008850b2,
            cursor_style: CursorStyle::ResizeUpDown,
        },
        Rect {
            x: 480,
            y: 180,
            width: 160,
            height: 180,
            color: 0x00b25091,
            cursor_style: CursorStyle::ResizeAll,
        },
    ];

    window.set_title("Different cursor on each color region");

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let (mx, my) = window.get_mouse_pos(MouseMode::Clamp).unwrap();

        for rect in &rects {
            fill_rect(&mut buffer, rect);
            if rect.is_inside(mx, my) {
                window.set_cursor_style(rect.cursor_style);
            }
        }

        // We unwrap here as we want this code to exit if it fails
        window.update_with_buffer(&buffer, WIDTH, HEIGHT).unwrap();
    }
}
