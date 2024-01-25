#import "OSXWindow.h"
#import "OSXWindowFrameView.h"

@implementation OSXWindow

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (void)dealloc {
    [[NSNotificationCenter defaultCenter]
        removeObserver:self];
    [super dealloc];
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (void)setContentSize:(NSSize)newSize {
    NSSize sizeDelta = newSize;
    NSSize childBoundsSize = [childContentView bounds].size;
    sizeDelta.width -= childBoundsSize.width;
    sizeDelta.height -= childBoundsSize.height;

    OSXWindowFrameView *frameView = [super contentView];
    NSSize newFrameSize = [frameView bounds].size;
    newFrameSize.width += sizeDelta.width;
    newFrameSize.height += sizeDelta.height;

    [super setContentSize:newFrameSize];
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

-(void)flagsChanged:(NSEvent *)event {
    const uint32_t flags = [event modifierFlags];

    // Ctrl checking - First check device dependent flags, otherwise fallback to none-device dependent

    if ((flags & NX_DEVICELCTLKEYMASK) || (flags & NX_DEVICERCTLKEYMASK)) {
        key_callback(rust_data, 0x3b, (flags & NX_DEVICELCTLKEYMASK) ? 1 : 0); // Left Ctrl
        key_callback(rust_data, 0x3e, (flags & NX_DEVICERCTLKEYMASK) ? 1 : 0); // Right Ctrl
    } else if (flags & NX_CONTROLMASK) {
        key_callback(rust_data, 0x3b, 1); // Left Ctrl
        key_callback(rust_data, 0x3e, 1); // Right Ctrl
    } else {
        key_callback(rust_data, 0x3b, 0); // Left Ctrl
        key_callback(rust_data, 0x3e, 0); // Right Ctrl
    }

    // Shift checking - First check device dependent flags, otherwise fallback to none-device dependent

    if ((flags & NX_DEVICELSHIFTKEYMASK) || (flags & NX_DEVICERSHIFTKEYMASK)) {
        key_callback(rust_data, 0x38, (flags & NX_DEVICELSHIFTKEYMASK) ? 1 : 0); // Left Shift
        key_callback(rust_data, 0x3c, (flags & NX_DEVICERSHIFTKEYMASK) ? 1 : 0); // Right Shift
    } else if (flags & NX_SHIFTMASK) {
        key_callback(rust_data, 0x38, 1); // Left Shift
        key_callback(rust_data, 0x3c, 1); // Right Shift
    } else {
        key_callback(rust_data, 0x38, 0); // Left Shift
        key_callback(rust_data, 0x3c, 0); // Right Shift
    }

    // Alt checking - First check device dependent flags, otherwise fallback to none-device dependent

    if ((flags & NX_DEVICELALTKEYMASK) || (flags & NX_DEVICERALTKEYMASK)) {
        key_callback(rust_data, 0x3a, (flags & NX_DEVICELALTKEYMASK) ? 1 : 0); // Left Alt
        key_callback(rust_data, 0x3d, (flags & NX_DEVICERALTKEYMASK) ? 1 : 0); // Right Alt
    } else if (flags & NX_ALTERNATEMASK) {
        key_callback(rust_data, 0x3a, 1); // Left Alt
        key_callback(rust_data, 0x3d, 1); // Right Alt
    } else {
        key_callback(rust_data, 0x3a, 0); // Left Alt
        key_callback(rust_data, 0x3d, 0); // Right Alt
    }

    // Cmd checking - First check device dependent flags, otherwise fallback to none-device dependent

    if ((flags & NX_DEVICELCMDKEYMASK) || (flags & NX_DEVICERCMDKEYMASK)) {
        key_callback(rust_data, 0x37, (flags & NX_DEVICELCMDKEYMASK) ? 1 : 0); // Left Cmd
        key_callback(rust_data, 0x36, (flags & NX_DEVICERCMDKEYMASK) ? 1 : 0); // Right Cmd
    } else if (flags & NX_COMMANDMASK) {
        key_callback(rust_data, 0x37, 1); // Left Cmd
        key_callback(rust_data, 0x36, 1); // Right Cmd
    } else {
        key_callback(rust_data, 0x37, 0); // Left Cmd
        key_callback(rust_data, 0x36, 0); // Right Cmd
    }

    [super flagsChanged:event];
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (void)keyDown:(NSEvent *)event {
    if (key_callback) {
        key_callback(rust_data, [event keyCode], 1);
    }

    if (char_callback) {
        NSString* characters = [event characters];
        NSUInteger i, length = [characters length];

        for (i = 0; i < length; i++) {
            char_callback(rust_data, [characters characterAtIndex:i]);
        }
    }
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (void)keyUp:(NSEvent *)event {
    if (key_callback) {
        key_callback(rust_data, [event keyCode], 0);
    }
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (void)mainWindowChanged:(NSNotification *)note {
    void* window = [note object];

    if (window == self) {
        self->is_active = true;
    } else {
        self->is_active = false;
    }
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (void)windowWillClose:(NSNotification *)notification {
    (void)notification;
    should_close = true;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (BOOL)windowShouldClose:(id)sender {
    (void)sender;
    should_close = true;
    return TRUE;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (void)setContentView:(NSView *)aView {
    if ([childContentView isEqualTo:aView]) {
        return;
    }

    NSRect bounds = [self frame];
    bounds.origin = NSZeroPoint;

    OSXWindowFrameView* frameView = [super contentView];
    if (!frameView) {
        [[NSNotificationCenter defaultCenter]
            addObserver:self
            selector:@selector(mainWindowChanged:)
            name:NSWindowDidBecomeMainNotification
            object:self];

        [[NSNotificationCenter defaultCenter]
            addObserver:self
            selector:@selector(mainWindowChanged:)
            name:NSWindowDidResignMainNotification
            object:self];

        frameView = [[[OSXWindowFrameView alloc] initWithFrame:bounds] autorelease];

        [super setContentView:frameView];
    }

    frame_view = frameView;

    if (childContentView) {
        [childContentView removeFromSuperview];
    }

    //printf("osxwindow: setContentFrameView %p\n", frameView);
    //printf("osxwindow: setting controller %p\n", view_controller);
    //frameView->m_view_controller = view_controller;

    //NSRect t = [self contentRectForFrameRect:bounds];

    childContentView = aView;
    [childContentView setFrame:[self contentRectForFrameRect:bounds]];
    [childContentView setAutoresizingMask:NSViewWidthSizable | NSViewHeightSizable];
    [frameView addSubview:childContentView];
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (NSView *)contentView {
    return childContentView;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (BOOL)canBecomeKeyWindow {
    return YES;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (BOOL)canBecomeMainWindow {
    return YES;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (NSRect)contentRectForFrameRect:(NSRect)windowFrame {
    windowFrame.origin = NSZeroPoint;
    return NSInsetRect(windowFrame, 0, 0);
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (void)updateSize {
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (void)onMenuPress:(id)sender {
    int menu_id = (int)((NSButton*)sender).tag;
    self->active_menu_id = menu_id;
}

@end
