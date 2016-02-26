
#include "OSXWindow.h"
#include "OSXWindowFrameView.h"
#include <Cocoa/Cocoa.h>
#include <unistd.h>

static bool s_init = false;

// window_handler.rs
const uint32_t WINDOW_BORDERLESS = 1 << 1;
const uint32_t WINDOW_RESIZE = 1 << 2;
const uint32_t WINDOW_TITLE = 1 << 3;

static void create_standard_menu();

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

#ifdef __clang__
#pragma clang diagnostic ignored "-Wobjc-method-access" // [window updateSize];
#endif

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

void* mfb_open(const char* name, int width, int height, uint32_t flags, int scale)
{
	bool prev_init = s_init;

	//NSAutoreleasePool* pool = [[NSAutoreleasePool alloc] init];

	if (!s_init) {
		[NSApplication sharedApplication];
		[NSApp setActivationPolicy:NSApplicationActivationPolicyRegular];
		create_standard_menu();
		s_init = true;
	}

	uint32_t styles = NSClosableWindowMask | NSMiniaturizableWindowMask;

	if (flags & WINDOW_BORDERLESS)
		styles |= NSBorderlessWindowMask;

	if (flags & WINDOW_RESIZE)
		styles |= NSResizableWindowMask;

	if (flags & WINDOW_TITLE)
		styles |= NSTitledWindowMask;

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
	window->shared_data = 0;

	window->menu_data = malloc(sizeof(MenuData));
	memset(window->menu_data, 0, sizeof(MenuData));

	[window updateSize];

	[window setTitle:[NSString stringWithUTF8String:name]];
	[window setReleasedWhenClosed:NO];
	[window performSelectorOnMainThread:@selector(makeKeyAndOrderFront:) withObject:nil waitUntilDone:YES];
	[window setAcceptsMouseMovedEvents:YES];

	[window center];

	[NSApp activateIgnoringOtherApps:YES];

	if (!prev_init)
		[NSApp finishLaunching];

	//[pool drain];

	return window;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

static NSString* findAppName(void)
{
    size_t i;
    NSDictionary* infoDictionary = [[NSBundle mainBundle] infoDictionary];

    // Keys to search for as potential application names
    NSString* GLFWNameKeys[] =
    {
        @"CFBundleDisplayName",
        @"CFBundleName",
        @"CFBundleExecutable",
    };

    for (i = 0;  i < sizeof(GLFWNameKeys) / sizeof(GLFWNameKeys[0]);  i++)
    {
        id name = [infoDictionary objectForKey:GLFWNameKeys[i]];
        if (name &&
            [name isKindOfClass:[NSString class]] &&
            ![name isEqualToString:@""])
        {
            return name;
        }
    }

    extern char** _NSGetProgname();
    char* progname = *_NSGetProgname();

    if (progname)
        return [NSString stringWithUTF8String:progname];

    // Really shouldn't get here
    return @"Unknown";
}


///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

static void create_standard_menu(void)
{
    NSString* appName = findAppName();

    NSMenu* bar = [[NSMenu alloc] init];
    [NSApp setMainMenu:bar];

    NSMenuItem* appMenuItem =
        [bar addItemWithTitle:@"" action:NULL keyEquivalent:@""];
    NSMenu* appMenu = [[NSMenu alloc] init];
    [appMenuItem setSubmenu:appMenu];

    [appMenu addItemWithTitle:[NSString stringWithFormat:@"About %@", appName]
                      action:@selector(orderFrontStandardAboutPanel:)
               keyEquivalent:@""];
    [appMenu addItem:[NSMenuItem separatorItem]];
    NSMenu* servicesMenu = [[NSMenu alloc] init];
    [NSApp setServicesMenu:servicesMenu];
    [[appMenu addItemWithTitle:@"Services"
                       action:NULL
                keyEquivalent:@""] setSubmenu:servicesMenu];
    [servicesMenu release];
    [appMenu addItem:[NSMenuItem separatorItem]];
    [appMenu addItemWithTitle:[NSString stringWithFormat:@"Hide %@", appName]
                       action:@selector(hide:)
                keyEquivalent:@"h"];
    [[appMenu addItemWithTitle:@"Hide Others"
                       action:@selector(hideOtherApplications:)
                keyEquivalent:@"h"]
        setKeyEquivalentModifierMask:NSAlternateKeyMask | NSCommandKeyMask];
    [appMenu addItemWithTitle:@"Show All"
                       action:@selector(unhideAllApplications:)
                keyEquivalent:@""];
    [appMenu addItem:[NSMenuItem separatorItem]];
    [appMenu addItemWithTitle:[NSString stringWithFormat:@"Quit %@", appName]
                       action:@selector(terminate:)
                keyEquivalent:@"q"];

	/*
    NSMenuItem* windowMenuItem =
        [bar addItemWithTitle:@"" action:NULL keyEquivalent:@""];
    [bar release];
    NSMenu* windowMenu = [[NSMenu alloc] initWithTitle:@"Window"];
    [NSApp setWindowsMenu:windowMenu];
    [windowMenuItem setSubmenu:windowMenu];

    [windowMenu addItemWithTitle:@"Minimize"
                          action:@selector(performMiniaturize:)
                   keyEquivalent:@"m"];
    [windowMenu addItemWithTitle:@"Zoom"
                          action:@selector(performZoom:)
                   keyEquivalent:@""];
    [windowMenu addItem:[NSMenuItem separatorItem]];
    [windowMenu addItemWithTitle:@"Bring All to Front"
                          action:@selector(arrangeInFront:)
                   keyEquivalent:@""];
    // TODO: Make this appear at the bottom of the menu (for consistency)
    [windowMenu addItem:[NSMenuItem separatorItem]];
    [[windowMenu addItemWithTitle:@"Enter Full Screen"
                           action:@selector(toggleFullScreen:)
                    keyEquivalent:@"f"]
     setKeyEquivalentModifierMask:NSControlKeyMask | NSCommandKeyMask];
    */

    // Prior to Snow Leopard, we need to use this oddly-named semi-private API
    // to get the application menu working properly.
    //SEL setAppleMenuSelector = NSSelectorFromString(@"setAppleMenu:");
    //[NSApp performSelector:setAppleMenuSelector withObject:appMenu];
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
	NSEvent* event;
	NSAutoreleasePool* pool = [[NSAutoreleasePool alloc] init];

    do
    {
        event = [NSApp nextEventMatchingMask:NSAnyEventMask untilDate:[NSDate distantPast] inMode:NSDefaultRunLoopMode dequeue:YES];

        if (event) {
            [NSApp sendEvent:event];
        }
    }
    while (event);

	[pool release];

	/*
	int state = 0;
	NSEvent* event = [NSApp nextEventMatchingMask:NSAnyEventMask untilDate:[NSDate distantPast] inMode:NSDefaultRunLoopMode dequeue:YES];
	[NSApp sendEvent:event];
	*/

	return 0;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

static int generic_update(OSXWindow* win) {
	int state = update_events();

    if (win->shared_data) {
		NSPoint p = [win mouseLocationOutsideOfEventStream];
		NSRect originalFrame = [win frame];
		NSRect contentRect = [NSWindow contentRectForFrameRect: originalFrame styleMask: NSTitledWindowMask];
		win->shared_data->mouse_x = p.x;
		win->shared_data->mouse_y = contentRect.size.height - p.y;
	}

	return state;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

int mfb_update(void* window, void* buffer)
{
	OSXWindow* win = (OSXWindow*)window;
	return generic_update(win);
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

int mfb_update_with_buffer(void* window, void* buffer)
{
	OSXWindow* win = (OSXWindow*)window;
	memcpy(win->draw_buffer, buffer, win->width * win->height * 4);

	int state = generic_update(win);

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

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

void mfb_set_mouse_data(void* window, SharedData* shared_data)
{
	OSXWindow* win = (OSXWindow*)window;
	win->shared_data = shared_data;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

void mfb_remove_menu(void* window, const char* name)
{
	OSXWindow* win = (OSXWindow*)window;

	NSString* ns_name = [NSString stringWithUTF8String: name];
 	NSMenu* main_menu = [NSApp mainMenu];

 	int len = win->menu_data->menu_count;

 	for (int i = 0; i < len; ++i)
	{
		Menu* menu = &win->menu_data->menus[i];

		if (strcmp(menu->name, name))
			continue;

		Menu* menu_end = &win->menu_data->menus[len - 1];

		[main_menu removeItem:menu->menu_item];

		// swap remove
		menu->name = menu_end->name;
		menu->menu = menu_end->menu;
		menu->menu_item = menu_end->menu_item;

		win->menu_data->menu_count--;
	}
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

void mfb_add_menu(void* window, const char* name, void* m)
{
	OSXWindow* win = (OSXWindow*)window;

	const char* n = strdup(name);

	NSString* ns_name = [NSString stringWithUTF8String: n];

 	NSMenu* main_menu = [NSApp mainMenu];

    NSMenuItem* windowMenuItem = [main_menu addItemWithTitle:@"" action:NULL keyEquivalent:@""];
    NSMenu* windowMenu = [[NSMenu alloc] initWithTitle:ns_name];
    [NSApp setWindowsMenu:windowMenu];
    [windowMenuItem setSubmenu:windowMenu];

	MenuDesc* menu_desc = (MenuDesc*)m;

	[windowMenu setAutoenablesItems:NO];

	build_submenu(windowMenu, menu_desc);

	Menu* menu = &win->menu_data->menus[win->menu_data->menu_count++];

	menu->name = n;
	menu->menu = windowMenu;
	menu->menu_item = windowMenuItem;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

void mfb_update_menu(void* window, const char* name, void* m)
{
	OSXWindow* win = (OSXWindow*)window;

	NSString* ns_name = [NSString stringWithUTF8String: name];
 	NSMenu* main_menu = [NSApp mainMenu];

 	int len = win->menu_data->menu_count;

 	for (int i = 0; i < len; ++i)
	{
		Menu* menu = &win->menu_data->menus[i];

		if (!strcmp(menu->name, name)) {
			[menu->menu removeAllItems];
			build_submenu(menu->menu, (MenuDesc*)m);
			return;
		}
	}
}

