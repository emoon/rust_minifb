# Changelog

This project follows semantic versioning.

### v0.11.2 (2018-12-19)

- [added] Window.is_key_released

### v0.11.1 (2018-11-13)

- [fixed] Fixed bad window size in menu example

### v0.11 (2018-10-23)

- [changed] macOS now uses Metal for rendering the buffer.

### v0.10.7 (2018-08-10)

Thanks to Lukas Kalbertodt for these changes!

- [added] Debug impls for public types
- [fixed] Removed several `doc(hidden)`

### v0.10.6 (2018-05-18)

- [added] Scale x16 and x32 added for Unix

### v0.10.5 (2018-05-05)

- [added] Scale x8 added for Unix
- [fixed] Auto scaling now works correct if scale up is >= screen size

### v0.10.4 (2018-01-08)

- [fixed] Bumped kernel32 to 0.2.2 due to compile errors on Windows. Thanks to Thomas Versteeg for this fix.

### v0.10.1 (2017-08-15)

- [fixed] Typo in the Redox implementation was fixed after changes in 0.10.0

### v0.10.0 (2017-08-11)

- [changed]  ```update_with_buffer``` Now make sures that input buffer will be large enough. As of this it now returns ```Result<>``` to indicate the status of the call.

### v0.9.2 (2017-07-31)

- [fixed] Bumped x11-dll to 2.14 as it was causing issues on nightly.

### v0.9.1 (2017-04-02)

- [fixed] Correct link to docs in Cargo.toml

### v0.9.0 (2016-08-02)

- [added] ```get_unscaled_mouse_pos``` Can be used the get actual mouse pos on a scaled window

### v0.8.4 (2016-07-31)

- [fixed] Mac: Fixed crash on large window sizes

### v0.8.3 (2016-07-29)

- [fixed] Mac: "Plonk sound" when pressing keys
- [fixed] Mac: incorrect size for ``get_size()``

### v0.8.2 (2016-07-07)

- [fixed] Fixed so keypad keys works on Linux

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

