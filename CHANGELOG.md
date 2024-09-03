# Changelog

This project follows semantic versioning.

### v0.27 (2024-05-20)

- [fixed] Temporary fix for struct layout issue on Windows.
 
### v0.26 (2024-05-11)

- [fixed] Lots of formatting & style fixes (Thanks Stefano Incardone!)
- [fixed] raw-window-handle 0.6 (Thanks Stefano Incardone!)
- [fixed] Lots of various fixes and cleanups (Thanks Stefano Incardone!)
- [fixed] repr(C) on Window Struct (Thanks gillet-hernadez!)
- [fixed] Switch Linux scalar code from C++ to C (Thanks Speykious!)
- [API BREAKAGE] `limit_update_rate` has been removed. Use `set_target_fps` instead.

### v0.25 (2023-08-02)

- [fixed] Fix changing window title (Thanks royaltm!)

### v0.24 (2023-02-18)

- [fixed] Windows: Unable to use F10 key
- [fixed] set byposition flag when removing menus (Thanks piksel!)
- [fixed] fixed compilation for x11 32-bit mode (Thanks HBehrens!)
- [fixed] X11 window names not supporting UTF-8 (Thanks edarogh!)
- [fixed] `get_window_position` for multiscreen setup on macOS (Thanks AnderasOM!)
- [fixed] Using minifb on multiple threads under x11 works and doesn't crash (Thanks to konnorandrews for suggestion!)
- [Added] ARM and AARCH64 Windows Support (Thanks smb123w64gb!)

### v0.23 (2022-04-19)

- [fixed] wayland: Fix key character callback & Reduce wayland dependencies (Thanks vmedea!)
- [fixed] Use coercion on types failing compilation on 64 bit raspbian bullseye (Thanks wtfuzz!)
- [added] WASM support. Thanks dc740 and tversteeg! See https://github.com/dc740/minifb-async-examples for example.

### v0.22 (2022-03-27)

- [fixed] Updated docs to make it a bit more clear that only one of `update_with_buffer` or `update` should be used for updating a window.

### v0.21 (2022-03-27)

- [fixed] Holding down a key on x11 would no repeat correctly
- [fixed] Windows dependency cleanups (Thanks Arnab Animesh Das!)
- [fixed] Fixed mouse button being "stuck" when moved out side of window, released and then moved by in on Windows (Thanks Arnab Animesh Das for bug report!)
- [fixed] Memory-map the keymap FD on Wayland to fix EOF error (Thanks Greg Depoire--Ferrer!)
- [added] getter for window position (Thanks Andreas Neukoetter!)
- [fixed] Fix clippy lints under windows (Thanks Kevin K!)
- [added] Add `set_icon()` method to set a window icon at runtime (Thanks Kevin K!)
- [added] inputcallback: add a callback for key events and key_handler: add a callback for key events (Thanks xobs and vemoo!)
- [fixed] macOS: Fix segmentation fault when resizing window. (Thanks KaDiWa!)
- [fixed] Various x11 and wayland fixes, version updates (Thanks vemoo!)

### v0.20 (2021-11-28)

- [API BREAKAGE] Changing return types of get_keys from Option<Vec<Key>> to Vec<Key> (Thanks Zij-IT!)
- [fixed] get_scroll_wheel() would get "stuck" on macOS. (Thanks NikoUY for bug report!)

### v0.19.3 (2021-03-23)

- [fixed] Fixed typos in description (Thanks hiqua!)
- [fixed] update wayland to 0.28 and small cleanup (Thanks xMAC94x!)
- [fixed] Bump xkbcommon-sys to 0.7.5
- [fixed] wayland missing cursor (Thanks dc740!)
- [fixed] windows: use c_void from winapi (Thanks xobs!)

### v0.19.2 (2021-01-18)

TODO

### v0.19 (2020-09-22)

- [fixed] Removed dummy logging

### v0.19 (2020-09-16)

- [added] Added char_callback example on how to capture data.
- [added] Support for topmost on Windows (Thanks phillvancejr!)
- [fixed] ARM (Raspberry Pi) now builds and runs. (Thanks derpeter!)
- [changed] Removed a bunch of dependencies not needed anymore (Thanks RazrFalcon!)

### v0.18 (2020-08-14)

- [fixed] get_released_keys wasn't working under Wayland.

### v0.17 (2020-07-09)

- [changed] unix renamed to posix. (Thanks LoganDark)
- [changed] bunch of Linux/x11 fixes by Luna Siena. Such as Transparency support, Borderless, Cursor Visibility. Thanks!
- [changed] use `std::ptr::null_mut()` to Windows `PeekMessageW` it fix alt-tab stall on Window 7. Thanks lynnux for the report!
- [added] Implemented std::error::Error for minifb::Error. (Thanks Christofer Nolander!)

### v0.16 (2020-04-05)

- [added] Wayland support. (Big thanks to Luna Siena add this support with assistance by Cole Helbling!)
- [added] Added `get_released_keys` (Thanks to Alex Melville!)
- [added] Added Topmost/Always on Top functionality to macOS (Thanks phillvancejr!)
- [fixed] Removed left over logging on macOS (Thanks phillvancejr!)

### v0.15.3 (2020-01-21)

- [Added] On macOS NSView (MTKView) is supplied with raw_window_handle now

### v0.15.2 (2020-01-21)

- [fixed] Fixed forever block on macOS when using `update` and not `update_with_buffer`

### v0.15.1 (2019-12-27)

- [fixed] Fixed access to raw_window_handle()

### v0.15 (2019-12-16)

- [API BREAKAGE] - `update_with_buffer` now always take width and height parameters.
- [added] scale_mode in WindowOptions now allow for aspect correct scaling, center of non-scaled buffers and more.
- [added] Added `limit_update_rate(..)` in order to reduce CPU usage and not hammer the native system calls.
- [changed] x11 now uses C for it's scaling in software mode in order to always have opts on even in debug build.
- [changed] Several fixes with rescaling on all platforms
- [changed] on x11 some window mangers will resize a non-resizable windows and minifb handles this now correctly.
- [fixed] Cursor was behaving bad on Windows. This has now been fixed
- [known issues] There are some flickering and various issues when resizing on most platforms. PRs/ideas welcome for this.

### v0.14 (2019-12-03)

- [changed] Deprecated update_with_buffer on favor of update_with_buffer_size. The idea is that a size in of the buffer will be more robust and allow for aspect scaling as well.
- [changed] Improved macOS resizing support.
- [changed] Better modifier handling on macOS.
- [changed] Moved CI over to Github Actions
- [changed] Formatted all code with rustfmt
- [changed] Documentation improvments (Thanks Gary Guo & Arif Roktim!)
- [fixed] 'attempt to subtract with overflow' bug (Thanks Venceslas!)
- [fixed] Window close handling & missing Alt keys for X11 (Thanks Gary Guo!)
- [added] Juila example added (Thanks mishazawa!)
- [added] Add support for raspberry pi (Thanks Florian Blasius!)
- [added] Added support for raw-window-handle trait

### v0.13 (2019-08-30)

- [changed] unix: replaced scale functions with macro and added missing invocations (Thanks Johannes Stölp!)

### v0.12 (2019-07-21)

- [changed] Linux/Unix backend rewritten in Rust (thanks Chris West!)
- [changed] WinAPI updated to 0.3 (Thanks Richard Hozák!)
- [changed] Bump orbclient to 0.3.20 on Redox, remove alpha handling hacks (Thanks Nagy Tibor!)

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
