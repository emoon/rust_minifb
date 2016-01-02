
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
		
	unsigned int styles = NSResizableWindowMask | NSClosableWindowMask | NSTitledWindowMask;
		
	NSRect rectangle = NSMakeRect(0, 0, width, height);
	OSXWindow* window = [[OSXWindow alloc] initWithContentRect:rectangle styleMask:styles backing:NSBackingStoreBuffered defer:NO];

	if (!window)
		return 0;

	window->draw_buffer = malloc(width * height * 4);

	if (!window->draw_buffer)
		return 0;

	window->width = width;
	window->height = height;
	window->scale = scale;

	[window updateSize];

	[window setTitle:[NSString stringWithUTF8String:name]];
	[window setReleasedWhenClosed:NO];
	[window performSelectorOnMainThread:@selector(makeKeyAndOrderFront:) withObject:nil waitUntilDone:YES];

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
