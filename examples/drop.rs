use minifb::{Window, WindowOptions};
use std::thread;
use std::time::{Duration, Instant};

const WIDTH: usize = 640 / 2;
const HEIGHT: usize = 360 / 2;

fn show_window() {
    let mut window = Window::new(
        "Drop Test - Window will close after 2 seconds.",
        WIDTH,
        HEIGHT,
        WindowOptions::default(),
    )
    .expect("Unable to create window");

    let now = Instant::now();

    while window.is_open() && now.elapsed().as_secs() < 2 {
        window.update();
    }
}

fn main() {
    println!("Showing Window");
    show_window();
    println!("Dropped");
    thread::sleep(Duration::from_millis(2000));
    println!("Exiting");
}
