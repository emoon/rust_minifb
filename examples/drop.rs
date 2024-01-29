use minifb::{Window, WindowOptions};
use std::{
    thread,
    time::{Duration, Instant},
};

const WIDTH: usize = 1280;
const HEIGHT: usize = 720;

fn main() {
    println!("Creating and showing a Window");

    let mut window = Window::new(
        "Drop example - Window will close after 5 seconds",
        WIDTH,
        HEIGHT,
        WindowOptions::default(),
    )
    .expect("Unable to create the window");

    window.set_target_fps(60);

    let now = Instant::now();
    while window.is_open() && now.elapsed().as_secs() < 5 {
        window.update();
    }

    drop(window);
    println!("Dropped");

    thread::sleep(Duration::from_secs(2));
    println!("Exiting");
}
