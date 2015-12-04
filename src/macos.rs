#![cfg(target_os = "macos")]

use CreationError;
use CreationError::OsError;

use libc;
use cocoa::appkit;
use cocoa::appkit::*;
use cocoa::appkit::NSEventSubtype::*;

use cocoa::base::{id, nil};
use objc::runtime::{Class, Object, Sel, BOOL, YES, NO};

use cocoa::foundation::{NSAutoreleasePool, NSDate, NSDefaultRunLoopMode, NSPoint, NSRect, NSSize, 
                        NSString, NSUInteger}; 


struct Minifb {
    temp: isize,
}

impl Minifb {
    pub unsafe fn new(name: &str, width: isize, height: isize) -> Result<Minifb, CreationError> {
        let app = match Self::create_app() {
            Some(app) => app,
            None      => { return Err(OsError(format!("Couldn't create NSApplication"))); },
        };

        let masks = 0u64;

        //let masks = NSResizableWindowMask as NSUInteger | 
        //            NSClosableWindowMask as NSUInteger | 
        //           NSTitledWindowMaskas as NSUInteger;

        let frame = NSRect::new(NSPoint::new(0., 0.), NSSize::new(width as f64, height as f64));

        let window = IdRef::new(NSWindow::alloc(nil).initWithContentRect_styleMask_backing_defer_(
            frame,
            masks,
            NSBackingStoreBuffered,
            NO,
        ));

        if window.is_nil() {
            return Err(OsError(format!("Unable to create window"))); 
        }

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


