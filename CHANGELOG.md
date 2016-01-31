# Changelog

This project follows semantic versioning.

### v0.4.0 (2016-01-31)

This release breaks some of the API by changing names and parameters to some functions.

- [changed] ```Window::new(...)``` now takes WindowOptions struct to configure the creation of the Window. [doc](http://prodbg.com/minifb/minifb/struct.Window.html#method.new)
- [changed] ```window.update()``` Doesn't take a buffer anymore. See ```window.update_with_buffer``` [doc](http://prodbg.com/minifb/minifb/struct.Window.html#method.update)
- [added] ```window.update_with_buffer()``` Old update version that takes buffer as input parameter [doc](http://prodbg.com/minifb/minifb/struct.Window.html#method.update_with_buffer)
- [added] ```window.get_window_handle()``` Returns the native handle (os dependant) [doc](http://prodbg.com/minifb/minifb/struct.Window.html#method.get_window_handle)

### v0.3.1 (2016-01-29)

- [fixed] ```get_mouse_pos(Clamp)``` clamps to ```[(0, 0) - (width - 1, height - 1)]``` instead of ```(width, height)```

### v0.3.0 (2016-01-29)

This release adds support for mouse input. See the documentation and the examples for usage

- [added] [get_mouse_pos](http://prodbg.com/minifb/minifb/struct.Window.html#method.get_mouse_pos)
- [added] [get_mouse_down](http://prodbg.com/minifb/minifb/struct.Window.html#method.get_mouse_down)
- [added] [get_scroll_wheel](http://prodbg.com/minifb/minifb/struct.Window.html#method.get_scroll_wheel)

