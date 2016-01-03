#include <X11/Xresource.h>
#include <X11/Xlib.h>
#include <X11/Xutil.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

#define KEY_FUNCTION 0xFF
#define KEY_ESC 0x1B

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

static int s_window_count = 0;
static Display* s_display;
static int s_screen;
static int s_width;
static int s_height;
static GC s_gc;
static int s_depth;
static int s_setup_done = 0;
static Visual* s_visual;
static int s_screen_width;
static int s_screen_height;
static XContext s_context;

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

typedef struct WindowInfo {
	void (*key_callback)(void* user_data, int key, int state);
	void* rust_data;
	Window window;
	XImage* ximage;
	void* draw_buffer;
	int scale;
	int width;
	int height;
} WindowInfo;

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

static int setup_display() {
	int depth, i, formatCount, convDepth = -1;
	XPixmapFormatValues* formats;

	if (s_setup_done) {
		return 1;
	}

	s_display = XOpenDisplay(0);

	if (!s_display) {
		printf("Unable to open X11 display\n");
		return 0;
	}

	s_context = XUniqueContext();
	s_screen = DefaultScreen(s_display);
	s_visual = DefaultVisual(s_display, s_screen);
	formats = XListPixmapFormats(s_display, &formatCount);
	depth = DefaultDepth(s_display, s_screen);

	for (i = 0; i < formatCount; ++i) {
		if (depth == formats[i].depth) {
			convDepth = formats[i].bits_per_pixel;
			break;
		}
	}
  
	XFree(formats);

	// We only support 32-bit right now
	if (convDepth != 32) {
		printf("Unable to find 32-bit format for X11 display\n");
		XCloseDisplay(s_display);
		return 0;
	}

	s_depth = depth;

	s_gc = DefaultGC(s_display, s_screen);

	s_screen_width = DisplayWidth(s_display, s_screen);
	s_screen_height = DisplayHeight(s_display, s_screen);

	s_setup_done = 1;

	printf("setup done\n");

	return 1;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

void* mfb_open(const char* title, int width, int height, int scale)
{
	XSetWindowAttributes windowAttributes;
	XSizeHints sizeHints;
	XImage* image;
	Window window;
	WindowInfo* window_info;

	if (!setup_display()) {
		return 0;
	}

	Window defaultRootWindow = DefaultRootWindow(s_display);

	windowAttributes.border_pixel = BlackPixel(s_display, s_screen);
	windowAttributes.background_pixel = BlackPixel(s_display, s_screen);
	windowAttributes.backing_store = NotUseful;

	window = XCreateWindow(s_display, defaultRootWindow, (s_screen_width - width) / 2,
					(s_screen_height - height) / 2, width, height, 0, s_depth, InputOutput,
					s_visual, CWBackPixel | CWBorderPixel | CWBackingStore,
					&windowAttributes);
	if (!window) {
		printf("Unable to create X11 Window\n");
		return 0;
	}

	//XSelectInput(s_display, s_window, KeyPressMask | KeyReleaseMask);
	XStoreName(s_display, window, title);

	sizeHints.flags = PPosition | PMinSize | PMaxSize;
	sizeHints.x = 0;
	sizeHints.y = 0;
	sizeHints.min_width = width;
	sizeHints.max_width = width;
	sizeHints.min_height = height;
	sizeHints.max_height = height;

	XSelectInput(s_display, window, KeyPressMask | KeyReleaseMask);
  	XSetWMNormalHints(s_display, window, &sizeHints);
  	XClearWindow(s_display, window);
  	XMapRaised(s_display, window);
	XFlush(s_display);

	image = XCreateImage(s_display, CopyFromParent, s_depth, ZPixmap, 0, NULL, width, height, 32, width * 4);

	if (!image) {
		XDestroyWindow(s_display, window);
		printf("Unable to create XImage\n");
		return 0;
	}

	window_info = (WindowInfo*)malloc(sizeof(WindowInfo));
	window_info->key_callback = 0;
	window_info->rust_data = 0;
	window_info->window = window;
	window_info->ximage = image;
	window_info->scale = scale;
	window_info->width = width;
	window_info->height = height;
	window_info->draw_buffer = malloc(width * height * 4);

	XSaveContext(s_display, window, s_context, (XPointer) window_info);

	image->data = (char*)window_info->draw_buffer;

	s_window_count += 1;

	return (void*)window_info;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

static WindowInfo* find_handle(Window handle)
{
    WindowInfo* info;

    if (XFindContext(s_display, handle, s_context, (XPointer*) &info) != 0) {
        return 0;
    }

    return info;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

static void process_event(XEvent* event) {
	KeySym sym;

	WindowInfo* info = find_handle(event->xany.window);

	if (!info)
		return;

	if ((event->type == KeyPress) || (event->type == KeyRelease) && info->key_callback) {
		int sym = XLookupKeysym(&event->xkey, 0);

		if (event->type == KeyPress) {
			info->key_callback(info->rust_data, sym, 1);
		} else if (event->type == KeyRelease) {
			info->key_callback(info->rust_data, sym, 0);
		}
	}
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

static int process_events()
{
	int count;
	XEvent event;
	KeySym sym;

	count = XPending(s_display);

    while (count--)
    {
        XEvent event;
        XNextEvent(s_display, &event);
        process_event(&event);
    }

	return 0;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

void mfb_update(void* window_info, void* buffer)
{
	WindowInfo* info = (WindowInfo*)window_info;
	int width = info->width;
	int height = info->height;

	memcpy(info->draw_buffer, buffer, width * height * 4);

	XPutImage(s_display, info->window, s_gc, info->ximage, 0, 0, 0, 0, width, height);
	XFlush(s_display);

	process_events();
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

void mfb_close(void* window_info)
{
	WindowInfo* info = (WindowInfo*)window_info;

	info->ximage->data = NULL;

	XDestroyImage(info->ximage);
	XDestroyWindow(s_display, info->window);

	s_window_count--;

	// Only close display when there are no windows left

	if (s_window_count == 0) {
		XCloseDisplay(s_display);
	}
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

void mfb_set_key_callback(void* window, void* rust_data, void (*key_callback)(void* user_data, int key, int state))
{
	WindowInfo* win = (WindowInfo*)window;
	win->key_callback = key_callback;
	win->rust_data = rust_data;
}

