// This example produces a wasm module that you can add to a web project, check out
// https://github.com/dc740/minifb-async-examples to see how to integrate it

use minifb::{Window, WindowOptions};
use std::{cell::RefCell, panic, rc::Rc};
use wasm_bindgen::prelude::*;

const WIDTH: usize = 1280;
const HEIGHT: usize = 720;

fn window() -> web_sys::Window {
    web_sys::window().expect("no global `window` exists")
}

fn request_animation_frame(f: &Closure<dyn FnMut()>) {
    window()
        .request_animation_frame(f.as_ref().unchecked_ref())
        .expect("should register `requestAnimationFrame` OK");
}

#[wasm_bindgen(start)]
fn main() {
    panic::set_hook(Box::new(console_error_panic_hook::hook));

    // The id of a DOM element to render the window into
    let container = "minifb-container";

    let mut window = Window::new(container, WIDTH, HEIGHT, WindowOptions::default())
        .expect("Unable to create the window");

    let mut buffer = vec![0; WIDTH * HEIGHT];

    // A reference counted pointer to the closure that will update window
    let f = Rc::new(RefCell::new(None));
    let g = f.clone();
    // we update the window here just to reference the buffer
    // internally. Next calls to .update() will use the same buffer
    window.update_with_buffer(&buffer, WIDTH, HEIGHT).unwrap();

    // create the closure for updating and rendering the window
    *g.borrow_mut() = Some(Closure::wrap(Box::new(move || {
        for pixel in buffer.iter_mut() {
            *pixel = pixel.wrapping_add(1);
        }

        // as the buffer is referenced from inside the ImageData, and
        // we push that to the canvas, so we could call update() and
        // avoid all this. I don't think it's possible to get artifacts
        // on the web side, but I definitely see them on the desktop app
        let _ = window.update_with_buffer(&buffer, WIDTH, HEIGHT);

        // schedule this closure for running again at next frame
        request_animation_frame(f.borrow().as_ref().unwrap());
    }) as Box<dyn FnMut() + 'static>));

    // start the animation loop
    request_animation_frame(g.borrow().as_ref().unwrap());
}
