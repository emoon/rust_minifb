#![cfg(target_os = "macos")]

use Scale;
use Vsync;
use Key;

use libc;
//use cocoa::appkit;
use cocoa::appkit::*;
//use cocoa::appkit::NSEventSubtype::*;

#[allow(unused_imports)]
use cocoa::base::{id, nil};
#[allow(unused_imports)]
use objc::runtime::{Class, Object, Sel, BOOL, YES, NO};
use objc::declare::ClassDecl;

#[allow(unused_imports)]
use cocoa::foundation::{NSAutoreleasePool, NSDate, NSDefaultRunLoopMode, NSPoint, NSRect, NSSize, 
                        NSString, NSUInteger}; 

use std::ops::Deref;

pub struct Window {
    view: IdRef,
    window: IdRef,
    delegate: WindowDelegate,
}

struct DelegateState {
    view: IdRef,
    window: IdRef,
    resize_handler: Option<fn(u32, u32)>,
}

struct WindowDelegate {
    state: Box<DelegateState>,
    _this: IdRef,
}

//sthou

impl WindowDelegate {
    /// Get the delegate class, initiailizing it neccessary
    fn class() -> *const Class {
        use std::sync::{Once, ONCE_INIT};

        extern fn window_should_close(this: &Object, _: Sel, _: id) -> BOOL {
            unsafe {
                let state: *mut libc::c_void = *this.get_ivar("glutinState");
                let state = state as *mut DelegateState;
                //(*state).pending_events.lock().unwrap().push_back(Closed);
            }
            YES
        }

        extern fn window_did_resize(this: &Object, _: Sel, _: id) {
            unsafe {
                let state: *mut libc::c_void = *this.get_ivar("glutinState");
                let state = &mut *(state as *mut DelegateState);

                //let _: () = msg_send![*state.context, update];

                if let Some(handler) = state.resize_handler {
                    let rect = NSView::frame(*state.view);
                    let scale_factor = NSWindow::backingScaleFactor(*state.window) as f32;
                    (handler)((scale_factor * rect.size.width as f32) as u32,
                              (scale_factor * rect.size.height as f32) as u32);
                }
            }
        }

        extern fn window_did_become_key(this: &Object, _: Sel, _: id) {
            unsafe {
                // TODO: center the cursor if the window had mouse grab when it
                // lost focus

                let state: *mut libc::c_void = *this.get_ivar("glutinState");
                let state = state as *mut DelegateState;
                //(*state).pending_events.lock().unwrap().push_back(Focused(true));
            }
        }

        extern fn window_did_resign_key(this: &Object, _: Sel, _: id) {
            unsafe {
                let state: *mut libc::c_void = *this.get_ivar("glutinState");
                let state = state as *mut DelegateState;
                //(*state).pending_events.lock().unwrap().push_back(Focused(false));
            }
        }

        static mut delegate_class: *const Class = 0 as *const Class;
        static INIT: Once = ONCE_INIT;

        INIT.call_once(|| unsafe {
            // Create new NSWindowDelegate
            let superclass = Class::get("NSObject").unwrap();
            let mut decl = ClassDecl::new(superclass, "GlutinWindowDelegate").unwrap();

            // Add callback methods
            decl.add_method(sel!(windowShouldClose:),
                window_should_close as extern fn(&Object, Sel, id) -> BOOL);
            decl.add_method(sel!(windowDidResize:),
                window_did_resize as extern fn(&Object, Sel, id));
            
            decl.add_method(sel!(windowDidBecomeKey:),
                window_did_become_key as extern fn(&Object, Sel, id));
            decl.add_method(sel!(windowDidResignKey:),
                window_did_resign_key as extern fn(&Object, Sel, id));

            // Store internal state as user data
            decl.add_ivar::<*mut libc::c_void>("glutinState");

            delegate_class = decl.register();
        });

        unsafe {
            delegate_class
        }
    }

    fn new(state: DelegateState) -> WindowDelegate {
        // Box the state so we can give a pointer to it
        let mut state = Box::new(state);
        let state_ptr: *mut DelegateState = &mut *state;
        unsafe {
            let delegate = IdRef::new(msg_send![WindowDelegate::class(), new]);

            (&mut **delegate).set_ivar("glutinState", state_ptr as *mut libc::c_void);
            let _: () = msg_send![*state.window, setDelegate:*delegate];

            WindowDelegate { state: state, _this: delegate }
        }
    }
}

impl Drop for WindowDelegate {
    fn drop(&mut self) {
        unsafe {
            // Nil the window's delegate so it doesn't still reference us
            let _: () = msg_send![*self.state.window, setDelegate:nil];
        }
    }
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

            let ds = DelegateState {
                view: view.clone(),
                window: window.clone(),
                resize_handler: None,
            };

            println!("Created window and view");

            return Ok(Window {
                window: window,
                view: view,
                delegate: WindowDelegate::new(ds),
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
