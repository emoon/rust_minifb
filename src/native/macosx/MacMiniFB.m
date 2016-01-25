
#include "OSXWindow.h"
#include <Cocoa/Cocoa.h>
#include <unistd.h>

static bool s_init = false;


///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

void* mfb_open(const char* name, int width, int height, int scale)
{
	NSAutoreleasePool* pool = [[NSAutoreleasePool alloc] init];

	if (!s_init) {
		[NSApplication sharedApplication];
		[NSApp setActivationPolicy:NSApplicationActivationPolicyRegular];
		s_init = true;
	}
		
	unsigned int styles = NSClosableWindowMask | NSTitledWindowMask;
	NSRect rectangle = NSMakeRect(0, 0, width * scale, (height * scale));
		
	OSXWindow* window = [[OSXWindow alloc] initWithContentRect:rectangle styleMask:styles backing:NSBackingStoreBuffered defer:NO];

	if (!window)
		return 0;

	window->draw_buffer = malloc(width * height * 4);

	if (!window->draw_buffer)
		return 0;

	window->width = width;
	window->height = height;
	window->scale = scale;
	window->key_callback = 0;

	[window updateSize];

	[window setTitle:[NSString stringWithUTF8String:name]];
	[window setReleasedWhenClosed:NO];
	[window performSelectorOnMainThread:@selector(makeKeyAndOrderFront:) withObject:nil waitUntilDone:YES];
	[window setAcceptsMouseMovedEvents:YES];

	[window center];

	[NSApp activateIgnoringOtherApps:YES];

	[pool drain];

	return window;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

void mfb_close(void* win)
{
	NSWindow* window = (NSWindow*)win;

	NSAutoreleasePool* pool = [[NSAutoreleasePool alloc] init];

	if (window)
		[window close]; 

	[pool drain];
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

static int update_events()
{
	int state = 0;
	NSAutoreleasePool* pool = [[NSAutoreleasePool alloc] init];
	NSEvent* event = [NSApp nextEventMatchingMask:NSAnyEventMask untilDate:[NSDate distantPast] inMode:NSDefaultRunLoopMode dequeue:YES];
	[NSApp sendEvent:event];
	[pool release];

	return state;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

int mfb_update(void* window, void* buffer)
{
	OSXWindow* win = (OSXWindow*)window;
	memcpy(win->draw_buffer, buffer, win->width * win->height * 4);

	//g_updateBuffer = buffer;
	int state = update_events();
	[[win contentView] setNeedsDisplay:YES];
	return state;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

static float transformY(float y)
{
	float b = CGDisplayBounds(CGMainDisplayID()).size.height; 
	float t = b - y;
	return t;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

void mfb_set_position(void* window, int x, int y) 
{
	OSXWindow* win = (OSXWindow*)window;
	const NSRect contentRect = [[win contentView] frame];
    const NSRect dummyRect = NSMakeRect(x, transformY(y + contentRect.size.height), 0, 0);
    const NSRect frameRect = [win frameRectForContentRect:dummyRect];
    [win setFrameOrigin:frameRect.origin];
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

int mfb_should_close(void* window) 
{
	OSXWindow* win = (OSXWindow*)window;
	return win->should_close;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

uint32_t mfb_get_screen_size() 
{
	NSRect e = [[NSScreen mainScreen] frame];
	uint32_t w = (uint32_t)e.size.width;
	uint32_t h = (uint32_t)e.size.height;
	return (w << 16) | h;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

void mfb_set_key_callback(void* window, void* rust_data, void (*key_callback)(void* user_data, int key, int state))
{
	OSXWindow* win = (OSXWindow*)window;
	win->key_callback = key_callback;
	win->rust_data = rust_data;
}

