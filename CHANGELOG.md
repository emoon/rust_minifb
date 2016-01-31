# Changelog

This project follows semantic versioning.

### UNRELEASED (to be 0.4.0)

This release breaks some of the API by changing names and parameters to some functions.

- [changed] ```Window::new(...)``` now takes WindowOptions struct to configure the creation of the Window.
- [changed] ```window.update()``` Doesn't take a buffer anymore. See ```window.update_with_buffer```
- [added] ```window.update_with_buffer()``` Old update version that takes buffer as input parameter
- [added] ```window.get_window_handle()``` Returns the native handle (os dependant) 

### v0.3.1 (2016-01-29)

- [fixed] ```get_mouse_pos(MouseMode::Clamp)``` now is in the region [(0, 0) - (width - 1, height - 1)] instead of (width, height)

### v0.3.0 (2016-01-29)

This release adds support for mouse input. See the documentation and the examples for usage

- [added] ```[get_mouse_pos](http://prodbg.com/minifb/minifb/struct.Window.html#method.get_mouse_pos)```
- [added] ```get_mouse_down```
- [added] ```get_scroll_wheel```

