#include <X11/Xresource.h>
#include <X11/Xlib.h>
#include <X11/Xutil.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

#define KEY_FUNCTION 0xFF
#define KEY_ESC 0x1B
#define Button6 6
#define Button7 7

void mfb_close(void* window_info);

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

static int s_window_count = 0;
static Display* s_display;
static int s_screen;
static GC s_gc;
static int s_depth;
static int s_setup_done = 0;
static Visual* s_visual;
static int s_screen_width;
static int s_screen_height;
static XContext s_context;
static Atom s_wm_delete_window;

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

typedef struct SharedData {
	unsigned int width;
	unsigned int height;
    float mouse_x;
    float mouse_y;
    float scroll_x;
    float scroll_y;
    unsigned char state[3];
} SharedData;

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

typedef struct WindowInfo {
	void (*key_callback)(void* user_data, int key, int state);
	void* rust_data;
	SharedData* shared_data;
	Window window;
	XImage* ximage;
	void* draw_buffer;
	int scale;
	int width;
	int height;
	int update;
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

	const char* wmDeleteWindowName = "WM_DELETE_WINDOW";
    XInternAtoms(s_display, (char**)&wmDeleteWindowName, 1, False, &s_wm_delete_window);

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

	width *= scale;
	height *= scale;

	Window defaultRootWindow = DefaultRootWindow(s_display);

	windowAttributes.border_pixel = 0;
	windowAttributes.event_mask = StructureNotifyMask | KeyPressMask | KeyReleaseMask |
					PointerMotionMask | ButtonPressMask | ButtonReleaseMask |
					ExposureMask | FocusChangeMask | VisibilityChangeMask |
					EnterWindowMask | LeaveWindowMask | PropertyChangeMask;

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

	XSelectInput(s_display, window, ExposureMask | ButtonPressMask |
         KeyPressMask | KeyReleaseMask | ButtonReleaseMask | ButtonMotionMask
         | PointerMotionHintMask);

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
	window_info->update = 1;

	XSetWMProtocols(s_display, window, &s_wm_delete_window, 1);

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

static int process_event(XEvent* event) {
	KeySym sym;

	WindowInfo* info = find_handle(event->xany.window);

	if (!info)
		return 1;

	if (event->type == ClientMessage) {
		if ((Atom)event->xclient.data.l[0] == s_wm_delete_window) {
			info->update = 0;
			mfb_close(info);

			return 0;
		}
	}

	switch (event->type) {
		case KeyPress:
		{
  			sym = XLookupKeysym(&event->xkey, 0);

			if (info->key_callback)
				info->key_callback(info->rust_data, sym, 1);

			break;
		}

		case KeyRelease:
		{
  			sym = XLookupKeysym(&event->xkey, 0);

			if (info->key_callback)
				info->key_callback(info->rust_data, sym, 0);
			break;
		}

		case ButtonPress:
        {
        	printf("button %p\n", info->shared_data);

        	if (!info->shared_data)
        		break;

            if (event->xbutton.button == Button1)
            	info->shared_data->state[0] = 1;
            else if (event->xbutton.button == Button2)
            	info->shared_data->state[1] = 1;
            else if (event->xbutton.button == Button3)
            	info->shared_data->state[2] = 1;
            else if (event->xbutton.button == Button4)
            	info->shared_data->scroll_y = 10.0f;
            else if (event->xbutton.button == Button5)
            	info->shared_data->scroll_y = -10.0f;
            else if (event->xbutton.button == Button6)
            	info->shared_data->scroll_x = 10.0f;
            else if (event->xbutton.button == Button7)
            	info->shared_data->scroll_y = -10.0f;

            break;
		}

		case ButtonRelease:
        {
        	if (!info->shared_data)
        		break;

            if (event->xbutton.button == Button1)
            	info->shared_data->state[0] = 0;
            else if (event->xbutton.button == Button2)
            	info->shared_data->state[1] = 0;
            else if (event->xbutton.button == Button3)
            	info->shared_data->state[2] = 0;

            break;
		}
	}

	return 1;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

static void get_mouse_pos(WindowInfo* info) {
	Window root, child;
	int rootX, rootY, childX, childY;
	unsigned int mask;

	XQueryPointer(s_display, info->window,
					&root, &child,
					&rootX, &rootY, &childX, &childY,
					&mask);

	if (info->shared_data) {
		info->shared_data->mouse_x = (float)childX;
		info->shared_data->mouse_y = (float)childY;
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
		
		// Don't process any more messages if event is 0
        if (process_event(&event) == 0)
        	return 0;
    }

	return 0;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

static void scale_2x(unsigned int* dest, unsigned int* source, int width, int height, int scale) {
	int x, y;
	for (y = 0; y < height; y += scale) {
		for (x = 0; x < width; x += scale) {
			const unsigned int t = *source++;
			dest[0] = t;
			dest[1] = t;
			dest[width + 0] = t;
			dest[width + 1] = t;
			dest += scale;
		}

		dest += width * (scale - 1);
	}
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

static void scale_4x(unsigned int* dest, unsigned int* source, int width, int height, int scale) {
	int x, y;
	for (y = 0; y < height; y += scale) {
		for (x = 0; x < width; x += scale) {
			const unsigned int t = *source++;
			dest[(width * 0) + 0] = t;
			dest[(width * 0) + 1] = t;
			dest[(width * 0) + 2] = t;
			dest[(width * 0) + 3] = t;
			dest[(width * 1) + 0] = t;
			dest[(width * 1) + 1] = t;
			dest[(width * 1) + 2] = t;
			dest[(width * 1) + 3] = t;
			dest[(width * 2) + 0] = t;
			dest[(width * 2) + 1] = t;
			dest[(width * 2) + 2] = t;
			dest[(width * 2) + 3] = t;
			dest[(width * 3) + 0] = t;
			dest[(width * 3) + 1] = t;
			dest[(width * 3) + 2] = t;
			dest[(width * 3) + 3] = t;
			dest += scale;
		}

		dest += width * (scale - 1);
	}
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

void mfb_update(void* window_info, void* buffer)
{
	WindowInfo* info = (WindowInfo*)window_info;
	int width = info->width;
	int height = info->height;
	int scale = info->scale;

	if (info->update) {
		switch (scale) {
			case 1: {
				memcpy(info->draw_buffer, buffer, width * height * 4);
				break;
			}
			case 2: {
				scale_2x(info->draw_buffer, buffer, width, height, scale); 
				break;
			}

			case 4: {
				scale_4x(info->draw_buffer, buffer, width, height, scale); 
				break;
			}
		}

		XPutImage(s_display, info->window, s_gc, info->ximage, 0, 0, 0, 0, width, height);
		XFlush(s_display);
	}

	// clear before processing new events

	if (info->shared_data) {
		info->shared_data->scroll_x = 0.0f;
		info->shared_data->scroll_y = 0.0f;
	}

	get_mouse_pos(info);
	process_events();
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

void mfb_set_position(void* window, int x, int y) 
{
	WindowInfo* info = (WindowInfo*)window;
    XMoveWindow(s_display, info->window, x, y);
	XFlush(s_display);
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

void mfb_close(void* window_info)
{
	WindowInfo* info = (WindowInfo*)window_info;

	if (!info->draw_buffer)
		return;

	XSaveContext(s_display, info->window, s_context, (XPointer)0);

	free(info->draw_buffer);

	info->ximage->data = NULL;
	info->draw_buffer = 0;

	XDestroyImage(info->ximage);
	XDestroyWindow(s_display, info->window);
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

void mfb_set_key_callback(void* window, void* rust_data, void (*key_callback)(void* user_data, int key, int state))
{
	WindowInfo* win = (WindowInfo*)window;
	win->key_callback = key_callback;
	win->rust_data = rust_data;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

void mfb_set_shared_data(void* window, SharedData* data)
{
	WindowInfo* win = (WindowInfo*)window;
	win->shared_data = data;
	win->shared_data->width = win->width;
	win->shared_data->height = win->height;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

int mfb_should_close(void* window) {
	WindowInfo* win = (WindowInfo*)window;
	return !!win->update;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

unsigned int mfb_get_screen_size() {
	setup_display();
	return (s_screen_width << 16) | s_screen_height;
}

