use minifb::{CursorStyle, Key, MouseMode, Window, WindowOptions};

const WIDTH: usize = 1280;
const HEIGHT: usize = 720;

struct Rect {
    top_left_x: usize,
    top_left_y: usize,
    width: usize,
    height: usize,
    color: u32,
    cursor_style: CursorStyle,
}

impl Rect {
    const WIDTH: usize = WIDTH / 4; // Four rectangles per row
    const HEIGHT: usize = HEIGHT / 2; // Two rectangles per column

    pub fn is_inside(&self, top_left_x: usize, top_left_y: usize) -> bool {
        let bottom_right_x = self.top_left_x + self.width;
        let bottom_right_y = self.top_left_y + self.height;

        (top_left_y >= self.top_left_y)
            && (top_left_y <= bottom_right_y)
            && (top_left_x >= self.top_left_x)
            && (top_left_x <= bottom_right_x)
    }
}

fn fill_rect(dest: &mut [u32], rect: &Rect) {
    for y in 0..rect.height {
        for x in 0..rect.width {
            dest[((rect.top_left_y + y) * WIDTH) + rect.top_left_x + x] = rect.color;
        }
    }
}

fn main() {
    let mut buffer = vec![0u32; WIDTH * HEIGHT];

    let mut window = Window::new("I haz no title :(", WIDTH, HEIGHT, WindowOptions::default())
        .expect("Unable to open the window");

    window.set_target_fps(60);

    let rects = [
        // Top row
        Rect {
            top_left_x: 0,
            top_left_y: 0,
            width: Rect::WIDTH,
            height: Rect::HEIGHT,
            color: 0x00b27474,
            cursor_style: CursorStyle::Arrow,
        },
        Rect {
            top_left_x: Rect::WIDTH,
            top_left_y: 0,
            width: Rect::WIDTH,
            height: Rect::HEIGHT,
            color: 0x00b28050,
            cursor_style: CursorStyle::Ibeam,
        },
        Rect {
            top_left_x: Rect::WIDTH * 2,
            top_left_y: 0,
            width: Rect::WIDTH,
            height: Rect::HEIGHT,
            color: 0x00a9b250,
            cursor_style: CursorStyle::Crosshair,
        },
        Rect {
            top_left_x: Rect::WIDTH * 3,
            top_left_y: 0,
            width: Rect::WIDTH,
            height: Rect::HEIGHT,
            color: 0x0060b250,
            cursor_style: CursorStyle::ClosedHand,
        },
        // Bottom row
        Rect {
            top_left_x: 0,
            top_left_y: Rect::HEIGHT,
            width: Rect::WIDTH,
            height: Rect::HEIGHT,
            color: 0x004fb292,
            cursor_style: CursorStyle::OpenHand,
        },
        Rect {
            top_left_x: Rect::WIDTH,
            top_left_y: Rect::HEIGHT,
            width: Rect::WIDTH,
            height: Rect::HEIGHT,
            color: 0x004f71b2,
            cursor_style: CursorStyle::ResizeLeftRight,
        },
        Rect {
            top_left_x: Rect::WIDTH * 2,
            top_left_y: Rect::HEIGHT,
            width: Rect::WIDTH,
            height: Rect::HEIGHT,
            color: 0x008850b2,
            cursor_style: CursorStyle::ResizeUpDown,
        },
        Rect {
            top_left_x: Rect::WIDTH * 3,
            top_left_y: Rect::HEIGHT,
            width: Rect::WIDTH,
            height: Rect::HEIGHT,
            color: 0x00b25091,
            cursor_style: CursorStyle::ResizeAll,
        },
    ];

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let (mx, my) = window.get_mouse_pos(MouseMode::Clamp).unwrap();

        for rect in &rects {
            fill_rect(&mut buffer, rect);
            if rect.is_inside(mx as usize, my as usize) {
                window.set_cursor_style(rect.cursor_style);
                window.set_title(&format!(
                    "Cursor of hovered rectangle: {:?}",
                    rect.cursor_style
                ));
            }
        }

        window.update_with_buffer(&buffer, WIDTH, HEIGHT).unwrap();
    }
}
