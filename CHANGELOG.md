# Changelog

This project follows semantic versioning.

### v0.8.1 (2016-07-07)

- [fixed] Character callback wouldn't get called on Mac and Linux
- [fixed] Resize cursors on Windows was swapped

### v0.8.0 (2016-06-24)

- [added] ```window.set_title``` Can now change title after creation
- [added] ```window.set_cursor_style``` Can now change the style of the cursor with a number of (OS supported types)
- [added] Added cursor_title example code to show the newly added features

### v0.7.1 (2016-05-27)

- [fixed] Character callback wouldn't get called on Mac.

### v0.7.0 (2016-05-12)

- [changed] - Fully rewrote the Menu API. See the documentation/menu example for the changes.
- [added] - Added ```Window::get_unix_menus``` to get data access to menus on Linux/x11

### v0.6.0 (2016-05-01)

- [added] added ```get_size()``` to retrive the size of the window.

### v0.5.2 (2016-04-29)

- [fixed] On Mac shortcuts using F1-F12 wasn't working correctly.

### v0.5.1 (2016-04-25)

- [fixed] ```get_window_handle``` would return an invalid value on Unix. Now fixed.

### v0.5.0 (2016-03-04)

- [changed] - Proper Errors which uses ```std::Error``` as base. ```Window::new``` uses this but the API itself hasn't changed.
- [added] - Menu support on Mac and Windows. See the Menu API functions [here](http://prodbg.com/minifb/minifb/struct.Window.html#method.add_menu)
- [known issue] - ```remove_menu``` doesn't work on Windows [issue](https://github.com/emoon/rust_minifb/issues/16)
- [known issue] - On Mac when running an application from terminal on has to switch to another application and back to get menu focus. [issue](https://github.com/emoon/rust_minifb/issues/17)

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

