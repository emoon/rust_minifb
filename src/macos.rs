#![cfg(target_os = "macos")]

use Scale;
use Vsync;
use Key;

//use libc;
//use cocoa::appkit;
use cocoa::appkit::*;
//use cocoa::appkit::NSEventSubtype::*;

#[allow(unused_imports)]
use cocoa::base::{id, nil};
#[allow(unused_imports)]
use objc::runtime::{Class, Object, Sel, BOOL, YES, NO};

#[allow(unused_imports)]
use cocoa::foundation::{NSAutoreleasePool, NSDate, NSDefaultRunLoopMode, NSPoint, NSRect, NSSize, 
                        NSString, NSUInteger}; 

use std::ops::Deref;

pub struct Window {
    view: IdRef,
    window: IdRef,
}

impl Window {
    pub fn new(name: &str, width: usize, height: usize, _: Scale, _: Vsync) -> Result<Window, &str> {
        unsafe  {
            let app = match Self::create_app() {
                Some(app) => app,
                None => { 
                    return Err("Couldn't create NSApplication"); 
                },
            };

            let window = match Self::create_window(name, width, height) {
                Some(window) => window,
                None => {
                    return Err("Unable to create NSWindow");
                }
            };

            let view = match Self::create_view(*window) {
                Some(view) => view,
                None => {
                    return Err("Unable to create NSView");
                }
            };

            app.activateIgnoringOtherApps_(YES);

            println!("Created window and view");

            return Ok(Window {
                window: window,
                view: view
            });
        }
    }

    unsafe fn create_window(name: &str, width: usize, height: usize) -> Option<IdRef> {
        let masks = NSResizableWindowMask as NSUInteger | 
                    NSClosableWindowMask as NSUInteger | 
                    NSTitledWindowMask as NSUInteger;

        let frame = NSRect::new(NSPoint::new(0., 0.), NSSize::new(width as f64, height as f64));

        let window = IdRef::new(NSWindow::alloc(nil).initWithContentRect_styleMask_backing_defer_(
            frame,
            masks,
            NSBackingStoreBuffered,
            NO,
        ));

        window.non_nil().map(|window| {
            let title = IdRef::new(NSString::alloc(nil).init_str(name));
            window.setTitle_(*title);
            window.center();
            window
        })
    }

    unsafe fn create_view(window: id) -> Option<IdRef> {
        let view = IdRef::new(NSView::alloc(nil).init());
        view.non_nil().map(|view| {
            window.setContentView_(*view);
            view
        })
    }

    pub fn update(_: &[u32]) { 
    }

    pub fn get_keys() -> Option<Vec<Key>> {
        return None;
    }

    pub fn is_esc_pressed() -> bool {
        false
    }

    pub fn close() {

    }

    fn create_app() -> Option<id> {
        unsafe {
            let app = NSApp();
            if app == nil {
                None
            } else {
                app.setActivationPolicy_(NSApplicationActivationPolicyRegular);
                Some(app)
            }
        }
    }
}



struct IdRef(id);

impl IdRef {
    fn new(i: id) -> IdRef {
        IdRef(i)
    }

    #[allow(dead_code)]
    fn retain(i: id) -> IdRef {
        if i != nil {
            let _: id = unsafe { msg_send![i, retain] };
        }
        IdRef(i)
    }

    fn non_nil(self) -> Option<IdRef> {
        if self.0 == nil { None } else { Some(self) }
    }
}

impl Drop for IdRef {
    fn drop(&mut self) {
        if self.0 != nil {
            let _: () = unsafe { msg_send![self.0, release] };
        }
    }
}

impl Deref for IdRef {
    type Target = id;
    fn deref<'a>(&'a self) -> &'a id {
        &self.0
    }
}

impl Clone for IdRef {
    fn clone(&self) -> IdRef {
        if self.0 != nil {
            let _: id = unsafe { msg_send![self.0, retain] };
        }
        IdRef(self.0)
    }
}
