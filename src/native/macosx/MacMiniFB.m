#include "OSXWindow.h"
#include "OSXWindowFrameView.h"
#include <Cocoa/Cocoa.h>
#include <Carbon/Carbon.h>
#include <unistd.h>

static bool s_init = false;

// window_handler.rs
const uint32_t WINDOW_BORDERLESS = 1 << 1;
const uint32_t WINDOW_RESIZE = 1 << 2;
const uint32_t WINDOW_TITLE = 1 << 3;

static void create_standard_menu();

// Needs to match lib.rs enum
enum CursorStyle {
    CursorStyle_Arrow,
    CursorStyle_Ibeam,
    CursorStyle_Crosshair,
    CursorStyle_ClosedHand,
    CursorStyle_OpenHand,
    CursorStyle_ResizeLeftRight,
    CursorStyle_ResizeUpDown,
    CursorStyle_SizeAll,
    CursorStyle_Count,
};

static NSCursor* s_cursors[CursorStyle_Count];

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

void cursor_init() {
    s_cursors[CursorStyle_Arrow] = [[NSCursor arrowCursor] retain];
    s_cursors[CursorStyle_Ibeam] = [[NSCursor IBeamCursor] retain];
    s_cursors[CursorStyle_Crosshair] = [[NSCursor crosshairCursor] retain];
    s_cursors[CursorStyle_ClosedHand] = [[NSCursor closedHandCursor] retain];
    s_cursors[CursorStyle_OpenHand] = [[NSCursor openHandCursor] retain];
    s_cursors[CursorStyle_ResizeLeftRight] = [[NSCursor resizeLeftRightCursor] retain];
    s_cursors[CursorStyle_ResizeUpDown] = [[NSCursor resizeUpDownCursor] retain];
    s_cursors[CursorStyle_SizeAll] = [[NSCursor closedHandCursor] retain];
}

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
		cursor_init();
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
	window->active_menu_id = -1;
	window->prev_cursor = 0;

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

void mfb_set_title(void* window, const char* title)
{
	OSXWindow* win = (OSXWindow*)window;
	NSString* ns_title = [NSString stringWithUTF8String: title];
	[win setTitle: ns_title];
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

static int generic_update(OSXWindow* win)
{
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

void mfb_update_title(void* window, const char* title)
{
	OSXWindow* win = (OSXWindow*)window;
	NSString* ns_title = [NSString stringWithUTF8String: title];
	[win setTitle: ns_title];
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

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

void mfb_set_key_callback(void* window, void* rust_data,
						  void (*key_callback)(void* user_data, int key, int state),
						  void (*char_callback)(void* user_data, uint32_t key))
{
	OSXWindow* win = (OSXWindow*)window;
	win->key_callback = key_callback;
	win->char_callback = char_callback;
	win->rust_data = rust_data;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

void mfb_set_mouse_data(void* window, SharedData* shared_data)
{
	OSXWindow* win = (OSXWindow*)window;
	win->shared_data = shared_data;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

void mfb_set_cursor_style(void* window, int cursor)
{
	OSXWindow* win = (OSXWindow*)window;

	if (win->prev_cursor == cursor)
		return;

	if (cursor < 0 || cursor >= CursorStyle_Count) {
		printf("cursor out of range %d\n", cursor);
		return;
	}

	[s_cursors[cursor] set];

	win->prev_cursor = cursor;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

uint32_t mfb_is_active(void* window)
{
	OSXWindow* win = (OSXWindow*)window;
	return win->is_active;
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

int mfb_active_menu(void* window) {
	OSXWindow* win = (OSXWindow*)window;
	int active_menu_id = win->active_menu_id;
	win->active_menu_id = -1;
	return active_menu_id;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

static CFStringRef create_string_for_key(CGKeyCode keyCode)
{
    TISInputSourceRef currentKeyboard = TISCopyCurrentKeyboardInputSource();
    CFDataRef layoutData = TISGetInputSourceProperty(currentKeyboard, kTISPropertyUnicodeKeyLayoutData);

	if (!layoutData)
		return 0;

    const UCKeyboardLayout *keyboardLayout = (const UCKeyboardLayout *)CFDataGetBytePtr(layoutData);

    UInt32 keysDown = 0;
    UniChar chars[4];
    UniCharCount realLength;

    UCKeyTranslate(keyboardLayout,
                   keyCode,
                   kUCKeyActionDisplay,
                   0,
                   LMGetKbdType(),
                   kUCKeyTranslateNoDeadKeysBit,
                   &keysDown,
                   sizeof(chars) / sizeof(chars[0]),
                   &realLength,
                   chars);
    CFRelease(currentKeyboard);

    return CFStringCreateWithCharacters(kCFAllocatorDefault, chars, 1);
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

static NSString* convert_key_code_to_string(int key)
{
	if (key < 128)
	{
		NSString* charName = (NSString*)create_string_for_key(key);

		if (charName)
			return charName;

		return [NSString stringWithFormat:@"%c", (char)key];
	}

	return [NSString stringWithFormat:@"%C", (uint16_t)key];
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

const uint32_t MENU_KEY_COMMAND = 1;
const uint32_t MENU_KEY_WIN = 2;
const uint32_t MENU_KEY_SHIFT= 4;
const uint32_t MENU_KEY_CTRL = 8;
const uint32_t MENU_KEY_ALT = 16;

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

static NSString* get_string_for_key(uint32_t t) {
	unichar c = (unichar)t;
	NSString* key = [NSString stringWithCharacters:&c length:1];
	return key;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

uint64_t mfb_add_menu_item(
	void* in_menu,
	int32_t menu_id,
	const char* item_name,
	bool enabled,
	uint32_t key,
	uint32_t modfier)
{
	NSMenu* menu = (NSMenu*)in_menu;

	NSString* name = [NSString stringWithUTF8String: item_name];

	if (menu_id == -1)
	{
		[menu addItem:[NSMenuItem separatorItem]];
	}
	else
	{
		NSString* key_string = 0;
		int mask = 0;
		NSMenuItem* newItem = [[NSMenuItem alloc] initWithTitle:name action:@selector(onMenuPress:) keyEquivalent:@""];
		[newItem setTag:menu_id];

		// This code may look a bit weird but is here for a reason:
		//
		// In order to make it easier to bulid cross-platform apps Ctrl is often used as
		// default modifier on Windows/Nix* while it's Command on Mac. Now we when Ctrl
		// is set we default to Command on Mac for that reason but if Command AND Ctrl is
		// set we allow both Ctrl and Command to be used but then it's up to the developer
		// to deal with diffrent shortcuts depending on OS.
		//

		if ((modfier & MENU_KEY_CTRL)) {
			mask |= NSCommandKeyMask;
		}
		if ((modfier & MENU_KEY_CTRL) &&
		    (modfier & MENU_KEY_COMMAND)) {
			mask |= NSControlKeyMask;
		}
		if (modfier & MENU_KEY_SHIFT) {
			mask |= NSShiftKeyMask;
		}
		if (modfier & MENU_KEY_ALT) {
			mask |= NSAlternateKeyMask;
		}

		switch (key) {
			case 0x7a: { key_string = get_string_for_key(NSF1FunctionKey); break; } // F1
			case 0x78: { key_string = get_string_for_key(NSF2FunctionKey); break; } // F2
			case 0x63: { key_string = get_string_for_key(NSF3FunctionKey); break; } // F3
			case 0x76: { key_string = get_string_for_key(NSF4FunctionKey); break; } // F4
			case 0x60: { key_string = get_string_for_key(NSF5FunctionKey); break; } // F5
			case 0x61: { key_string = get_string_for_key(NSF6FunctionKey); break; } // F6
			case 0x62: { key_string = get_string_for_key(NSF7FunctionKey); break; } // F7
			case 0x64: { key_string = get_string_for_key(NSF8FunctionKey); break; } // F8
			case 0x65: { key_string = get_string_for_key(NSF9FunctionKey); break; } // F9
			case 0x6d: { key_string = get_string_for_key(NSF10FunctionKey); break; } // F10
			case 0x67: { key_string = get_string_for_key(NSF11FunctionKey); break; } // F11
			case 0x6f: { key_string = get_string_for_key(NSF12FunctionKey); break; } // F12
			case 0x7f: break;
			default: {
				key_string = convert_key_code_to_string(key);
			}
		}

		if (key_string) {
			[newItem setKeyEquivalentModifierMask: mask];
			[newItem setKeyEquivalent:key_string];
		}

		if (enabled) {
			[newItem setEnabled:YES];
		} else {
			[newItem setEnabled:NO];
		}

		[newItem setOnStateImage: newItem.offStateImage];
		[menu addItem:newItem];

		return (uint64_t)newItem;
	}

	return 0;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

void mfb_add_sub_menu(void* parent_menu, const char* menu_name, void* attach_menu) {
	NSMenu* parent = (NSMenu*)parent_menu;
	NSMenu* attach = (NSMenu*)attach_menu;
	NSString* name = [NSString stringWithUTF8String: menu_name];

	NSMenuItem* newItem = [[NSMenuItem alloc] initWithTitle:name action:NULL keyEquivalent:@""];
	[newItem setSubmenu:attach];

	[parent addItem:newItem];
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

void* mfb_create_menu(const char* name) {
	NSString* ns_name = [NSString stringWithUTF8String: name];
    NSMenu* menu = [[NSMenu alloc] initWithTitle:ns_name];
	return (void*)menu;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

void mfb_destroy_menu(void* menu_item, const char* name)
{
	NSMenuItem* item = (NSMenuItem*)menu_item;
 	NSMenu* main_menu = [NSApp mainMenu];
	[main_menu removeItem:item];
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

void mfb_remove_menu_item(void* parent, uint64_t menu_item) {
	NSMenu* menu = (NSMenu*)parent;
	NSMenuItem* item = (NSMenuItem*)(uintptr_t)menu_item;
	[menu removeItem:item];
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

uint64_t mfb_add_menu(void* window, void* m)
{
	OSXWindow* win = (OSXWindow*)window;
	NSMenu* menu = (NSMenu*)m;

 	NSMenu* main_menu = [NSApp mainMenu];

    NSMenuItem* windowMenuItem = [main_menu addItemWithTitle:@"" action:NULL keyEquivalent:@""];
    [NSApp setWindowsMenu:menu];
    [windowMenuItem setSubmenu:menu];

    return (uint64_t)menu;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

void mfb_remove_menu_at(void* window, int index)
{
	(void)window;
 	NSMenu* main_menu = [NSApp mainMenu];
	[main_menu removeItemAtIndex:index];
}


