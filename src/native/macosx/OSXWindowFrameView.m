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
	CGContextRef context = [[NSGraphicsContext currentContext] graphicsPort];

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

- (void)mouseDown:(NSEvent*)event
{
    OSXWindow* window = (OSXWindow*)[self window];
    window->shared_data->mouse_state[0] = 1;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (void)mouseUp:(NSEvent*)event
{
    OSXWindow* window = (OSXWindow*)[self window];
    window->shared_data->mouse_state[0] = 0;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (void)rightMouseDown:(NSEvent*)event
{
    OSXWindow* window = (OSXWindow*)[self window];
    window->shared_data->mouse_state[2] = 1;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (void)rightMouseUp:(NSEvent*)event
{
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

- (void)windowResized:(NSNotification *)notification;
{
	NSSize size = [[self window] frame].size;
    OSXWindow* window = (OSXWindow*)[self window];
    window->shared_data->width = (int)size.width;
    window->shared_data->height = (int)size.height;
}

@end

