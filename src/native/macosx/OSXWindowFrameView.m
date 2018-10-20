#import "OSXWindowFrameView.h"
#import "OSXWindow.h"

@implementation OSXWindowFrameView

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

-(void)updateTrackingAreas
{
    if(trackingArea != nil) {
        [self removeTrackingArea:trackingArea];
        [trackingArea release];
    }

    int opts = (NSTrackingMouseEnteredAndExited | NSTrackingActiveAlways);
    trackingArea = [ [NSTrackingArea alloc] initWithRect:[self bounds]
                                            options:opts
                                            owner:self
                                            userInfo:nil];
    [self addTrackingArea:trackingArea];
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (void)drawRect:(NSRect)rect
{
    (void)rect;
    CGContextRef context = [[NSGraphicsContext currentContext] CGContext];

    printf("drawRect\n");

    CGColorSpaceRef space = CGColorSpaceCreateDeviceRGB();
    CGDataProviderRef provider = CGDataProviderCreateWithData(NULL, draw_buffer, width * height * 4, NULL);

    CGImageRef img = CGImageCreate(width, height, 8, 32, width * 4, space, kCGImageAlphaNoneSkipFirst | kCGBitmapByteOrder32Little,
                                   provider, NULL, false, kCGRenderingIntentDefault);

    CGColorSpaceRelease(space);
    CGDataProviderRelease(provider);

    CGContextDrawImage(context, CGRectMake(0, 0, width * scale, height * scale), img);

    CGImageRelease(img);
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

/*
- (BOOL)wantsUpdateLayer
{
    return TRUE;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

-(void)updateLayer
{
    printf("update layer\n");
    // Force the graphics context to clear to black so we don't get a flash of
    // white until the app is ready to draw. In practice on modern macOS, this
    // only gets called for window creation and other extraordinary events.
    self.layer.backgroundColor = NSColor.blackColor.CGColor;
    //NSGraphicsContext* context = [NSGraphicsContext currentContext];
    //[context scheduleUpdate];
    
    //(void)rect;
    CGContextRef context = [[NSGraphicsContext currentContext] CGContext];

    //printf("drawRect\n");

    CGColorSpaceRef space = CGColorSpaceCreateDeviceRGB();
    CGDataProviderRef provider = CGDataProviderCreateWithData(NULL, draw_buffer, width * height * 4, NULL);

    CGImageRef img = CGImageCreate(width, height, 8, 32, width * 4, space, kCGImageAlphaNoneSkipFirst | kCGBitmapByteOrder32Little,
                                   provider, NULL, false, kCGRenderingIntentDefault);

    CGColorSpaceRelease(space);
    CGDataProviderRelease(provider);

    CGContextDrawImage(context, CGRectMake(0, 0, width * scale, height * scale), img);

    CGImageRelease(img);

    //ScheduleContextUpdates((SDL_WindowData *) _sdlWindow->driverdata);
    //SDL_SendWindowEvent(_sdlWindow, SDL_WINDOWEVENT_EXPOSED, 0, 0);
    //[context scheduleUpdate];
}
*/

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (void)mouseDown:(NSEvent*)event
{
    (void)event;
    OSXWindow* window = (OSXWindow*)[self window];
    window->shared_data->mouse_state[0] = 1;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (void)mouseUp:(NSEvent*)event
{
    (void)event;
    OSXWindow* window = (OSXWindow*)[self window];
    window->shared_data->mouse_state[0] = 0;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (void)rightMouseDown:(NSEvent*)event
{
    (void)event;
    OSXWindow* window = (OSXWindow*)[self window];
    window->shared_data->mouse_state[2] = 1;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (void)rightMouseUp:(NSEvent*)event
{
    (void)event;
    OSXWindow* window = (OSXWindow*)[self window];
    window->shared_data->mouse_state[2] = 0;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (void)scrollWheel:(NSEvent *)event
{
    OSXWindow* window = (OSXWindow*)[self window];
    window->shared_data->scroll_x = [event deltaX];
    window->shared_data->scroll_y = [event deltaY];
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (BOOL)canBecomeKeyView
{
    return YES;
}

- (NSView *)nextValidKeyView
{
    return self;
}

- (NSView *)previousValidKeyView
{
    return self;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (BOOL)acceptsFirstResponder
{
    return YES;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (void)viewDidMoveToWindow
{
    [[NSNotificationCenter defaultCenter] addObserver:self
    selector:@selector(windowResized:) name:NSWindowDidResizeNotification
    object:[self window]];
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (void)dealloc
{
    [[NSNotificationCenter defaultCenter] removeObserver:self];
    [super dealloc];
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (void)windowResized:(NSNotification *)notification
{
    (void)notification;
    NSSize size = [self bounds].size;
    OSXWindow* window = (OSXWindow*)[self window];

    if (window->shared_data) {
        window->shared_data->width = (int)(size.width);
        window->shared_data->height = (int)(size.height);
    }
}

@end

